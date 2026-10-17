use super::{Scene, magic, memory, parameter, record};
use crate::audio::renderer::{AudioRendererParameter, work_buffer_size};
use crate::audio::voice::{ADPCM, PCM_INT16};

fn waveform() -> (Vec<i16>, Vec<u8>) {
    let samples: Vec<i16> = (0..2400).map(|n| ((n % 100) as i16 - 50) * 400).collect();
    let bytes = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    (samples, bytes)
}

fn pcm_scene(revision: u32, effects: Vec<Vec<u8>>) -> Scene {
    Scene {
        revision,
        format: PCM_INT16,
        samples: 2400,
        looping: false,
        coefficients: None,
        mix: [0.5, 0.25],
        effects,
    }
}

#[test]
fn work_buffer_size_matches_the_reference_layout() {
    let set = |sample_rate,
               sample_count,
               mix_buffer_count,
               sub_mix_count,
               voice_count,
               sink_count,
               effect_count,
               performance_frame_count,
               splitter_count,
               splitter_destination_count,
               revision| {
        let parameter = AudioRendererParameter {
            sample_rate,
            sample_count,
            mix_buffer_count,
            sub_mix_count,
            voice_count,
            sink_count,
            effect_count,
            performance_frame_count,
            splitter_count,
            splitter_destination_count,
            revision: magic(revision),
        };
        work_buffer_size(&parameter).unwrap()
    };
    assert_eq!(set(48_000, 240, 6, 0, 1, 1, 0, 0, 0, 0, 1), 0x7000);
    assert_eq!(set(48_000, 240, 24, 4, 32, 2, 4, 1, 2, 4, 2), 0x40000);
    assert_eq!(set(48_000, 240, 24, 4, 32, 2, 4, 1, 2, 4, 13), 0x41000);
    assert_eq!(set(48_000, 240, 24, 4, 32, 2, 4, 1, 2, 4, 14), 0x41000);
    assert_eq!(set(48_000, 240, 24, 4, 32, 2, 4, 1, 2, 4, 15), 0x41000);
    assert_eq!(set(48_000, 240, 22, 1, 96, 1, 1, 0, 0, 0, 6), 0x34000);
    assert_eq!(set(32_000, 160, 6, 0, 24, 1, 0, 0, 0, 0, 2), 0xD000);
    assert!(work_buffer_size(&parameter(16, 0)).is_err());
}

#[test]
fn pcm16_voice_reaches_the_device_sink_at_its_mix_volumes() {
    let (samples, bytes) = waveform();
    let scene = pcm_scene(13, Vec::new());
    let (mut renderer, output, status) = scene.play(&mut memory(&bytes), 10);
    assert_eq!(status[0x40..0x44], 5u32.to_le_bytes());
    let expected: Vec<i16> = samples.iter().flat_map(|&s| [s / 2, s / 4]).collect();
    assert_eq!(output, expected);
    assert_eq!(output, scene.play(&mut memory(&bytes), 10).1);
    let mut next = vec![0; 0x1000];
    renderer
        .request_update(&scene.parcel(false), &mut next)
        .unwrap();
    let voice = 0x40 + 4 * 0x10;
    assert_eq!(
        next[voice..voice + 12],
        [2400u64.to_le_bytes().as_slice(), &1u32.to_le_bytes()].concat()[..]
    );
    assert!(renderer.pending_event() && !renderer.pending_event());
}

#[test]
fn biquad_and_delay_effects_run_on_the_final_mix() {
    for revision in [13, 15] {
        biquad_and_delay_effects_at(revision);
    }
}

fn biquad_and_delay_effects_at(revision: u32) {
    let (samples, bytes) = waveform();
    let (b0_at, b0, channels): (_, &[u8], _) = if revision >= 15 {
        (0x30, &0.5f32.to_le_bytes(), 0x44)
    } else {
        (0x2C, &8192i16.to_le_bytes(), 0x36)
    };
    let half_gain = record(
        0xC0,
        &[
            (0, &[6, 1, 1]),
            (0x20, &[0]),
            (0x26, &[0]),
            (b0_at, b0),
            (channels, &[1]),
        ],
    );
    let q14 = |v: f32| ((v * 16384.0) as u32).to_le_bytes();
    let one_period_delay = record(
        0xC0,
        &[
            (0, &[3, 1, 1]),
            (0x18, &1u32.to_le_bytes()),
            (0x20, &[1]),
            (0x26, &[1]),
            (0x2C, &1u16.to_le_bytes()),
            (0x2E, &1u16.to_le_bytes()),
            (0x30, &5u32.to_le_bytes()),
            (0x34, &5u32.to_le_bytes()),
            (0x3C, &q14(1.0)),
            (0x44, &q14(1.0)),
        ],
    );
    let (_, output, status) =
        pcm_scene(revision, vec![half_gain, one_period_delay]).play(&mut memory(&bytes), 10);
    let effects = 0x40 + 6 * 0x10 + 0x10;
    assert_eq!([status[effects], status[effects + 0x90]], [3, 3]);
    let left: Vec<i16> = output.iter().step_by(2).copied().collect();
    let right: Vec<i16> = output.iter().skip(1).step_by(2).copied().collect();
    assert_eq!(left, samples.iter().map(|s| s / 4).collect::<Vec<_>>());
    assert_eq!(right[..240], [0; 240]);
    assert_eq!(
        right[240..],
        samples[..2160].iter().map(|s| s / 4).collect::<Vec<_>>()[..]
    );
}

#[test]
fn adpcm_voice_decodes_with_the_pool_coefficient_table() {
    let frame = [0x18, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11];
    let mut data = frame.repeat(4);
    data.resize(0x8000, 0);
    data.extend_from_slice(&[0, 0, 0, 0, 0, 8, 0, 0]);
    let scene = Scene {
        revision: 13,
        format: ADPCM,
        samples: 56,
        looping: false,
        coefficients: Some(super::BASE + 0x8000),
        mix: [0.5, 0.5],
        effects: Vec::new(),
    };
    let (_, output, _) = scene.play(&mut memory(&data), 1);
    let left: Vec<i16> = output.iter().step_by(2).copied().collect();
    assert_eq!(
        left[..56],
        (1..=56).map(|n| n * 128).collect::<Vec<i16>>()[..]
    );
    assert_eq!(left[56..], [0; 184]);
}

#[test]
fn rev1_parcel_without_rev8_fields_loops_forever() {
    let (_, bytes) = waveform();
    let scene = Scene {
        revision: 1,
        samples: 240,
        looping: true,
        ..pcm_scene(1, Vec::new())
    };
    let (mut renderer, output, status) = scene.play(&mut memory(&bytes), 3);
    assert_eq!(output[..480], output[480..960]);
    assert_eq!(output[..480], output[960..]);
    let total = 0x40 + 4 * 0x10 + 0x10 + 0x20 + 0x10 + 0xB0;
    assert_eq!(status[0x3C..0x40], (total as u32).to_le_bytes());
    let mut next = vec![0; 0x1000];
    renderer
        .request_update(&scene.parcel(false), &mut next)
        .unwrap();
    assert_eq!(next[0x40 + 4 * 0x10 + 8..][..4], 0u32.to_le_bytes());
}
