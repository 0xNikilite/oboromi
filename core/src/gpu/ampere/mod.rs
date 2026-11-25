//! Ampere GPFIFO submissions. Register offsets in errors are byte offsets.
//! Packet and register layouts follow NVIDIA's `open-gpu-doc` headers
//! `classes/host/clc76f.h` and `classes/dma-copy/clc7b5.h`.
mod block_linear;
mod copy;
mod memory;
pub use memory::AddressSpace;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    InvalidMapping,
    Unmapped { va: u64, size: u64 },
    UnsupportedEntry(u64),
    UnsupportedHeader(u32),
    TruncatedPacket { remaining: u32 },
    DiscontinuousPacket { expected: u64 },
    UnsupportedClass(u32),
    UnboundSubchannel(usize),
    UnsupportedMethod(u32),
    UnsupportedLaunch(u32),
    UnalignedSemaphore(u64),
    UnsupportedPitch,
    UnsupportedSurface,
    OverlappingRows,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SubmitError {
    pub entry: usize,
    pub pushbuffer_va: u64,
    pub fault: Fault,
}

#[derive(Default)]
pub struct Channel {
    copy_bound: [bool; 8],
    copy: copy::CopyEngine,
}

#[derive(Default)]
struct Packet {
    subchannel: usize,
    method: u32,
    remaining: u32,
    mode: u32,
}

impl Channel {
    /// Execute unconditional main-level descriptors in order. A packet can continue
    /// in the next contiguous entry. On fault, earlier method writes remain visible.
    /// The caller supplies one complete submission per call.
    pub fn submit(
        &mut self,
        memory: &mut AddressSpace,
        entries: &[u64],
    ) -> Result<(), SubmitError> {
        const VA_MASK: u64 = (1 << 40) - 1;
        const LENGTH_MASK: u64 = (1 << 21) - 1;
        const ENTRY_FIELDS: u64 = (VA_MASK & !3) | (LENGTH_MASK << 42);
        let mut packet = Packet::default();
        let mut next_va = 0;
        let mut last_data_entry = 0;
        for (entry, &descriptor) in entries.iter().enumerate() {
            if descriptor == 0 {
                continue;
            }
            let va = descriptor & VA_MASK & !3;
            let words = (descriptor >> 42) & LENGTH_MASK;
            let error = |addr, fault| SubmitError {
                entry,
                pushbuffer_va: addr,
                fault,
            };
            let size = words * 4;
            if descriptor & !ENTRY_FIELDS != 0 || words == 0 || va + size > 1 << 40 {
                return Err(error(va, Fault::UnsupportedEntry(descriptor)));
            }
            if packet.remaining != 0 && va != next_va {
                return Err(error(va, Fault::DiscontinuousPacket { expected: next_va }));
            }
            memory.span(va, size).map_err(|fault| error(va, fault))?;
            last_data_entry = entry;
            next_va = va + size;
            for pc in (va..next_va).step_by(4) {
                let bytes = memory.read(pc, 4).map_err(|fault| error(pc, fault))?;
                let word = u32::from_le_bytes(bytes.try_into().unwrap());
                self.word(memory, &mut packet, word)
                    .map_err(|fault| error(pc, fault))?;
            }
        }
        if packet.remaining != 0 {
            return Err(SubmitError {
                entry: last_data_entry,
                pushbuffer_va: next_va,
                fault: Fault::TruncatedPacket {
                    remaining: packet.remaining,
                },
            });
        }
        Ok(())
    }

    fn word(
        &mut self,
        memory: &mut AddressSpace,
        packet: &mut Packet,
        word: u32,
    ) -> Result<(), Fault> {
        if packet.remaining > 0 {
            self.method(memory, packet.subchannel, packet.method, word)?;
            packet.remaining -= 1;
            match packet.mode {
                1 => packet.method += 4,
                5 => {
                    packet.method += 4;
                    packet.mode = 3;
                }
                _ => (),
            }
            return Ok(());
        }
        if word == 0 {
            return Ok(());
        }
        let mode = word >> 29;
        if word & (1 << 12) != 0 || !matches!(mode, 1 | 3 | 4 | 5) {
            return Err(Fault::UnsupportedHeader(word));
        }
        let method = (word & 0xfff) * 4;
        let subchannel = ((word >> 13) & 7) as usize;
        let count = (word >> 16) & 0x1fff;
        if mode == 4 {
            return self.method(memory, subchannel, method, count);
        }
        *packet = Packet {
            subchannel,
            method,
            remaining: count,
            mode,
        };
        Ok(())
    }

    fn method(
        &mut self,
        memory: &mut AddressSpace,
        subchannel: usize,
        offset: u32,
        value: u32,
    ) -> Result<(), Fault> {
        match offset {
            0 => {
                if value != copy::CLASS {
                    return Err(Fault::UnsupportedClass(value));
                }
                self.copy_bound[subchannel] = true;
                Ok(())
            }
            0x08 => Ok(()), // channel NOP
            _ if self.copy_bound[subchannel] => self.copy.method(offset, value, memory),
            _ => Err(Fault::UnboundSubchannel(subchannel)),
        }
    }
}

#[cfg(test)]
mod tests;
