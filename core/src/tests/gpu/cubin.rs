#[cfg(test)]
mod tests {
    use crate::gpu::cubin::Arg;
    use crate::tests::gpu::harness::run_kernel;
    use crate::tests::gpu::program::tests::{arrays, bytes, sha256, string, u32s};

    const CUBIN: &[u8] = include_bytes!("../../../tests/fixtures/kernels.cubin");
    const MANDELBROT: &[u8] = include_bytes!("../../../tests/fixtures/mandelbrot-sm86.cubin");

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

    #[test]
    fn mandelbrot_matches_silicon() {
        let output = vec![0; 800 * 600 * 4];
        let [width, height, crunch, frame, animation, grid_width, blocks] =
            [800i32, 600, 512, 0, 0, 50, 1900].map(i32::to_le_bytes);
        let [x_off, zero, scale] = [-0.5f32, 0.0, 3.2].map(f32::to_le_bytes);
        let args = [
            Arg::Buffer(&output), Arg::Value(&width), Arg::Value(&height), Arg::Value(&crunch),
            Arg::Value(&x_off), Arg::Value(&zero), Arg::Value(&zero), Arg::Value(&zero),
            Arg::Value(&scale), Arg::Value(&[3, 5, 7, 0]), Arg::Value(&frame),
            Arg::Value(&animation), Arg::Value(&grid_width), Arg::Value(&blocks), Arg::Value(&[0]),
        ];
        let name = "_Z11Mandelbrot0IfEvP6uchar4iiiT_S2_S2_S2_S2_S0_iiiib";
        let Some(buffers) = run_kernel(MANDELBROT, name, [1900, 1, 1], [16, 16, 1], &args) else { return };
        assert_eq!(sha256(&buffers[0]), "76d7f1e876402617f9c8a698acff40ad674c5681171a6493893a066523ef1859");
    }
}
