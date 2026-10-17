//! The dungeon's sound, played the way a game drives audren: every source is a voice on a looping or one-shot
//! procedural sample. The host only computes each voice's distance gain, stereo pan and wall count; the renderer does
//! the rest: a low-pass biquad per voice for the walls, a splitter sending the dry signal to the final mix and reverb
//! sends to one submix per zone, each with a reverb sized to its zone, faded towards the zone the listener is in.
use super::demo_dungeon::{Kind, Rng, Sfx, V3, World, dot, length, sub, unit};
use crate::audio::HostMemory;
use crate::audio::parcel::put;
use crate::audio::renderer::{AudioRendererParameter, Renderer, work_buffer_size};
use std::f32::consts::{FRAC_PI_4, PI, TAU};

const RATE: f32 = 48_000.0;
const BASE: u64 = 0x10000;
const MAGIC: u32 = 0x56_4552 | (0x30 + 13) << 24;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Sample {
    Torch,
    Fire,
    River,
    Wind,
    Heart,
    Creak,
    Bats,
    Drip,
    Scream,
    Step,
    WetStep,
}

impl From<Sfx> for Sample {
    fn from(sfx: Sfx) -> Sample {
        match sfx {
            Sfx::Torch => Sample::Torch,
            Sfx::Fire => Sample::Fire,
            Sfx::River => Sample::River,
            Sfx::Wind => Sample::Wind,
            Sfx::Heart => Sample::Heart,
            Sfx::Creak => Sample::Creak,
            Sfx::Bats => Sample::Bats,
        }
    }
}

struct Biquad([f32; 5], [f32; 4]);

impl Biquad {
    fn new(kind: u8, frequency: f32, q: f32) -> Biquad {
        let w = TAU * frequency / RATE;
        let (sin, cos) = w.sin_cos();
        let alpha = sin / (2.0 * q);
        let a0 = 1.0 + alpha;
        let b = match kind {
            b'l' => [(1.0 - cos) / 2.0, 1.0 - cos, (1.0 - cos) / 2.0],
            b'h' => [(1.0 + cos) / 2.0, -(1.0 + cos), (1.0 + cos) / 2.0],
            _ => [alpha, 0.0, -alpha],
        };
        Biquad([b[0] / a0, b[1] / a0, b[2] / a0, -2.0 * cos / a0, (1.0 - alpha) / a0], [0.0; 4])
    }
    fn run(&mut self, x: f32) -> f32 {
        let [b0, b1, b2, a1, a2] = self.0;
        let [x1, x2, y1, y2] = self.1;
        let y = b0 * x + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
        self.1 = [x, x1, y, y1];
        y
    }
}

/// `seconds` of mono audio from `f`(time, noise source).
fn synth(seconds: f32, f: &mut dyn FnMut(f32, &mut Rng) -> f32) -> Vec<f32> {
    let mut r = Rng(0x9e37_79b9);
    (0..(seconds * RATE) as usize).map(|n| f(n as f32 / RATE, &mut r)).collect()
}

/// Short decaying bursts at random times: `rate` per second, each `shape`(age, strength).
fn bursts(seconds: f32, rate: f32, seed: u32, out: &mut [f32], shape: &dyn Fn(f32, f32, &mut Rng) -> f32, length: f32) {
    let mut r = Rng(seed);
    let mut at = 0.0;
    loop {
        at += -(1.0 - r.next()).ln() / rate;
        if at >= seconds {
            break;
        }
        let strength = r.next().powi(2);
        let start = (at * RATE) as usize;
        let seed = (r.next() * 1e6) as u32;
        let mut rr = Rng(seed | 1);
        for i in 0..(length * RATE) as usize {
            let n = (start + i) % out.len();
            out[n] += shape(i as f32 / RATE, strength, &mut rr);
        }
    }
}

fn crackle(seconds: f32, roar: f32, pops: f32, seed: u32) -> Vec<f32> {
    let mut low = Biquad::new(b'l', 380.0, 0.7);
    let mut hiss = Biquad::new(b'b', 3000.0, 0.6);
    let mut out = synth(seconds, &mut |t, r| {
        let swell = 0.6 + 0.4 * (t * TAU / seconds * 3.0).sin() * (t * TAU / seconds * 7.0).cos();
        let n = r.next() * 2.0 - 1.0;
        roar * swell * low.run(n) * 2.5 + 0.03 * hiss.run(n)
    });
    let mut click = Biquad::new(b'b', 2600.0, 1.2);
    bursts(seconds, pops, seed, &mut out, &|age, strength, r| (r.next() * 2.0 - 1.0) * strength * (-age / 0.004).exp() * 0.9, 0.03);
    bursts(seconds, pops * 0.15, seed + 7, &mut out, &|age, strength, _| (age * TAU * 90.0).sin() * strength * (-age / 0.03).exp() * 0.5, 0.15);
    out.iter_mut().for_each(|s| *s = 0.6 * *s + 0.4 * click.run(*s));
    out
}

fn sample(kind: Sample) -> Vec<f32> {
    match kind {
        Sample::Torch => crackle(4.0, 0.12, 14.0, 11),
        Sample::Fire => crackle(6.0, 0.45, 40.0, 12),
        Sample::River => {
            let [mut low, mut mid, mut top] = [Biquad::new(b'l', 500.0, 0.6), Biquad::new(b'b', 1100.0, 0.5), Biquad::new(b'b', 4200.0, 0.8)];
            let mut out = synth(5.0, &mut |t, r| {
                let n = r.next() * 2.0 - 1.0;
                let surge = 0.8 + 0.2 * (t * TAU * 0.4).sin();
                surge * (1.6 * low.run(n) + 0.5 * mid.run(n)) + 0.08 * top.run(n)
            });
            bursts(5.0, 30.0, 13, &mut out, &|age, strength, r| {
                let f = 350.0 + 700.0 * r.next() + age * 4000.0;
                (age * TAU * f).sin() * strength * (-age / 0.02).exp() * 0.12
            }, 0.06);
            out
        }
        Sample::Wind => {
            let mut band = Biquad::new(b'b', 500.0, 3.0);
            synth(8.0, &mut |t, r| {
                let centre = 420.0 + 260.0 * (t * TAU / 8.0).sin() + 90.0 * (t * TAU * 3.0 / 8.0).sin();
                if (t * RATE) as usize % 64 == 0 {
                    let state = band.1;
                    band = Biquad::new(b'b', centre, 4.0);
                    band.1 = state;
                }
                let swell = 0.55 + 0.45 * (t * TAU * 2.0 / 8.0).sin().powi(2);
                swell * band.run(r.next() * 2.0 - 1.0) * 1.6
            })
        }
        Sample::Heart => synth(1.05, &mut |t, _| {
            let beat = |t0: f32, f: f32, a: f32| {
                let age = t - t0;
                if age < 0.0 { 0.0 } else { (age * TAU * f * (1.0 - age)).sin() * a * (-age / 0.07).exp() * (age / 0.006).min(1.0) }
            };
            beat(0.0, 52.0, 1.0) + beat(0.27, 44.0, 0.7)
        }),
        Sample::Creak => {
            let mut out = vec![0.0; (8.0 * RATE) as usize];
            bursts(8.0, 0.5, 14, &mut out, &|age, strength, r| {
                let f = 38.0 + 30.0 * (age * 2.3).sin() + 8.0 * r.next();
                let pulse = ((age * f).fract() < 0.18) as u8 as f32;
                pulse * (0.4 + 0.6 * strength) * (age * PI / 1.1).sin().max(0.0) * 0.45
            }, 1.1);
            let mut body = Biquad::new(b'b', 700.0, 2.0);
            out.iter_mut().for_each(|s| *s = body.run(*s) * 3.0);
            out
        }
        Sample::Bats => {
            let mut out = vec![0.0; RATE as usize];
            let mut band = Biquad::new(b'b', 900.0, 0.8);
            for (n, s) in out.iter_mut().enumerate() {
                let t = n as f32 / RATE;
                let flap = ((t * 11.0).fract() * TAU).sin().max(0.0).powi(3);
                *s = band.run(((n as u32).wrapping_mul(2_654_435_761) >> 8) as f32 / 8_388_608.0 - 1.0) * flap * 1.2;
            }
            bursts(1.0, 5.0, 15, &mut out, &|age, strength, _| (age * TAU * (7000.0 - age * 30000.0)).sin() * strength * (-age / 0.008).exp() * 0.25, 0.04);
            out
        }
        Sample::Drip => synth(0.35, &mut |t, r| {
            let f = 900.0 + 900.0 * (t / 0.03).min(1.0);
            (t * TAU * f).sin() * (-t / 0.05).exp() * 0.8 + (r.next() - 0.5) * (-t / 0.004).exp() * 0.4
        }),
        Sample::Scream => {
            let mut formants = [Biquad::new(b'b', 900.0, 4.0), Biquad::new(b'b', 2400.0, 5.0)];
            let mut phase = 0.0;
            synth(2.4, &mut |t, r| {
                let glide = 620.0 + 520.0 * (t / 0.5).min(1.0) - 300.0 * ((t - 1.2).max(0.0) / 1.2);
                phase += (glide + 30.0 * (t * TAU * 6.5).sin()) / RATE;
                let voice = (phase.fract() * 2.0 - 1.0) * 0.6 + (r.next() - 0.5) * 0.35;
                let envelope = (t / 0.08).min(1.0) * (1.0 - t / 2.4).max(0.0).powf(0.7);
                (formants[0].run(voice) * 1.4 + formants[1].run(voice)) * envelope * 1.5
            })
        }
        Sample::Step | Sample::WetStep => {
            let wet = kind == Sample::WetStep;
            let [mut grit, mut splash] = [Biquad::new(b'b', 1800.0, 0.9), Biquad::new(b'h', 3500.0, 0.7)];
            synth(if wet { 0.32 } else { 0.2 }, &mut |t, r| {
                let n = r.next() * 2.0 - 1.0;
                let thud = (t * TAU * (95.0 - t * 200.0)).sin() * (-t / 0.03).exp() * 0.9;
                let scrape = grit.run(n) * (-t / 0.025).exp() * 1.2;
                let water = if wet { splash.run(n) * (-(t - 0.01).abs() / 0.06).exp() * (1.0 + (t * 230.0).sin()) * 0.7 } else { 0.0 };
                thud + scrape + water
            })
        }
    }
}

const SAMPLES: [Sample; 11] = [Sample::Torch, Sample::Fire, Sample::River, Sample::Wind, Sample::Heart, Sample::Creak, Sample::Bats, Sample::Drip, Sample::Scream, Sample::Step, Sample::WetStep];

fn looping(kind: Sample) -> bool {
    !matches!(kind, Sample::Drip | Sample::Scream | Sample::Step | Sample::WetStep)
}

struct Voice {
    sample: Sample,
    pos: V3,
    gain: f32,
    pitch: f32,
    restart: bool,
    started: bool,
    /// dry left and right, a send per zone, the low-pass cutoff.
    dry: [f32; 2],
    sends: Vec<f32>,
    cutoff: f32,
}

pub const DRIPS: usize = 8;

pub struct Sound {
    renderer: Renderer,
    memory: HostMemory,
    /// address and frame count of each sample.
    samples: Vec<(u64, u32)>,
    voices: Vec<Voice>,
    zones: Vec<(V3, Kind)>,
    weights: Vec<f32>,
    pools_sent: bool,
    next_drip: usize,
    next_step: usize,
    output: Vec<u8>,
}

impl Sound {
    /// The renderer with one looping voice per source, then the drip, scream and footstep voices.
    pub fn new(world: &World, sources: &[(Sfx, V3, f32)]) -> Sound {
        let mut bytes = Vec::new();
        let mut samples = Vec::new();
        for kind in SAMPLES {
            let pcm = sample(kind);
            samples.push((BASE + bytes.len() as u64, pcm.len() as u32));
            bytes.extend(pcm.iter().flat_map(|s| ((s.clamp(-1.0, 1.0) * 32000.0) as i16).to_le_bytes()));
            bytes.resize(bytes.len().next_multiple_of(0x100), 0);
        }
        bytes.resize(bytes.len().next_multiple_of(0x1000), 0);
        let zones: Vec<(V3, Kind)> = world.zones.iter().map(|z| (sub(z.space.hi, z.space.lo), z.kind)).collect();
        let mut voices: Vec<Voice> = sources.iter().enumerate().map(|(i, &(sfx, pos, gain))| Voice {
            sample: sfx.into(),
            pos,
            gain,
            pitch: 0.92 + 0.16 * ((i * 7) % 11) as f32 / 10.0,
            restart: true,
            started: false,
            dry: [0.0; 2],
            sends: vec![0.0; zones.len()],
            cutoff: 16000.0,
        }).collect();
        for sample in [[Sample::Drip; DRIPS].as_slice(), &[Sample::Scream, Sample::Step, Sample::Step]].concat() {
            voices.push(Voice { sample, pos: [0.0; 3], gain: 0.0, pitch: 1.0, restart: false, started: false, dry: [0.0; 2], sends: vec![0.0; zones.len()], cutoff: 16000.0 });
        }
        let count = voices.len() as u32;
        let parameter = AudioRendererParameter {
            sample_rate: 48_000,
            sample_count: 240,
            mix_buffer_count: 2 + 2 * zones.len() as u32,
            sub_mix_count: zones.len() as u32,
            voice_count: count,
            sink_count: 1,
            effect_count: zones.len() as u32,
            splitter_count: count,
            splitter_destination_count: count * (1 + zones.len() as u32),
            revision: MAGIC,
            ..Default::default()
        };
        let mut work = vec![0; work_buffer_size(&parameter).unwrap() as usize];
        let mut renderer = Renderer::open(parameter, &mut work).unwrap();
        renderer.start();
        let weights = vec![0.0; zones.len()];
        Sound { renderer, memory: HostMemory { base: BASE, bytes }, samples, voices, zones, weights, pools_sent: false, next_drip: 0, next_step: 0, output: vec![0; 0x2_0000] }
    }

    pub fn move_source(&mut self, i: usize, pos: V3) {
        self.voices[i].pos = pos;
    }

    /// Plays a drip ping, the scream or a footstep at `pos`.
    pub fn play(&mut self, sample: Sample, pos: V3, gain: f32, pitch: f32) {
        let first = self.voices.len() - DRIPS - 3;
        let slot = match sample {
            Sample::Drip => {
                self.next_drip = (self.next_drip + 1) % DRIPS;
                first + self.next_drip
            }
            Sample::Scream => first + DRIPS,
            _ => {
                self.next_step ^= 1;
                first + DRIPS + 1 + self.next_step
            }
        };
        self.voices[slot] = Voice { sample, pos, gain, pitch, restart: true, started: true, dry: [0.0; 2], sends: vec![0.0; self.zones.len()], cutoff: 16000.0 };
    }

    /// Sets every voice's pan, distance and wall loss and the reverb sends for a listener at `eye` facing `yaw`,
    /// fading the listener's zone in over a third of a second.
    pub fn listen(&mut self, world: &World, eye: V3, yaw: f32, dt: f32) {
        let here: Vec<usize> = world.zones_at(eye).collect();
        for (z, w) in self.weights.iter_mut().enumerate() {
            let target = if here.contains(&z) { 1.0 / here.len() as f32 } else { 0.0 };
            *w += (target - *w) * (dt / 0.33).min(1.0);
        }
        let (forward, right) = ([yaw.sin(), 0.0, -yaw.cos()], [yaw.cos(), 0.0, yaw.sin()]);
        for v in &mut self.voices {
            let to = sub(v.pos, eye);
            let distance = length(to);
            let azimuth = if distance < 0.3 { 0.0 } else { dot(unit(to), right).atan2(dot(unit(to), forward)) };
            let walls = world.walls_between(eye, v.pos);
            let occlusion = 0.32f32.powi(walls as i32);
            let gain = v.gain * (1.2 / distance.max(1.2)) * occlusion * (0.8 + 0.2 * azimuth.cos());
            let pan = (azimuth.sin() + 1.0) * FRAC_PI_4;
            v.dry = [gain * pan.cos(), gain * pan.sin()];
            v.cutoff = [16000.0, 1600.0, 650.0, 320.0][walls.min(3) as usize] * (0.75 + 0.25 * azimuth.cos());
            let source: Vec<usize> = world.zones_at(v.pos).collect();
            let wet = v.gain * (1.2 / distance.max(1.2)).sqrt() * occlusion.sqrt();
            for (z, send) in v.sends.iter_mut().enumerate() {
                *send = 0.35 * wet * f32::from(source.contains(&z)) + 0.3 * gain * self.weights[z];
            }
        }
    }

    /// Sends one update and renders `periods` periods of interleaved stereo.
    pub fn render(&mut self, periods: usize) -> Vec<i16> {
        let parcel = self.parcel();
        self.renderer.request_update(&parcel, &mut self.output).unwrap();
        (0..periods).flat_map(|_| self.renderer.step(&mut self.memory).to_vec()).collect()
    }

    fn parcel(&mut self) -> Vec<u8> {
        let (voices, zones) = (self.voices.len(), self.zones.len());
        let record = |len: usize, fields: &[(usize, &[u8])]| {
            let mut r = vec![0; len];
            fields.iter().for_each(|&(at, value)| put(&mut r, at, value));
            r
        };
        let mut pools = vec![0; 0x20 * (zones + voices * 4)];
        if !std::mem::replace(&mut self.pools_sent, true) {
            put(&mut pools, 0, &BASE.to_le_bytes());
            put(&mut pools, 8, &(self.memory.bytes.len() as u64).to_le_bytes());
            put(&mut pools, 0x10, &4u32.to_le_bytes());
        }
        let channels: Vec<u8> = (0..voices).flat_map(|i| record(0x70, &[(0, &(i as u32).to_le_bytes()), (0x64, &[1])])).collect();
        let mut voice_section = Vec::new();
        for (i, v) in self.voices.iter_mut().enumerate() {
            let (address, frames) = self.samples[v.sample as usize];
            let fresh = std::mem::take(&mut v.restart);
            let looped = looping(v.sample);
            let wave = record(0x38, &[(0, &address.to_le_bytes()), (8, &(u64::from(frames) * 2).to_le_bytes()), (0x14, &frames.to_le_bytes()), (0x18, &[u8::from(looped), u8::from(!looped), u8::from(!fresh)]), (0x1C, &(if looped { -1i32 } else { 0 }).to_le_bytes())]);
            let w = TAU * v.cutoff.min(18000.0) / RATE;
            let (sin, cos) = w.sin_cos();
            let alpha = sin / 2.0f32.sqrt();
            let a0 = 1.0 + alpha;
            let q14 = |x: f32| ((x * 16384.0).round().clamp(-32768.0, 32767.0) as i16).to_le_bytes();
            let biquad = [&[1u8, 0][..], &q14((1.0 - cos) / 2.0 / a0), &q14((1.0 - cos) / a0), &q14((1.0 - cos) / 2.0 / a0), &q14(2.0 * cos / a0), &q14(-(1.0 - alpha) / a0)].concat();
            let playing = v.started || looped;
            voice_section.extend(record(0x170, &[
                (8, &[u8::from(fresh), 1, u8::from(!playing), 2]),
                (0xC, &48_000u32.to_le_bytes()),
                (0x18, &1u32.to_le_bytes()),
                (0x1C, &v.pitch.to_le_bytes()),
                (0x20, &1f32.to_le_bytes()),
                (0x24, &biquad),
                (0x58, &i32::MAX.to_le_bytes()),
                (0x5C, &(i as u32).to_le_bytes()),
                (0x60, &wave),
                (0x140, &(i as i32).to_le_bytes()),
            ]));
        }
        let q14 = |x: f32| ((x * 16384.0) as u32).to_le_bytes();
        let effects: Vec<u8> = self.zones.iter().enumerate().flat_map(|(z, &(size, kind))| {
            let volume = size[0] * size[1] * size[2];
            let decay = (0.35 + 0.13 * volume.cbrt()) * if kind == Kind::Cave { 1.9 } else { 1.0 };
            let predelay = (size[0].max(size[2]) / 343.0 * 1000.0).min(60.0);
            let [early, late] = if kind == Kind::Corridor { [0.9, 0.5] } else { [0.6, 0.8] };
            record(0xC0, &[
                (0, &[4, u8::from(!self.renderer_started()), 1]),
                (4, &(z as i32 + 1).to_le_bytes()),
                (0x20, &[0, 1, 0, 0, 0, 0, 0, 1]),
                (0x2C, &2u16.to_le_bytes()),
                (0x2E, &2u16.to_le_bytes()),
                (0x38, &q14(early)),
                (0x3C, &q14(predelay)),
                (0x44, &q14(late)),
                (0x48, &q14(decay)),
                (0x4C, &q14(0.55)),
                (0x50, &q14(0.6)),
                (0x54, &q14(1.0)),
                (0x58, &q14(0.7)),
                (0x5C, &q14(0.0)),
            ])
        }).collect();
        let per = 1 + zones;
        let mut splitters = record(0x20, &[(0, b"SNDH"), (4, &(voices as u32).to_le_bytes()), (8, &((voices * per) as u32).to_le_bytes())]);
        for i in 0..voices {
            splitters.extend(record(0x10, &[(0, b"SNDI"), (4, &(i as u32).to_le_bytes()), (8, &48_000u32.to_le_bytes()), (0xC, &(per as u32).to_le_bytes())]));
            splitters.extend((0..per).flat_map(|d| ((i * per + d) as u32).to_le_bytes()));
            splitters.extend([0; 0xC]);
        }
        for (i, v) in self.voices.iter().enumerate() {
            for d in 0..per {
                let volume = if d == 0 { v.dry } else { [v.sends[d - 1] * (0.5 + v.dry[0]), v.sends[d - 1] * (0.5 + v.dry[1])].map(|s| s.min(4.0)) };
                let [l, r] = volume.map(f32::to_le_bytes);
                splitters.extend(record(0x90, &[(0, b"SNDD"), (4, &((i * per + d) as u32).to_le_bytes()), (8, &l), (0xC, &r), (0x68, &(d as i32).to_le_bytes()), (0x84, &[1, u8::from(!self.renderer_started())])]));
            }
        }
        splitters.resize(splitters.len().next_multiple_of(0x10), 0);
        let mut mixes = record(0x20, &[(4, &(zones as u32 + 1).to_le_bytes())]);
        for m in 0..=zones {
            let volume = if m == 0 { 1.0 } else { 0.3 + 0.7 * self.weights[m - 1] };
            let destination = if m == 0 { i32::MAX } else { 0 };
            mixes.extend(record(0x930, &[
                (0, &volume.to_le_bytes()),
                (4, &48_000u32.to_le_bytes()),
                (8, &2u32.to_le_bytes()),
                (0xC, &[1]),
                (0x10, &(m as i32).to_le_bytes()),
                (0x24, &1f32.to_le_bytes()),
                (0x24 + 0x60 + 4, &1f32.to_le_bytes()),
                (0x924, &destination.to_le_bytes()),
                (0x928, &u32::MAX.to_le_bytes()),
            ]));
        }
        let sink = record(0x140, &[(0, &[1, 1]), (0x120, &2u32.to_le_bytes()), (0x124, &[0, 1])]);
        let sections = [record(0x10, &[(0, &MAGIC.to_le_bytes())]), pools, channels, voice_section, effects, splitters, mixes, sink, vec![0; 0x10]];
        let mut header = vec![0; 0x40];
        put(&mut header, 0, &MAGIC.to_le_bytes());
        for (slot, section) in [(0x4, 0), (0x8, 1), (0x10, 2), (0xC, 3), (0x14, 4), (0x18, 6), (0x1C, 7), (0x20, 8)] {
            put(&mut header, slot, &(sections[section].len() as u32).to_le_bytes());
        }
        let total = 0x40 + sections.iter().map(Vec::len).sum::<usize>();
        put(&mut header, 0x3C, &(total as u32).to_le_bytes());
        [header, sections.concat()].concat()
    }

    fn renderer_started(&self) -> bool {
        self.renderer.rendered_updates() > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_is_centred_and_walls_reduce_rms_and_high_band_energy() {
        let world = World::new(8);
        let source = world.zones[0].space;
        let pos = [(source.lo[0] + source.hi[0]) * 0.5, 1.65, (source.lo[2] + source.hi[2]) * 0.5];
        let measure = |eye| {
            let mut sound = Sound::new(&world, &[(Sfx::River, pos, 1.0)]);
            sound.listen(&world, eye, 0.0, 1.0);
            let pcm = sound.render(80);
            let channels: Vec<[f64; 2]> = pcm.chunks_exact(2).map(|p| [p[0] as f64, p[1] as f64]).collect();
            let energy = channels.iter().map(|p| p[0] * p[0] + p[1] * p[1]).sum::<f64>() / channels.len() as f64;
            let high = channels.windows(2).map(|p| (p[1][0] - p[0][0]).powi(2) + (p[1][1] - p[0][1]).powi(2)).sum::<f64>() / channels.len() as f64;
            let rms = [0, 1].map(|c| (channels.iter().map(|p| p[c] * p[c]).sum::<f64>() / channels.len() as f64).sqrt());
            let imbalance = (rms[0] - rms[1]).abs();
            (energy.sqrt(), high / energy, imbalance)
        };
        let near = measure(pos);
        let blocked_pos = [source.hi[0] + 1.0, pos[1], source.lo[2] + 0.5];
        assert!(world.walls_between(pos, blocked_pos) > 0);
        let blocked = measure(blocked_pos);
        eprintln!("dungeon audio: near rms={} high={} imbalance={}; wall rms={} high={}", near.0, near.1, near.2, blocked.0, blocked.1);
        assert!(near.0 > 100.0);
        assert!(near.2 < near.0 * 0.02);
        assert!(blocked.0 < near.0 * 0.5);
        assert!(blocked.1 < near.1 * 0.5);
    }
}
