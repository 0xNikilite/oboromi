//! Guest GPU memory: each address space maps GPU VA pages onto guest-physical pages of guest RAM.
use std::collections::HashMap;
use std::ops::Range;

// The big page size nvhost-as-gpu maps with (Ryujinx NvHostAsGpuDeviceFile BigPageSize).
pub const PAGE: u64 = 0x1_0000;

pub type PaRange = Range<u64>;

#[derive(Debug)]
pub enum MemoryError {
    Unmapped(u64),
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
