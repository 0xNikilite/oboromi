use crate::audio::demo_music::{music, sequence};
use crate::gpu::demo_bounce::Bounce;

#[test]
fn music_demo_renders_the_same_non_silent_periods_every_run() {
    let run = || {
        let mut sound = music();
        (0..400).flat_map(|_| {
            sequence(&mut sound);
            sound.period().to_vec()
        }).collect::<Vec<i16>>()
    };
    let first = run();
    assert!(first.iter().any(|&s| s != 0));
    assert_eq!(first, run());
}

#[test]
fn bounce_demo_knocks_a_voice_within_the_first_second() {
    let mut bounce = Bounce::new();
    assert!((0..200).any(|_| {
        bounce.step();
        bounce.sound.period().iter().any(|&s| s != 0)
    }));
}
