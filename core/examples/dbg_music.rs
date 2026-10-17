fn main() {
    let mut buf = vec![0i16; 48000 * 2];
    let mut t = 0.0f32;
    for round in 0..2 {
        let mut loud = 0usize; let mut frames = 0;
        while t < if round == 0 { 25.0 } else { 5.0 } {
            oboromi_core::audio::demo_music::play_music(t);
            let n = oboromi_core::audio::pull(&mut buf[..1600]);
            frames += n; loud += buf[..n * 2].iter().filter(|&&s| s != 0).count();
            t += 0.016;
        }
        println!("round {round}: frames {frames} loud {loud} gen {}", oboromi_core::audio::generation());
        t = 0.0;
    }
}
