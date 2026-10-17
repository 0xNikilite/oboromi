//! Guest GPU memory: each address space maps GPU VA pages onto guest-physical pages of guest RAM, and the
//! `MemoryTracker` over those physical pages decides what host mirrors see, so mirrors key on PA and generation.
use super::compute::HostBuffer;
use super::{VkError, VkState};
use ash::vk;
use std::alloc::{Layout, alloc_zeroed, dealloc, handle_alloc_error};
use std::collections::HashMap;
use std::ops::Range;
use std::ptr::NonNull;
use std::sync::Arc;

// The big page size nvhost-as-gpu maps with (Ryujinx NvHostAsGpuDeviceFile BigPageSize).
pub const PAGE: u64 = 0x1_0000;

pub type PaRange = Range<u64>;

pub struct Ram {
    ptr: NonNull<u8>,
    len: usize,
    alignment: usize,
}

// The allocation is owned by Ram and only reached through its slices.
unsafe impl Send for Ram {}

impl Ram {
    pub fn new(bytes: usize, alignment: usize) -> Self {
        let len = bytes.max(1).next_multiple_of(alignment);
        let layout = Layout::from_size_align(len, alignment).unwrap();
        let ptr = NonNull::new(unsafe { alloc_zeroed(layout) }).unwrap_or_else(|| handle_alloc_error(layout));
        Self { ptr, len, alignment }
    }

    pub fn resize(&mut self, bytes: usize) {
        let next = Self::new(bytes, self.alignment);
        unsafe { next.ptr.as_ptr().copy_from_nonoverlapping(self.ptr.as_ptr(), self.len.min(next.len)) };
        *self = next;
    }
}

impl std::ops::Deref for Ram {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl std::ops::DerefMut for Ram {
    fn deref_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

impl Drop for Ram {
    fn drop(&mut self) {
        unsafe { dealloc(self.ptr.as_ptr(), Layout::from_size_align_unchecked(self.len, self.alignment)) };
    }
}

#[derive(Debug)]
pub enum MemoryError {
    Unmapped(u64),
    Vk(VkError),
}

impl From<VkError> for MemoryError {
    fn from(e: VkError) -> Self {
        Self::Vk(e)
    }
}

fn pages(start: u64, len: u64) -> Range<u64> {
    start / PAGE..(start + len).div_ceil(PAGE)
}

/// `ranges` cut at page boundaries, in order.
fn pieces(ranges: &[PaRange]) -> impl Iterator<Item = PaRange> + '_ {
    ranges.iter().flat_map(|range| pages(range.start, range.end - range.start).map(|page| (page * PAGE).max(range.start)..((page + 1) * PAGE).min(range.end)))
}

/// The bytes `bytes` of `ranges` laid end to end, as pieces of guest-physical memory and where each starts in `bytes`.
pub fn within(ranges: &[PaRange], bytes: Range<u64>) -> impl Iterator<Item = (u64, PaRange)> + '_ {
    let starts = ranges.iter().scan(0, |at, range| Some(std::mem::replace(at, *at + range.end - range.start)));
    starts.zip(ranges).filter_map(move |(start, range)| {
        let (lo, hi) = (bytes.start.max(start), bytes.end.min(start + range.end - range.start));
        (lo < hi).then(|| (lo, range.start + lo - start..range.start + hi - start))
    })
}

#[derive(Default)]
pub struct PageMap {
    pa_by_va_page: HashMap<u64, u64>,
}

impl PageMap {
    pub fn map(&mut self, va: u64, pa: u64, len: u64) {
        assert!((va | pa | len).is_multiple_of(PAGE), "unaligned map {va:#x} -> {pa:#x} + {len:#x}");
        for (page, pa) in pages(va, len).zip((pa..).step_by(PAGE as usize)) {
            self.pa_by_va_page.insert(page, pa);
        }
    }

    pub fn unmap(&mut self, va: u64, len: u64) {
        for page in pages(va, len) {
            self.pa_by_va_page.remove(&page);
        }
    }

    /// From the lowest mapped page to the end of the highest.
    pub fn span(&self) -> Option<Range<u64>> {
        let pages = self.pa_by_va_page.keys();
        Some(pages.clone().min()? * PAGE..(pages.max()? + 1) * PAGE)
    }

    pub fn resolve(&self, va: u64, len: u64) -> Result<impl Iterator<Item = PaRange> + '_, MemoryError> {
        let end = va + len;
        if let Some(page) = pages(va, len).find(|page| !self.pa_by_va_page.contains_key(page)) {
            return Err(MemoryError::Unmapped((page * PAGE).max(va)));
        }
        let mut pieces = pages(va, len)
            .map(move |page| {
                let start = (page * PAGE).max(va);
                let pa = self.pa_by_va_page[&page] + start % PAGE;
                pa..pa + end.min((page + 1) * PAGE) - start
            })
            .peekable();
        Ok(std::iter::from_fn(move || {
            let mut range = pieces.next()?;
            while let Some(next) = pieces.next_if(|next| next.start == range.end) {
                range.end = next.end;
            }
            Some(range)
        }))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Writer {
    Cpu,
    Gpu,
}

/// Guest-physical pages with the generation and writer of their last write; generations come from one counter, so
/// the generation of a range rises whenever any of its pages is written.
#[derive(Default)]
pub struct MemoryTracker {
    last_write: HashMap<u64, (u64, Writer)>,
    generation: u64,
    /// Bytes copied from guest RAM into host mirrors.
    pub uploaded: u64,
}

/// A host-visible copy of guest-physical ranges, gathered in order into one buffer, with the generation each page piece
/// had when the buffer last matched it.
pub struct Mapping<'a> {
    pub buffer: HostBuffer<'a>,
    pub ranges: Vec<PaRange>,
    seen: Vec<(PaRange, u64)>,
}

impl<'a> Mapping<'a> {
    pub fn new(buffer: HostBuffer<'a>, ranges: Vec<PaRange>) -> Self {
        let seen = pieces(&ranges).map(|piece| (piece, u64::MAX)).collect();
        Self { buffer, ranges, seen }
    }

    /// Lays `ranges` over the buffer, keeping every page piece that stays at its offset.
    pub fn retarget(&mut self, ranges: Vec<PaRange>) {
        let old = std::mem::replace(&mut self.seen, pieces(&ranges).map(|piece| (piece, u64::MAX)).collect());
        let (mut old_at, mut at) = (0, 0);
        for ((piece, seen), (old, old_seen)) in self.seen.iter_mut().zip(old) {
            if old_at == at && old == *piece {
                *seen = old_seen;
            }
            (old_at, at) = (old_at + old.end - old.start, at + piece.end - piece.start);
        }
        self.ranges = ranges;
    }
}

impl MemoryTracker {
    pub fn mark_cpu_written(&mut self, pa: u64, len: u64) {
        self.mark(pa, len, Writer::Cpu);
    }

    pub fn mark_gpu_written(&mut self, pa: u64, len: u64) {
        self.mark(pa, len, Writer::Gpu);
    }

    fn mark(&mut self, pa: u64, len: u64, writer: Writer) {
        self.generation += 1;
        for page in pages(pa, len) {
            self.last_write.insert(page, (self.generation, writer));
        }
    }

    pub fn generation(&self, pa: u64, len: u64) -> u64 {
        pages(pa, len).filter_map(|page| self.last_write.get(&page)).map(|&(generation, _)| generation).max().unwrap_or(0)
    }

    pub fn writer(&self, pa: u64) -> Option<Writer> {
        self.last_write.get(&(pa / PAGE)).map(|&(_, writer)| writer)
    }

    pub fn upload<'a>(&mut self, state: &'a VkState, ram: &[u8], ranges: impl IntoIterator<Item = PaRange>) -> Result<Mapping<'a>, VkError> {
        let ranges: Vec<_> = ranges.into_iter().collect();
        let len = ranges.iter().map(|range| range.end - range.start).sum::<u64>();
        let mut mapping = Mapping::new(HostBuffer::new(state, len.max(1))?, ranges);
        self.refresh(&mut mapping, ram)?;
        Ok(mapping)
    }

    /// Copies into `mapping` the page pieces written since it last matched them, one copy per contiguous run.
    pub fn refresh(&mut self, mapping: &mut Mapping, ram: &[u8]) -> Result<(), VkError> {
        if mapping.buffer.imported.is_some() {
            mapping.seen.iter_mut().for_each(|(piece, seen)| *seen = self.generation(piece.start, piece.end - piece.start));
            return Ok(());
        }
        let (mut offset, mut runs) = (0, Vec::<(u64, PaRange)>::new());
        for (piece, seen) in &mut mapping.seen {
            let generation = self.generation(piece.start, piece.end - piece.start);
            if *seen != generation {
                *seen = generation;
                match runs.last_mut() {
                    Some((at, run)) if run.end == piece.start && *at + run.end - run.start == offset => run.end = piece.end,
                    _ => runs.push((offset, piece.clone())),
                }
            }
            offset += piece.end - piece.start;
        }
        for (offset, run) in runs {
            self.uploaded += run.end - run.start;
            mapping.buffer.write_at(offset, &ram[run.start as usize..run.end as usize])?;
        }
        Ok(())
    }

    /// Stores the page pieces the device changed in `mapping`, which matched guest memory before it ran.
    pub fn download(&mut self, mapping: &mut Mapping, ram: &mut [u8]) -> Result<(), VkError> {
        if mapping.buffer.imported.is_some() {
            mapping.ranges.iter().for_each(|range| self.mark_gpu_written(range.start, range.end - range.start));
        } else {
            let len = mapping.ranges.iter().map(|range| range.end - range.start).sum();
            self.store(ram, &mapping.ranges, mapping.buffer.view(0..len)?, Writer::Gpu);
        }
        for (piece, seen) in &mut mapping.seen {
            *seen = self.generation(piece.start, piece.end - piece.start);
        }
        Ok(())
    }

    /// Scatters `bytes` over `ranges` in order, marking only the page pieces whose bytes change.
    pub fn store(&mut self, ram: &mut [u8], ranges: &[PaRange], bytes: &[u8], writer: Writer) {
        let mut rest = bytes;
        for piece in pieces(ranges) {
            let (head, tail) = rest.split_at((piece.end - piece.start) as usize);
            let ram = &mut ram[piece.start as usize..piece.end as usize];
            if ram != head {
                ram.copy_from_slice(head);
                self.mark(piece.start, piece.end - piece.start, writer);
            }
            rest = tail;
        }
    }

    /// The global-memory SSBO window for `va..va + len` and the VA its first byte holds, aligned down for 128-bit
    /// accesses; the whole window must be mapped.
    pub fn window<'a>(&mut self, state: &'a VkState, ram: &[u8], space: &PageMap, va: u64, len: u64) -> Result<(Mapping<'a>, u64), MemoryError> {
        let base = va & !0xf;
        Ok((self.upload(state, ram, space.resolve(base, va + len - base)?)?, base))
    }
}

/// Guest RAM imported as device memory.
pub struct ImportedRam<'a> {
    device: &'a ash::Device,
    pub memory: vk::DeviceMemory,
    pub memory_type: u32,
    pub size: u64,
}

impl<'a> ImportedRam<'a> {
    fn new(state: &'a VkState, ram: &mut [u8]) -> Result<Option<Self>, VkError> {
        let Some((host, alignment)) = &state.external_memory_host else { return Ok(None) };
        let (ptr, len) = (ram.as_mut_ptr(), ram.len() as u64);
        // VK_EXT_external_memory_host, Host External Memory (VkImportMemoryHostPointerInfoEXT): the pointer and the
        // allocation size must both be multiples of minImportedHostPointerAlignment, and the memory type one
        // vkGetMemoryHostPointerPropertiesEXT reports; only a coherent type needs no flush through a mapping.
        if !(ptr as u64 | len).is_multiple_of(*alignment) {
            return Ok(None);
        }
        let handle_type = vk::ExternalMemoryHandleTypeFlags::HOST_ALLOCATION_EXT;
        let mut pointer = vk::MemoryHostPointerPropertiesEXT::default();
        if let Err(e) = unsafe { (host.fp().get_memory_host_pointer_properties_ext)(state.device.handle(), handle_type, ptr.cast(), &mut pointer) }.result() {
            return if e == vk::Result::ERROR_DEVICE_LOST { Err(e.into()) } else { Ok(None) };
        }
        let types = unsafe { state.instance.get_physical_device_memory_properties(state.physical_device) };
        let coherent = (0..types.memory_type_count).find(|&i| {
            pointer.memory_type_bits >> i & 1 == 1 && types.memory_types[i as usize].property_flags.contains(vk::MemoryPropertyFlags::HOST_COHERENT)
        });
        let Some(memory_type) = coherent else { return Ok(None) };
        let mut import = vk::ImportMemoryHostPointerInfoEXT::default().handle_type(handle_type).host_pointer(ptr.cast());
        let alloc = vk::MemoryAllocateInfo::default().allocation_size(len).memory_type_index(memory_type).push_next(&mut import);
        match unsafe { state.device.allocate_memory(&alloc, None) } {
            Ok(memory) => Ok(Some(Self { device: &state.device, memory, memory_type, size: len })),
            Err(vk::Result::ERROR_DEVICE_LOST) => Err(vk::Result::ERROR_DEVICE_LOST.into()),
            Err(_) => Ok(None),
        }
    }
}

impl Drop for ImportedRam<'_> {
    fn drop(&mut self) {
        unsafe { self.device.free_memory(self.memory, None) };
    }
}

/// Imports guest RAM on first use, so buffers over contiguous physical ranges alias it instead of mirroring it.
pub struct ZeroCopy<'a> {
    state: &'a VkState,
    imported: Option<Arc<ImportedRam<'a>>>,
    failed: bool,
}

impl<'a> ZeroCopy<'a> {
    pub fn new(state: &'a VkState) -> Option<Self> {
        state.external_memory_host.is_some().then(|| Self { state, imported: None, failed: false })
    }

    pub fn imported(&self) -> bool {
        self.imported.is_some()
    }

    /// A buffer over `ranges` in guest RAM itself, or None unless they are one range inside one importable span.
    pub fn buffer(&mut self, ram: &mut [u8], ranges: &[PaRange]) -> Result<Option<HostBuffer<'a>>, VkError> {
        let [range] = ranges else { return Ok(None) };
        if range.end > ram.len() as u64 || self.failed {
            return Ok(None);
        }
        if self.imported.is_none() {
            self.imported = ImportedRam::new(self.state, ram)?.map(Arc::new);
            self.failed = self.imported.is_none();
        }
        match &self.imported {
            Some(imported) => HostBuffer::imported(self.state, imported, range.start, range.end - range.start),
            None => Ok(None),
        }
    }
}
