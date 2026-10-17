#[cfg(test)]
mod tests {
    use crate::gpu::demo::{Scene, asset_dir};
    use crate::tests::gpu::harness::shared_device;
    use std::io::Write;
    use std::process::{Command, Stdio};

    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 180;
    // The clear colour 0.02, 0.02, 0.03, 1 as RGBA8.
    const CLEAR: [u8; 4] = [5, 5, 8, 255];

    // Hardware rounds interpolation and filtering its own way, so the frame's digest is pinned on lavapipe, which CI runs.
    fn check_first_frame(asset: &str, lavapipe_sha256: &str) {
        let Some(state) = shared_device() else { return };
        let mut scene = Scene::new(state, None, asset, WIDTH, HEIGHT).unwrap();
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
        check_first_frame("boombox", "4033f8312986d3ccee78e64e568c39ee57b67426d9e70269d976fc7606cb1957");
    }

    #[test]
    fn helmet_frame_draws_its_six_materials_over_the_clear_colour() {
        if !asset_dir("flighthelmet").exists() {
            return eprintln!("skipped: run demos/assets/fetch.sh for the FlightHelmet");
        }
        check_first_frame("flighthelmet", "df0a73df3ca1032eb27552361619d92d08c01f1ebd47e28ca7a8d7c68fb61e4f");
    }
}
