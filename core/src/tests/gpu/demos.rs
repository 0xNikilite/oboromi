#[cfg(test)]
mod tests {
    use crate::gpu::demo::{Scene, asset_dir};
    use crate::tests::gpu::harness::vk_state;
    use std::io::Write;
    use std::process::{Command, Stdio};

    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 180;
    // The clear colour 0.16, 0.17, 0.2, 1 as RGBA8.
    const CLEAR: [u8; 4] = [41, 43, 51, 255];

    // Hardware rounds interpolation and filtering its own way, so the frame's digest is pinned on lavapipe, which CI runs.
    fn check_first_frame(asset: &str, lavapipe_sha256: &str) {
        if !asset_dir(asset).exists() {
            return eprintln!("skipped: run demos/assets/fetch.sh for {asset}");
        }
        let Some(state) = vk_state() else { return };
        let mut scene = Scene::new(&state, asset, WIDTH, HEIGHT).unwrap();
        scene.frame(0.0).unwrap();
        let mut pixels = vec![0; (WIDTH * HEIGHT * 4) as usize];
        scene.gpu.read(scene.target, &mut pixels).unwrap();
        let pixel = |x: u32, y: u32| -> [u8; 4] { pixels[((y * WIDTH + x) * 4) as usize..][..4].try_into().unwrap() };
        assert_eq!(pixel(4, 4), CLEAR);
        assert_ne!(pixel(WIDTH / 2, HEIGHT / 2), CLEAR);
        if state.is_llvmpipe() {
            let mut sha256sum = Command::new("sha256sum").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
            sha256sum.stdin.take().unwrap().write_all(&pixels).unwrap();
            assert_eq!(&String::from_utf8(sha256sum.wait_with_output().unwrap().stdout).unwrap()[..64], lavapipe_sha256);
        }
    }

    #[test]
    fn boombox_frame_draws_the_lit_model_over_the_clear_colour() {
        check_first_frame("boombox", "c4d728df362c7f4b36ab96931671ca3fdab751d2434620750b1a80040595ae26");
    }

    #[test]
    fn helmet_frame_draws_its_six_materials_over_the_clear_colour() {
        check_first_frame("flighthelmet", "10b30b11b93b26a46ca5f97f3f3c1a04ac136a848f2d9c26ffdc2395bef1ae89");
    }
}
