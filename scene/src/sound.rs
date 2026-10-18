//! Note sequencing and audren parcels shared by the host and guest demos.
use core::f32::consts::FRAC_PI_4;
pub const REVISION: u32 = 0x3d76_4552;
pub const EFFECTS: usize = 2;
#[derive(Clone, Copy, Default)]
pub struct Note {
    pub sample: usize, pub pitch: f32, pub volume: f32, pub pan: f32,
    pub used: bool, pub start: bool, pub ends: u64,
}
impl Note {
    pub fn new(period: u64, samples: &[(u64, u32)], sample: usize, pitch: f32, volume: f32, pan: f32) -> Self {
        Self { sample, pitch, volume, pan, used: true, start: true, ends: period + (samples[sample].1 as f32 / pitch / 240.0) as u64 + 1 }
    }
}
pub fn free_voice(notes: &[Note]) -> usize { (0..notes.len()).min_by_key(|&v| notes[v].ends).unwrap() }
pub fn parameter(voices: usize, sinks: u32) -> [u32; 13] { [48000, 240, 2, 0, voices as u32, sinks, 2, 0, 0, 0, 0, 0, REVISION] }
pub fn parcel_size(voices: usize, sinks: usize) -> usize { 0x40 + 0x10 + 0x20 * (EFFECTS + 4 * voices) + 0x70 * voices + 0x170 * voices + 0xc0 * EFFECTS + 0x950 + 0x140 * sinks + 0x10 }
fn record(bytes: &mut [u8], at: usize, fields: &[(usize, &[u8])]) {
    for &(offset, value) in fields { bytes[at + offset..at + offset + value.len()].copy_from_slice(value); }
}
fn sin_cos(angle: f32) -> (f32, f32) {
    let x = f64::from(angle) % core::f64::consts::TAU;
    let x = if x > core::f64::consts::PI { x - core::f64::consts::TAU } else if x < -core::f64::consts::PI { x + core::f64::consts::TAU } else { x };
    let (mut sin, mut cos, mut s, mut c) = (x, 1.0, x, 1.0);
    for n in 1..13 {
        s *= -x * x / ((2 * n) * (2 * n + 1)) as f64;
        c *= -x * x / ((2 * n - 1) * (2 * n)) as f64;
        sin += s;
        cos += c;
    }
    (sin as f32, cos as f32)
}
pub fn parcel(bytes: &mut [u8], notes: &mut [Note], samples: &[(u64, u32)], pool: (u64, u64), first: bool, biquad: &[u8; 10], ring: Option<u64>) -> usize {
    let (voices, sinks) = (notes.len(), 1 + usize::from(ring.is_some()));
    let sizes = [0x10, 0x20 * (EFFECTS + 4 * voices), 0x70 * voices, 0x170 * voices, 0xc0 * EFFECTS, 0x950, 0x140 * sinks, 0x10];
    let len = parcel_size(voices, sinks);
    bytes[..len].fill(0);
    record(bytes, 0, &[(0, &REVISION.to_le_bytes()), (0x3c, &(len as u32).to_le_bytes())]);
    for (slot, section) in [0,1,3,2,4,5,6,7].into_iter().enumerate() { record(bytes, 4 + slot * 4, &[(0, &(sizes[section] as u32).to_le_bytes())]); }
    let mut at = 0x40;
    record(bytes, at, &[(0, &REVISION.to_le_bytes())]); at += sizes[0];
    record(bytes, at, &[(0, &pool.0.to_le_bytes()), (8, &pool.1.to_le_bytes()), (0x10, &4u32.to_le_bytes())]); at += sizes[1];
    for (i, note) in notes.iter_mut().enumerate() {
        let id = (i as u32).to_le_bytes();
        let (sin, cos) = sin_cos((note.pan.clamp(-1.0, 1.0) + 1.0) * FRAC_PI_4);
        record(bytes, at + i * 0x70, &[(0, &id), (4, &cos.to_le_bytes()), (8, &sin.to_le_bytes()), (0x64, &[1])]);
        let (address, samples) = samples[note.sample];
        let voice = at + sizes[2] + i * 0x170;
        record(bytes, voice, &[(0, &id), (8, &[u8::from(note.start), u8::from(note.used), 0, 2]), (0xc, &48000u32.to_le_bytes()), (0x18, &1u32.to_le_bytes()), (0x1c, &note.pitch.to_le_bytes()), (0x20, &note.volume.to_le_bytes()), (0x5c, &u32::MAX.to_le_bytes()), (0x140, &id)]);
        record(bytes, voice + 0x60, &[(0, &address.to_le_bytes()), (8, &(u64::from(samples) * 2).to_le_bytes()), (0x14, &samples.to_le_bytes()), (0x1a, &[u8::from(!note.start)])]);
        note.start = false;
    }
    at += sizes[2] + sizes[3];
    record(bytes, at, &[(0, &[6, u8::from(first), 1]), (0x20, &[0,1]), (0x26, &[0,1]), (0x2c, biquad), (0x36, &[2,u8::from(!first)])]);
    record(bytes, at + 0xc0, &[(0, &[3,u8::from(first),1]), (0x18, &1u32.to_le_bytes()), (0x20, &[0,1]), (0x26, &[0,1]), (0x2c, &[2,0,2,0]), (0x30, &400u32.to_le_bytes()), (0x34, &240u32.to_le_bytes())]);
    for (i, gain) in [0.6f32,0.35,0.5,1.0,0.6,0.3].into_iter().enumerate() { record(bytes, at + 0xc0 + 0x3c + i * 4, &[(0, &((gain * 16384.0) as u32).to_le_bytes())]); }
    at += sizes[4];
    record(bytes, at, &[(4, &1u32.to_le_bytes())]);
    record(bytes, at + 0x20, &[(0, &1f32.to_le_bytes()), (4, &48000u32.to_le_bytes()), (8, &2u32.to_le_bytes()), (0xc, &[1]), (0x924, &i32::MAX.to_le_bytes()), (0x928, &u32::MAX.to_le_bytes())]); at += sizes[5];
    record(bytes, at, &[(0, &[1,1]), (0x120, &2u32.to_le_bytes()), (0x124, &[0,1])]);
    if let Some(address) = ring { record(bytes, at + 0x140, &[(0, &[2,1]), (0x20, &address.to_le_bytes()), (0x28, &960u32.to_le_bytes()), (0x2c, &2u32.to_le_bytes()), (0x30, &1u32.to_le_bytes()), (0x3c, &[0,1])]); }
    len
}
const KICK: usize = 0;
const HAT: usize = 1;
const BASS: usize = 2;
const PLUCK: usize = 3;
/// Periods per sixteenth note: 256 steps make the 20.48 s loop.
const STEP: u64 = 16;
// Chord tones in semitones from A, two bars each: Am, F, C, G.
const CHORDS: [[i32; 4]; 4] = [[0, 3, 7, 12], [-4, 0, 3, 8], [3, 7, 10, 15], [-2, 2, 5, 10]];
const ARPEGGIO: [usize; 16] = [0, 1, 2, 3, 2, 1, 0, 1, 0, 1, 2, 3, 2, 3, 2, 1];
const BASS_STEPS: u16 = 0b0100_1001_0100_1001;

/// Starts this period's notes from the pattern table: the arpeggio throughout, bass from bar 3, drums from bar 5.
pub fn sequence(periods: u64, semitones: impl Fn(i32) -> f32, mut trigger: impl FnMut(usize, usize, f32, f32, f32)) {
    if periods % STEP != 0 {
        return;
    }
    let step = (periods / STEP) as usize % 256;
    let (bar, beat) = (step / 16, step % 16);
    let chord = CHORDS[bar / 2 % 4];
    let side = if beat % 2 == 0 { -0.5 } else { 0.5 };
    trigger(3 + beat % 5, PLUCK, semitones(chord[ARPEGGIO[beat]]), 0.22, side);
    if bar >= 2 && BASS_STEPS >> beat & 1 != 0 {
        trigger(2, BASS, semitones(chord[0] - if chord[0] > 0 { 12 } else { 0 }), 0.35, 0.0);
    }
    if bar >= 4 && beat % 4 == 0 {
        trigger(0, KICK, 1.0, 0.5, 0.0);
    }
    if bar >= 4 && beat % 2 == 0 && beat % 4 != 0 {
        trigger(1, HAT, 1.0, 0.3, 0.3);
    }
}
#[cfg(feature = "std")]
pub mod assets {
use std::{vec, vec::Vec, f32::consts::{TAU, FRAC_1_SQRT_2}};
pub fn pcm(samples: impl Iterator<Item = f32>) -> Vec<i16> {
    samples.map(|s| (s.clamp(-1.0, 1.0) * 32767.0) as i16).collect()
}

/// Kick, hat, bass and a Karplus-Strong pluck at A3, from a fixed noise seed.
pub fn instruments() -> Vec<Vec<i16>> {
    let mut seed = 0x1234_5678u32;
    let mut noise = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed as i32 as f32 / 2_147_483_648.0
    };
    let time = |frames: usize| (0..frames).map(|n| n as f32 / 48_000.0);
    let mut phase = 0.0;
    let kick = pcm(time(19_200).map(|t| {
        phase += TAU * (45.0 + 110.0 * (-25.0 * t).exp()) / 48_000.0;
        phase.sin() * (-6.0 * t).exp()
    }));
    let mut last = 0.0;
    let hat = pcm(time(4_000).map(|t| {
        let n = noise();
        let high = n - last;
        last = n;
        0.4 * high * (-60.0 * t).exp()
    }));
    let bass = pcm(time(48_000).map(|t| (0.8 * (TAU * 110.0 * t).sin() + 0.25 * (TAU * 220.0 * t).sin()) * (-3.0 * t).exp()));
    let mut line: Vec<f32> = (0..218).map(|_| noise()).collect();
    let pluck = pcm((0..72_000).map(|n| {
        let (i, j) = (n % line.len(), (n + 1) % line.len());
        let sample = line[i];
        line[i] = 0.498 * (sample + line[j]);
        0.6 * sample
    }));
    vec![kick, hat, bass, pluck]
}


pub fn biquad() -> [u8; 10] {
    let omega = TAU * 7000.0 / 48000.0;
    let (alpha, cos) = (omega.sin() * FRAC_1_SQRT_2, omega.cos());
    let b = [(1.0-cos)*0.5, 1.0-cos, (1.0-cos)*0.5, 2.0*cos, alpha-1.0].map(|c| ((c/(1.0+alpha)*16384.0) as i16).to_le_bytes());
    core::array::from_fn(|i| b[i/2][i%2])
}
pub fn place(samples: &[Vec<i16>], base: u64) -> (Vec<u8>, Vec<(u64,u32)>) {
    let (mut bytes, mut placed) = (Vec::new(), Vec::new());
    for sample in samples {
        placed.push((base + bytes.len() as u64, sample.len() as u32));
        bytes.extend(sample.iter().flat_map(|s| s.to_le_bytes()));
        bytes.resize(bytes.len().next_multiple_of(0x1000), 0);
    }
    (bytes, placed)
}
}
