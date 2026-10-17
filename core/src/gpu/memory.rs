//! Guest GPU memory: each address space maps GPU VA pages onto guest-physical pages of guest RAM, and the
//! `MemoryTracker` over those physical pages decides what host mirrors see, so mirrors key on PA and generation.
use super::compute::HostBuffer;
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

/// `ranges` cut at page boundaries, in order.
fn pieces(ranges: &[PaRange]) -> impl Iterator<Item = PaRange> + '_ {
    ranges.iter().flat_map(|range| pages(range.start, range.end - range.start).map(|page| (page * PAGE).max(range.start)..((page + 1) * PAGE).min(range.end)))
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
        self.store(ram, &mapping.ranges, mapping.buffer.view()?, Writer::Gpu);
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
