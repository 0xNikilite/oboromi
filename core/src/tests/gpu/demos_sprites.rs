#[cfg(test)]
mod tests {
    use crate::gpu::demo_sprites::{SHAPES, SKY, SUNSET, TARGET, instances, sprites_frame, sprites_guest_with_programs};
    use crate::tests::gpu::harness::{sha256, shared_device};
    use std::io::Write;
    use std::process::{Command, Stdio};

    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 180;
    // Intel Arc and llvmpipe, which round blending into the unorm target and pick a few edge texels differently.
    const DIGESTS: [&str; 2] = [
        "95ca1a7f7a475041fb9e0b9a76664f0bc3b76c48726a24b51f1939f48b25f978",
        "bba65be7e8f6e29017ce2f04d72fc477192502bbccbb2684a635046901a136ff",
    ];

    // The sun is the first instance, alone in the sky at 0 s, so its centre is the disc texel tinted by the sunset over
    // the sky clear colour by the instance's alpha.
    #[test]
    fn nak_compiled_sprites_match_driver_within_unorm_rounding() {
        let Some(state) = shared_device() else { return };
        let programs = [
            [include_bytes!("../../../tests/fixtures/driver-shaders/sprite2d.vert.bin").as_slice(), include_bytes!("../../../tests/fixtures/driver-shaders/sprite2d.frag.bin").as_slice()],
            [include_bytes!("../../../tests/fixtures/driver-shaders/sprite2d-nak.vert.bin").as_slice(), include_bytes!("../../../tests/fixtures/driver-shaders/sprite2d-nak.frag.bin").as_slice()],
        ];
        let [frame, nak] = programs.map(|programs| {
            let mut gpu = sprites_guest_with_programs(state, None, WIDTH, HEIGHT, programs).unwrap();
            sprites_frame(&mut gpu, 0.0).unwrap();
            let mut frame = vec![0; (WIDTH * HEIGHT * 4) as usize];
            gpu.read(TARGET, &mut frame).unwrap();
            frame
        });
        let differences = frame.iter().zip(&nak).filter(|(a, b)| a != b).count();
        let delta = frame.iter().zip(&nak).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
        assert!(differences <= 8 && delta <= 1, "driver {} NAK {}: {differences} bytes differ by at most {delta}", sha256(&frame), sha256(&nak));
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
