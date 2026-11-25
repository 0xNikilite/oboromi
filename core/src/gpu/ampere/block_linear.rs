//! Native 2D block-linear layout. Coordinates are bytes when C7B5 remapping is off.
use super::{AddressSpace, Fault};

#[derive(Default)]
pub(super) struct Surface {
    block_size: u32,
    width: u32,
    height: u32,
    depth: u32,
    layer: u32,
    origin: u32,
}

impl Surface {
    pub fn set(&mut self, register: u32, value: u32) {
        match register {
            0 => self.block_size = value,
            4 => self.width = value,
            8 => self.height = value,
            12 => self.depth = value,
            16 => self.layer = value,
            20 => self.origin = value,
            _ => unreachable!("aligned surface register"),
        }
    }

    pub fn transfer(
        &self,
        ram: &mut AddressSpace,
        upload: bool,
        tiled_va: u64,
        linear_va: u64,
        pitch: u32,
        width: u32,
        rows: u32,
    ) -> Result<(), Fault> {
        let x0 = self.origin & 0xffff;
        let y0 = self.origin >> 16;
        let log_height = (self.block_size >> 4) & 0xf;
        if self.block_size & !0xf0 != 0x1000
            || log_height > 5
            || self.depth != 1
            || self.layer != 0
            || self.width.saturating_sub(x0) < width
            || self.height.saturating_sub(y0) < rows
        {
            return Err(Fault::UnsupportedSurface);
        }
        if rows > 1 && (pitch < width || pitch > i32::MAX as u32) {
            return Err(Fault::UnsupportedPitch);
        }
        let gobs_high = 1u64 << log_height;
        let gobs_wide = u64::from(self.width).div_ceil(64);
        let band_size = gobs_wide * gobs_high * 512;
        let bands = u64::from(self.height).div_ceil(8 * gobs_high);
        let tiled_size = band_size
            .checked_mul(bands)
            .ok_or(Fault::UnsupportedSurface)?;
        let linear_size = u64::from(rows - 1) * u64::from(pitch) + u64::from(width);
        let tiled = ram.span(tiled_va, tiled_size)?;
        let linear = ram.span(linear_va, linear_size)?;
        if linear_va < tiled_va + tiled_size && tiled_va < linear_va + linear_size {
            return Err(Fault::OverlappingRows);
        }
        for row in 0..rows {
            let y = u64::from(y0 + row);
            let band = y / (8 * gobs_high) * band_size;
            let gob_row = y / 8 % gobs_high * 512;
            let mut column = 0;
            while column < width {
                let x = u64::from(x0 + column);
                let gob_column = x / 64 * gobs_high * 512;
                // In a native 64 x 8 byte GOB, low address bits are x[3:0],
                // y[0], y[1], x[4], y[2], x[5], from least to most significant.
                let lane =
                    (x & 15) | ((y & 3) << 4) | ((x & 16) << 2) | ((y & 4) << 5) | ((x & 32) << 3);
                let count = (16 - (x as u32 & 15)).min(width - column);
                let mut a = linear;
                a.start += (u64::from(row) * u64::from(pitch) + u64::from(column)) as usize;
                let mut b = tiled;
                b.start += (band + gob_column + gob_row + lane) as usize;
                if upload {
                    ram.copy(a, b, count as usize);
                } else {
                    ram.copy(b, a, count as usize);
                }
                column += count;
            }
        }
        Ok(())
    }
}
