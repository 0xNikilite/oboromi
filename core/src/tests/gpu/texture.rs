#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::run_textured;
    use crate::tests::gpu_test::inst::tld_lz;

    #[test]
    fn tld_fetches_unorm_texels_packed_by_write_mask() {
        let texels: Vec<u8> = (0..64).map(|i| 4 * i + 3).collect();
        let seeds: Vec<Vec<u32>> = (0..16).map(|t| vec![0, 0, t % 4, t / 4]).collect();
        let program = [tld_lz(4, 6, 2, 3, 0x10, 0xf), tld_lz(8, 0xff, 2, 3, 0x10, 0b1010)];
        let run = run_textured(|d| d.translate_program(&program), &seeds, &[], &[], &[(4, 4, &texels)]);
        let Some((lanes, _)) = run else { return };
        // Lavapipe lands one ulp off n / 255, which Vulkan allows, so registers are compared as bytes.
        for (t, regs) in lanes.iter().enumerate() {
            let bytes: Vec<u8> = regs[4..10].iter().map(|&r| (f32::from_bits(r) * 255.0).round() as u8).collect();
            assert_eq!(bytes, [0, 1, 2, 3, 1, 3].map(|c| texels[4 * t + c]), "texel {t}");
        }
    }
}
