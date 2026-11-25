//! Host-backed GPU virtual ranges for command submission and DMA.
use super::Fault;

struct Mapping {
    base: u64,
    bytes: Vec<u8>,
}

#[derive(Default)]
pub struct AddressSpace {
    mappings: Vec<Mapping>,
}

#[derive(Clone, Copy)]
pub(super) struct Span {
    pub mapping: usize,
    pub start: usize,
}

impl AddressSpace {
    /// Registers a nonempty, disjoint range in the copy engine's 49-bit address space.
    pub fn map(&mut self, base: u64, bytes: Vec<u8>) -> Result<(), Fault> {
        let end = base
            .checked_add(bytes.len() as u64)
            .filter(|&end| end <= 1 << 49 && end > base)
            .ok_or(Fault::InvalidMapping)?;
        let i = self.mappings.partition_point(|m| m.base < base);
        if i > 0 {
            let prev = &self.mappings[i - 1];
            if prev.base + prev.bytes.len() as u64 > base {
                return Err(Fault::InvalidMapping);
            }
        }
        if self.mappings.get(i).is_some_and(|next| next.base < end) {
            return Err(Fault::InvalidMapping);
        }
        self.mappings.insert(i, Mapping { base, bytes });
        Ok(())
    }

    pub fn unmap(&mut self, base: u64) -> Option<Vec<u8>> {
        let i = self.mappings.binary_search_by_key(&base, |m| m.base).ok()?;
        Some(self.mappings.remove(i).bytes)
    }

    pub(super) fn span(&self, va: u64, size: u64) -> Result<Span, Fault> {
        let fault = Fault::Unmapped { va, size };
        let i = self
            .mappings
            .partition_point(|m| m.base <= va)
            .checked_sub(1)
            .ok_or(fault)?;
        let mapping = &self.mappings[i];
        let offset = va - mapping.base;
        if offset
            .checked_add(size)
            .is_none_or(|end| end > mapping.bytes.len() as u64)
        {
            return Err(fault);
        }
        Ok(Span {
            mapping: i,
            start: offset as usize,
        })
    }

    pub fn read(&self, va: u64, size: usize) -> Result<&[u8], Fault> {
        let span = self.span(va, size as u64)?;
        Ok(&self.mappings[span.mapping].bytes[span.start..span.start + size])
    }

    pub fn write(&mut self, va: u64, bytes: &[u8]) -> Result<(), Fault> {
        let span = self.span(va, bytes.len() as u64)?;
        self.mappings[span.mapping].bytes[span.start..span.start + bytes.len()]
            .copy_from_slice(bytes);
        Ok(())
    }

    pub(super) fn copy(&mut self, src: Span, dst: Span, width: usize) {
        let range = src.start..src.start + width;
        if src.mapping == dst.mapping {
            self.mappings[src.mapping]
                .bytes
                .copy_within(range, dst.start);
            return;
        }
        let (low, high) = self.mappings.split_at_mut(src.mapping.max(dst.mapping));
        let (input, output) = if src.mapping < dst.mapping {
            (&low[src.mapping], &mut high[0])
        } else {
            (&high[0], &mut low[dst.mapping])
        };
        output.bytes[dst.start..dst.start + width].copy_from_slice(&input.bytes[range]);
    }
}
