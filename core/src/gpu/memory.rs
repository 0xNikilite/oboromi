//! Guest GPU memory: each address space maps GPU VA pages onto guest-physical pages of guest RAM, and the
//! `MemoryTracker` over those physical pages decides what host mirrors see, so mirrors key on PA and generation.
use super::compute::HostBuffer;
use crate::cpu::GuestRam;
use super::{VkError, VkState};
use std::collections::HashMap;
use std::ops::Range;

// The big page size nvhost-as-gpu maps with (Ryujinx NvHostAsGpuDeviceFile BigPageSize).
pub const PAGE: u64 = 0x1_0000;

pub type PaRange = Range<u64>;

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
}

/// A host-visible copy of guest-physical ranges, gathered in order into one buffer.
pub struct Mapping<'a> {
    pub buffer: HostBuffer<'a>,
    pub ranges: Vec<PaRange>,
}

impl Mapping<'_> {
    pub fn refresh(&mut self, ram: &GuestRam) -> Result<(), VkError> {
        let bytes = self.ranges.iter().map(|range| unsafe { ram.slice(range.start, (range.end - range.start) as usize) }).collect::<Vec<_>>().concat();
        self.buffer.write(&bytes)
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

    pub fn upload<'a>(&self, state: &'a VkState, ram: &GuestRam, ranges: impl IntoIterator<Item = PaRange>) -> Result<Mapping<'a>, VkError> {
        let ranges: Vec<_> = ranges.into_iter().collect();
        let len = ranges.iter().map(|range| range.end - range.start).sum::<u64>();
        let mut mapping = Mapping { buffer: HostBuffer::new(state, len.max(1))?, ranges };
        mapping.refresh(ram)?;
        Ok(mapping)
    }

    pub fn download(&mut self, mapping: &Mapping, ram: &GuestRam) -> Result<(), VkError> {
        let mut bytes = vec![0; mapping.ranges.iter().map(|range| (range.end - range.start) as usize).sum()];
        mapping.buffer.read(&mut bytes)?;
        self.store(ram, &mapping.ranges, &bytes, Writer::Gpu);
        Ok(())
    }

    /// Scatters `bytes` over `ranges` in order.
    pub fn store(&mut self, ram: &GuestRam, ranges: &[PaRange], bytes: &[u8], writer: Writer) {
        let mut rest = bytes;
        for range in ranges {
            let (head, tail) = rest.split_at((range.end - range.start) as usize);
            unsafe { ram.slice_mut(range.start, head.len()) }.copy_from_slice(head);
            self.mark(range.start, range.end - range.start, writer);
            rest = tail;
        }
    }

    /// The global-memory SSBO window for `va..va + len` and the VA its first byte holds, aligned down for 128-bit
    /// accesses; the whole window must be mapped.
    pub fn window<'a>(&self, state: &'a VkState, ram: &GuestRam, space: &PageMap, va: u64, len: u64) -> Result<(Mapping<'a>, u64), MemoryError> {
        let base = va & !0xf;
        Ok((self.upload(state, ram, space.resolve(base, va + len - base)?)?, base))
    }
}
