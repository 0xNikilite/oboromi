#[cfg(test)]
mod tests {
    use crate::gpu::demo_sprites::{SHAPES, SKY, SUNSET, TARGET, instances, sprites_frame, sprites_guest};
    use crate::tests::gpu::harness::vk_state;
    use std::io::Write;
    use std::process::{Command, Stdio};

    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 180;
    // Intel Arc and llvmpipe, which round blending into the unorm target and pick a few edge texels differently.
    const DIGESTS: [&str; 2] = [
        "0831567c32572df6277f4934eaeeb9ee131191ac2d0f0f74d452a498796a9dd9",
        "6d194d53c79fd1c5b0ed6fa544895b6d775b73e65123dbf80e4acde603b2163c",
    ];

    // The sun is the first instance, alone in the sky at 0 s, so its centre is the disc texel tinted by the sunset over
    // the sky clear colour by the instance's alpha.
    #[test]
    fn sprites_frame_blends_the_tinted_sun_over_the_sky() {
        let Some(state) = vk_state() else { return };
        let mut gpu = sprites_guest(&state, WIDTH, HEIGHT).unwrap();
        sprites_frame(&mut gpu, 0.0).unwrap();
        let mut frame = vec![0; (WIDTH * HEIGHT * 4) as usize];
        gpu.read(TARGET, &mut frame).unwrap();
        let alpha = instances(0.0)[0][11];
        let want = [0, 1, 2].map(|c| (f32::from(SHAPES[0][c]) * SUNSET[c] * alpha + (SKY[c] * 255.0).round() * (1.0 - alpha)).round() as u8);
        let got = &frame[((22 * WIDTH + 270) * 4) as usize..][..4];
        assert!(got[..3].iter().zip(want).all(|(&g, w)| g.abs_diff(w) <= 1) && got[3] == 255, "{got:?} != {want:?}");
        let mut sha256sum = Command::new("sha256sum").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
        sha256sum.stdin.take().unwrap().write_all(&frame).unwrap();
        let digest = String::from_utf8(sha256sum.wait_with_output().unwrap().stdout).unwrap();
        assert!(DIGESTS.contains(&&digest[..64]), "{digest}");
    }
}
