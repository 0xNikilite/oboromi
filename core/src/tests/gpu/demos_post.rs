#[cfg(test)]
mod tests {
    use crate::gpu::demo_post::{Post, SCENE};
    use crate::tests::gpu::harness::{sha256, shared_device};

    const SIZE: [u32; 2] = [320, 180];

    // Rows 0 to 2 at the top centre and (160, 80) fall in the gaps between blades, so they hold the clear colour in the
    // scene. post.frag keeps green unshifted, dims odd rows (scanlines, sin((y + 0.5) * pi) = -1) and fades corners.
    #[test]
    fn post_pass_darkens_odd_rows_and_corners_of_the_scene_it_samples() {
        let Some(state) = shared_device() else { return };
        let mut post = Post::new(state, None, SIZE, SIZE).unwrap();
        post.frame(0.0).unwrap();
        let mut green = |va: u64, x: u32, y: u32| {
            let mut rgba = [0; 4];
            post.gpu.read(va + u64::from(y * SIZE[0] + x) * 4, &mut rgba).unwrap();
            rgba[1]
        };
        let background = [(160, 0), (160, 1), (160, 2), (160, 80), (0, 0)];
        assert!(background.iter().all(|&(x, y)| green(SCENE, x, y) == green(SCENE, 0, 0)), "{:?}", background.map(|(x, y)| green(SCENE, x, y)));
        let [top, scanline, below, centre, corner] = background.map(|(x, y)| green(post.target, x, y));
        assert!(scanline < top && scanline < below, "rows 0 to 2: {top} {scanline} {below}");
        assert!(corner < centre, "corner {corner} centre {centre}");
        if state.is_llvmpipe() {
            let mut frame = vec![0; (SIZE[0] * SIZE[1] * 4) as usize];
            post.gpu.read(post.target, &mut frame).unwrap();
            assert_eq!(sha256(&frame), "269174a3e362beb6a907da62581fc5392756e813e104ecd341e62b74a33bb649");
        }
    }
}
