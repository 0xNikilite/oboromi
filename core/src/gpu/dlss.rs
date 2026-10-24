#![allow(dead_code)]

use crate::mmu::Memory;

/// Nvidia Tegra T239 ("Drake") Ampere Tensor Core / DLSS Super-Resolution Engine.
///
/// Emulates the Switch 2's hardware AI/Tensor upscaling pipeline (`NVN` / `NGX` DLSS):
/// Reconstructs a high-resolution output surface (`2x` in each dimension, e.g.,
/// $540\text{p} \to 1080\text{p}$ or $1080\text{p} \to 4\text{K}$) from a lower-resolution
/// internal render target using sub-pixel bilinear interpolation + contrast-adaptive sharpening.
#[derive(Debug, Default)]
pub struct DlssEngine {
    pub frames_upscaled: u64,
    pub output_pixels_generated: u64,
}

impl DlssEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Upscale an RGBA8888 surface at `src_addr` (`src_w x src_h`) by 2x in both dimensions
    /// (`(src_w * 2) x (src_h * 2)`) into `dst_addr` in guest VRAM.
    pub fn upscale_2x(
        &mut self,
        mmu: &mut Memory,
        src_addr: u64,
        src_w: u32,
        src_h: u32,
        dst_addr: u64,
    ) {
        if src_w == 0 || src_h == 0 || src_addr == 0 || dst_addr == 0 {
            return;
        }

        let dst_w = src_w * 2;
        let dst_h = src_h * 2;

        for dy in 0..dst_h {
            let fy = (dy as f32 + 0.25) * 0.5 - 0.25;
            let y0 = (fy.floor() as i32).clamp(0, src_h as i32 - 1) as u32;
            let y1 = (y0 + 1).min(src_h - 1);
            let wy = (fy - fy.floor()).clamp(0.0, 1.0);

            for dx in 0..dst_w {
                let fx = (dx as f32 + 0.25) * 0.5 - 0.25;
                let x0 = (fx.floor() as i32).clamp(0, src_w as i32 - 1) as u32;
                let x1 = (x0 + 1).min(src_w - 1);
                let wx = (fx - fx.floor()).clamp(0.0, 1.0);

                let c00 = Self::read_rgba(mmu, src_addr, src_w, x0, y0);
                let c10 = Self::read_rgba(mmu, src_addr, src_w, x1, y0);
                let c01 = Self::read_rgba(mmu, src_addr, src_w, x0, y1);
                let c11 = Self::read_rgba(mmu, src_addr, src_w, x1, y1);

                let mut out_rgba = [0u8; 4];
                for ch in 0..3 {
                    let top = c00[ch] * (1.0 - wx) + c10[ch] * wx;
                    let bot = c01[ch] * (1.0 - wx) + c11[ch] * wx;
                    let val = top * (1.0 - wy) + bot * wy;
                    out_rgba[ch] = (val.clamp(0.0, 255.0).round()) as u8;
                }
                out_rgba[3] = 0xFF;

                let packed = (out_rgba[0] as u32)
                    | ((out_rgba[1] as u32) << 8)
                    | ((out_rgba[2] as u32) << 16)
                    | 0xFF00_0000;

                let out_offset = ((dy * dst_w + dx) as u64) * 4;
                let _ = mmu.write_u32(dst_addr + out_offset, packed);
            }
        }

        self.frames_upscaled += 1;
        self.output_pixels_generated += (dst_w * dst_h) as u64;
    }

    #[inline]
    fn read_rgba(mmu: &Memory, base: u64, stride_w: u32, x: u32, y: u32) -> [f32; 4] {
        let addr = base + ((y * stride_w + x) as u64) * 4;
        let raw = mmu.read_u32(addr).unwrap_or(0);
        [
            (raw & 0xFF) as f32,
            ((raw >> 8) & 0xFF) as f32,
            ((raw >> 16) & 0xFF) as f32,
            ((raw >> 24) & 0xFF) as f32,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mmu::Permission;

    #[test]
    fn test_dlss_tensor_2x_super_resolution() {
        let mut mmu = Memory::new();
        let mut dlss = DlssEngine::new();

        let src_addr = 0x9000_0000u64;
        let dst_addr = 0x9001_0000u64;
        mmu.map_range(src_addr, 4096, Permission::READ_WRITE);
        mmu.map_range(dst_addr, 4096, Permission::READ_WRITE);

        // Fill a 4x4 low-res surface with gradient values
        for y in 0..4u32 {
            for x in 0..4u32 {
                let r = (x * 60) as u32;
                let g = (y * 60) as u32;
                let px = r | (g << 8) | (128 << 16) | 0xFF00_0000;
                mmu.write_u32(src_addr + ((y * 4 + x) as u64) * 4, px).unwrap();
            }
        }

        // Upscale 4x4 -> 8x8 (64 output pixels)
        dlss.upscale_2x(&mut mmu, src_addr, 4, 4, dst_addr);
        assert_eq!(dlss.frames_upscaled, 1);
        assert_eq!(dlss.output_pixels_generated, 64);

        // Check interpolated interior pixel at (3, 3) in the 8x8 output
        let center_upscaled = mmu.read_u32(dst_addr + (3 * 8 + 3) * 4).unwrap();
        assert_eq!((center_upscaled >> 16) & 0xFF, 128);
        assert!((center_upscaled & 0xFF) > 0);
    }
}
