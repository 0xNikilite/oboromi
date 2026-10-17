#[cfg(test)]
mod tests {
    use crate::gpu::cubin::Arg;
    use crate::tests::gpu::harness::run_kernel;
    use crate::tests::gpu::program::tests::{arrays, bytes, sha256, string, u32s};

    const CUBIN: &[u8] = include_bytes!("../../../tests/fixtures/kernels.cubin");

    #[test]
    fn cubin_kernels_match_silicon() {
        for json in [include_str!("../../../tests/fixtures/mul-add-32.json"), include_str!("../../../tests/fixtures/mul-add-1024.json")] {
            let [a, b] = [0, 1].map(|i| bytes(&u32s(&arrays(json, "f32_bits")[i])));
            let expected = u32s(&arrays(json, "outputs")[0]);
            let n = expected.len() as u32;
            let args = [Arg::Buffer(&a), Arg::Buffer(&b), Arg::Buffer(&vec![0; 4 * n as usize]), Arg::Value(&n.to_le_bytes())];
            let Some(buffers) = run_kernel(CUBIN, "mul_add", [n.div_ceil(256), 1, 1], [256, 1, 1], &args) else {
                return;
            };
            assert_eq!(buffers[2], bytes(&expected));
        }
        let frame = run_kernel(CUBIN, "gradient", [120, 68, 1], [16, 16, 1], &[Arg::Buffer(&vec![0; 1920 * 1080 * 4])]).unwrap();
        assert_eq!(sha256(&frame[0]), string(include_str!("../../../tests/fixtures/gradient-sm86.json"), "sha256"));
    }
}
