//! Synchronous 2D copy subset of NVIDIA class C7B5 (AMPERE_DMA_COPY_B).
use super::block_linear::Surface;
use super::{AddressSpace, Fault};

pub(super) const CLASS: u32 = 0xc7b5;
const LAUNCH_DMA: u32 = 0x300;
const SRC_PITCH: u32 = 1 << 7;
const DST_PITCH: u32 = 1 << 8;
const MULTI_LINE: u32 = 1 << 9;
const RELEASE_WORD: u32 = 1 << 3;
const FLUSH: u32 = 1 << 2;

#[derive(Default)]
struct Address {
    upper: u32,
    lower: u32,
}

impl Address {
    fn va(&self) -> u64 {
        (u64::from(self.upper) << 32) | u64::from(self.lower)
    }
}

#[derive(Default)]
pub(super) struct CopyEngine {
    src_surface: Surface,
    dst_surface: Surface,
    src: Address,
    dst: Address,
    semaphore: Address,
    payload: u32,
    src_pitch: u32,
    dst_pitch: u32,
    width: u32,
    height: u32,
}

impl CopyEngine {
    pub fn method(
        &mut self,
        offset: u32,
        value: u32,
        memory: &mut AddressSpace,
    ) -> Result<(), Fault> {
        match offset {
            0x100 => (),
            0x240 => self.semaphore.upper = value & 0x1ffff,
            0x244 => self.semaphore.lower = value,
            0x248 => self.payload = value,
            0x70c..=0x720 => self.dst_surface.set(offset - 0x70c, value),
            0x728..=0x73c => self.src_surface.set(offset - 0x728, value),
            LAUNCH_DMA => return self.launch(value, memory),
            0x400 => self.src.upper = value & 0x1ffff,
            0x404 => self.src.lower = value,
            0x408 => self.dst.upper = value & 0x1ffff,
            0x40c => self.dst.lower = value,
            0x410 => self.src_pitch = value,
            0x414 => self.dst_pitch = value,
            0x418 => self.width = value,
            0x41c => self.height = value,
            _ => return Err(Fault::UnsupportedMethod(offset)),
        }
        Ok(())
    }

    fn launch(&self, flags: u32, memory: &mut AddressSpace) -> Result<(), Fault> {
        let transfer = flags & 3;
        let layouts = SRC_PITCH | DST_PITCH;
        let supported = 3 | FLUSH | RELEASE_WORD | layouts | MULTI_LINE;
        if flags & !supported != 0
            || transfer == 3
            || (transfer != 0
                && flags & layouts != layouts
                && (flags & MULTI_LINE == 0 || flags & layouts == 0))
        {
            return Err(Fault::UnsupportedLaunch(flags));
        }
        let signal = flags & RELEASE_WORD != 0;
        if signal {
            if self.semaphore.va() & 3 != 0 {
                return Err(Fault::UnalignedSemaphore(self.semaphore.va()));
            }
            memory.span(self.semaphore.va(), 4)?;
        }
        let rows = if flags & MULTI_LINE != 0 {
            self.height
        } else {
            1
        };
        if transfer != 0 && rows > 0 && self.width > 0 {
            match flags & layouts {
                SRC_PITCH => self.dst_surface.transfer(
                    memory,
                    true,
                    self.dst.va(),
                    self.src.va(),
                    self.src_pitch,
                    self.width,
                    rows,
                )?,
                DST_PITCH => self.src_surface.transfer(
                    memory,
                    false,
                    self.src.va(),
                    self.dst.va(),
                    self.dst_pitch,
                    self.width,
                    rows,
                )?,
                _ => self.copy_rows(rows, memory)?,
            }
        }
        if signal {
            memory.write(self.semaphore.va(), &self.payload.to_le_bytes())?;
        }
        Ok(())
    }

    fn copy_rows(&self, rows: u32, memory: &mut AddressSpace) -> Result<(), Fault> {
        if rows > 1
            && (self.src_pitch < self.width
                || self.dst_pitch < self.width
                || self.src_pitch > i32::MAX as u32
                || self.dst_pitch > i32::MAX as u32)
        {
            return Err(Fault::UnsupportedPitch);
        }
        let extent = |pitch| u64::from(rows - 1) * u64::from(pitch) + u64::from(self.width);
        let src_size = extent(self.src_pitch);
        let dst_size = extent(self.dst_pitch);
        // Check both surfaces first so a bad final row cannot leave a partial copy.
        let mut src = memory.span(self.src.va(), src_size)?;
        let mut dst = memory.span(self.dst.va(), dst_size)?;
        if rows > 1
            && src.mapping == dst.mapping
            && self.src.va() < self.dst.va() + dst_size
            && self.dst.va() < self.src.va() + src_size
        {
            // Two rectangles in the same image can share padding without sharing pixels.
            let mut a = self.src.va();
            let mut b = self.dst.va();
            let width = u64::from(self.width);
            while a < self.src.va() + src_size && b < self.dst.va() + dst_size {
                if a + width <= b {
                    a += u64::from(self.src_pitch);
                } else if b + width <= a {
                    b += u64::from(self.dst_pitch);
                } else {
                    return Err(Fault::OverlappingRows);
                }
            }
        }
        for row in 0..rows {
            if row != 0 {
                src.start += self.src_pitch as usize;
                dst.start += self.dst_pitch as usize;
            }
            memory.copy(src, dst, self.width as usize);
        }
        Ok(())
    }
}
