#[cfg(test)]
mod tests {
    use crate::gpu::texture::TexturePool;
    use crate::tests::gpu::harness::{Textures, run_textured};
    use crate::tests::gpu_test::inst::{self, texture};

    const TEXELS_ADDRESS: u64 = 0x1_0000_0400;
    const PITCH: usize = 32;
    const NEAREST: u32 = 1;
    const LINEAR: u32 = 2;

    // A pitch-linear 4x4 RGBA8 header at TIC index 1 and a clamp-to-edge sampler at TSC index 1.
    fn run(program: &[u128], seeds: &[Vec<u32>], texels: &[u8], filter: u32) -> Option<Vec<Vec<u32>>> {
        let header = [
            8 | 0b010_010_010_010 << 7 | 0o5432 << 19,
            TEXELS_ADDRESS as u32,
            (TEXELS_ADDRESS >> 32) as u32 | 2 << 21,
            PITCH as u32 >> 5,
            3 | 1 << 23,
            3 | 1 << 31,
            0,
            0,
        ];
        let sampler = [2 | 2 << 3, filter | filter << 4, 0, 0, 0, 0, 0, 0];
        let entries = |entry: [u32; 8]| -> Vec<u8> { [[0; 8], entry].iter().flatten().flat_map(|w| w.to_le_bytes()).collect() };
        let (tic, tsc) = (entries(header), entries(sampler));
        let rows: Vec<u8> = texels.chunks(16).flat_map(|row| [row, &[0; PITCH - 16]].concat()).collect();
        let memory = [(TEXELS_ADDRESS, &rows[..])];
        let textures = Textures { handle: 1 << 20 | 1, pool: TexturePool { tic: &tic, tsc: &tsc }, memory: &memory };
        run_textured(|d| d.translate_program(program), seeds, &[], &[], &textures).map(|(lanes, _)| lanes)
    }

    #[test]
    fn tld_fetches_unorm_texels_packed_by_write_mask() {
        let texels: Vec<u8> = (0..64).map(|i| 4 * i + 3).collect();
        let seeds: Vec<Vec<u32>> = (0..16).map(|t| vec![0, 0, t % 4, t / 4]).collect();
        let program = [texture(inst::TLD_LZ_2D, 4, 6, 2, 3, 0x10, 0xf), texture(inst::TLD_LZ_2D, 8, 0xff, 2, 3, 0x10, 0b1010)];
        let Some(lanes) = run(&program, &seeds, &texels, NEAREST) else { return };
        // Lavapipe lands one ulp off n / 255, which Vulkan allows, so registers are compared as bytes.
        for (t, regs) in lanes.iter().enumerate() {
            let bytes: Vec<u8> = regs[4..10].iter().map(|&r| (f32::from_bits(r) * 255.0).round() as u8).collect();
            assert_eq!(bytes, [0, 1, 2, 3, 1, 3].map(|c| texels[4 * t + c]), "texel {t}");
        }
    }

    #[test]
    fn tex_lod_zero_nearest_at_texel_centers_matches_tld() {
        let texels: Vec<u8> = (0..64).map(|i| 4 * i + 3).collect();
        let center = |i: u32| ((i as f32 + 0.5) / 4.0).to_bits();
        let seeds: Vec<Vec<u32>> = (0..16).map(|t| vec![t % 4, t / 4, center(t % 4), center(t / 4)]).collect();
        let program = [texture(inst::TLD_LZ_2D, 4, 6, 0, 3, 0x10, 0xf), texture(inst::TEX_LZ_2D, 8, 10, 2, 3, 0x10, 0xf)];
        let Some(lanes) = run(&program, &seeds, &texels, NEAREST) else { return };
        for (t, regs) in lanes.iter().enumerate() {
            assert_eq!(regs[8..12], regs[4..8], "texel {t}");
        }
    }

    #[test]
    fn tex_linear_between_two_texels_gives_their_midpoint() {
        let texels: Vec<u8> = (0..64).map(|i| (i * 37 % 256) as u8).collect();
        let seeds: Vec<Vec<u32>> = (0..12).map(|t| vec![0, 0, ((t % 3 + 1) as f32 / 4.0).to_bits(), ((t / 3) as f32 / 4.0 + 0.125).to_bits()]).collect();
        let program = [texture(inst::TEX_LZ_2D, 4, 6, 2, 3, 0x10, 0xf)];
        let Some(lanes) = run(&program, &seeds, &texels, LINEAR) else { return };
        for (t, regs) in lanes.iter().enumerate() {
            let left = 4 * (t / 3 * 4 + t % 3);
            for c in 0..4 {
                let midpoint = (texels[left + c] as f32 + texels[left + 4 + c] as f32) / 2.0;
                assert!((f32::from_bits(regs[4 + c]) * 255.0 - midpoint).abs() <= 1.0, "lane {t} component {c}");
            }
        }
    }

    #[test]
    fn txq_returns_width_height_and_levels() {
        let program = [texture(inst::TXQ_DIMENSION, 4, 6, 0xff, 3, 0x10, 0b1011)];
        let Some(lanes) = run(&program, &[vec![]], &[0; 64], NEAREST) else { return };
        assert_eq!(lanes[0][4..7], [4, 4, 1]);
    }

    #[test]
    fn sust_texels_read_back_through_suld_in_both_forms() {
        let pattern = |t: u32, base: u32| -> [u8; 4] { [0, 1, 2, 3].map(|c| (base + 16 * t + 4 * c) as u8) };
        let unorm = |byte: u8| (byte as f32 / 255.0).to_bits();
        let seeds: Vec<Vec<u32>> = (0..16)
            .map(|t| {
                let [x, y] = [t % 4, t / 4];
                let [r, g, b, a] = pattern(t, 1).map(unorm);
                vec![0, 0, x, y, 0, u32::from_le_bytes(pattern(t, 2)), 0, 0, r, g, b, a]
            })
            .collect();
        let program = [
            inst::surface(inst::SUST_P_RGBA, 0xff, 2, 8, 3, 0x10),
            inst::surface(inst::SULD_D_32, 4, 2, 0xff, 3, 0x10),
            inst::surface(inst::SUST_D_32, 0xff, 2, 5, 3, 0x10),
            inst::surface(inst::SULD_P_RGBA, 12, 2, 0xff, 3, 0x10),
        ];
        let Some(lanes) = run(&program, &seeds, &[0; 64], NEAREST) else { return };
        for (t, regs) in lanes.iter().enumerate() {
            assert_eq!(regs[4], u32::from_le_bytes(pattern(t as u32, 1)), "lane {t} .D readback");
            let bytes = regs[12..16].iter().map(|&r| (f32::from_bits(r) * 255.0).round() as u8);
            assert_eq!(bytes.collect::<Vec<_>>(), pattern(t as u32, 2), "lane {t} .P readback");
        }
    }
}
