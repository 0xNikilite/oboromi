//! Procedural dungeon sources are spatialized on the host in renderer-sized blocks, then the stereo pair is submitted
//! as ordinary PCM voices through audren.
use super::demo_dungeon::{Rng, Sfx, V3, World};
use crate::audio::HostMemory;
use crate::audio::parcel::put;
use crate::audio::renderer::{AudioRendererParameter, Renderer, work_buffer_size};
use crate::audio::spatial::{BLOCK, Mix, Mode, Source, Spatializer};
use std::f32::consts::{PI, TAU};
use std::sync::atomic::{AtomicU8, Ordering};

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
    GravelStep,
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
    let mut high = Biquad::new(b'h', 150.0, 0.7);
    let mut low = Biquad::new(b'l', 1800.0, 0.7);
    let mut out = synth(seconds, &mut |t, r| {
        let swell = 0.6 + 0.4 * (t * TAU / seconds * 3.0).sin() * (t * TAU / seconds * 7.0).cos();
        let n = r.next() * 2.0 - 1.0;
        roar * swell * low.run(high.run(n)) * 2.5
    });
    let mut events = vec![0.0; out.len()];
    bursts(seconds, pops.clamp(4.0, 12.0), seed, &mut events, &|age, strength, r| (r.next() * 2.0 - 1.0) * strength * (-age / 0.006).exp(), 0.04);
    let mut pop = Biquad::new(b'b', 3400.0, 0.8);
    for (sample, event) in out.iter_mut().zip(events) { *sample += pop.run(event) * 1.4; }
    bursts(seconds, 0.3, seed + 7, &mut out, &|age, strength, r| (r.next() * 2.0 - 1.0) * strength * (age / 0.08).min(1.0) * (-age / 0.18).exp() * 0.4, 0.5);
    out
}

fn step(wet: bool, gravel: bool, variation: usize) -> Vec<f32> {
    let mut r = Rng(0x51ed_0001 + variation as u32 * 977 + u32::from(wet) * 31 + u32::from(gravel) * 67);
    let [mut body, mut grit, mut splash] = [Biquad::new(b'l', 650.0, 0.7), Biquad::new(b'b', if gravel { 2400.0 } else { 1500.0 }, 0.8), Biquad::new(b'b', 3200.0, 0.7)];
    let [mut ceiling_a, mut ceiling_b] = [Biquad::new(b'l', 4200.0, 0.7), Biquad::new(b'l', 4200.0, 0.7)];
    (0..(0.24 * RATE) as usize).map(|n| {
        let t = n as f32 / RATE;
        let noise = r.next() * 2.0 - 1.0;
        let heel = (t * TAU * (105.0 - t * 180.0)).sin() * (-t / (0.025 + variation as f32 * 0.001)).exp();
        let toe_age = t - 0.055 - variation as f32 * 0.0015;
        let toe = if toe_age > 0.0 { (toe_age * TAU * 145.0).sin() * (-toe_age / 0.035).exp() * 0.55 } else { 0.0 };
        let scrape = grit.run(noise) * (-t / if gravel { 0.055 } else { 0.03 }).exp() * if gravel { 0.9 } else { 0.5 };
        let water = if wet { splash.run(noise) * (-(t - 0.055).abs() / 0.05).exp() * 0.5 } else { 0.0 };
        ceiling_b.run(ceiling_a.run(body.run(heel + toe) + scrape + water))
    }).collect()
}

fn sample(kind: Sample) -> Vec<f32> {
    match kind {
        Sample::Torch => crackle(4.0, 0.12, 8.0, 11),
        Sample::Fire => {
            let mut out = crackle(6.0, 0.45, 11.0, 12);
            let mut rumble = Biquad::new(b'b', 80.0, 0.8);
            let mut r = Rng(81);
            for sample in &mut out { *sample += rumble.run(r.next() * 2.0 - 1.0) * 0.75; }
            out
        }
        Sample::River => {
            let [mut rumble, mut low, mut mid, mut top] = [Biquad::new(b'b', 95.0, 0.6), Biquad::new(b'l', 500.0, 0.6), Biquad::new(b'b', 1100.0, 0.5), Biquad::new(b'b', 4200.0, 0.8)];
            let mut out = synth(5.0, &mut |t, r| {
                let n = r.next() * 2.0 - 1.0;
                let surge = 0.8 + 0.2 * (t * TAU * 0.4).sin();
                surge * (2.2 * rumble.run(n) + 1.3 * low.run(n) + 0.45 * mid.run(n)) + 0.06 * top.run(n)
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
        Sample::Drip => synth(0.22, &mut |t, r| {
            let ping = ((t * TAU * 3100.0).sin() + 0.55 * (t * TAU * 3370.0).sin()) * (-t / 0.055).exp();
            ping * 0.65 + (r.next() - 0.5) * (-t / 0.018).exp() * 0.25
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
        Sample::Step | Sample::WetStep | Sample::GravelStep => step(kind == Sample::WetStep, kind == Sample::GravelStep, 0),
    }
}

const SAMPLES: [Sample; 12] = [Sample::Torch, Sample::Fire, Sample::River, Sample::Wind, Sample::Heart, Sample::Creak, Sample::Bats, Sample::Drip, Sample::Scream, Sample::Step, Sample::WetStep, Sample::GravelStep];

fn looping(kind: Sample) -> bool {
    !matches!(kind, Sample::Drip | Sample::Scream | Sample::Step | Sample::WetStep | Sample::GravelStep)
}

struct Voice {
    sample: Sample,
    pos: V3,
    gain: f32,
    pitch: f32,
    cursor: f32,
    active: bool,
    tail: usize,
    variation: usize,
}

pub const DRIPS: usize = 8;

pub struct Sound {
    renderer: Renderer,
    memory: HostMemory,
    samples: Vec<Vec<f32>>,
    steps: Vec<Vec<f32>>,
    voices: Vec<Voice>,
    spatial: Spatializer,
    configured: bool,
    next_drip: usize,
    next_step: usize,
    output: Vec<u8>,
}

static MODE: AtomicU8 = AtomicU8::new(0);

pub fn set_mode(mode: Mode) {
    MODE.store(u8::from(mode == Mode::Speakers), Ordering::Relaxed);
}

impl Sound {
    /// The renderer with one looping voice per source, then the drip, scream and footstep voices.
    pub fn new(world: &World, sources: &[(Sfx, V3, f32)]) -> Sound {
        let samples = SAMPLES.map(sample).into();
        let steps = (0..24).map(|i| step(i / 8 == 1, i / 8 == 2, i % 8)).collect();
        let mut voices: Vec<Voice> = sources.iter().enumerate().map(|(i, &(sfx, pos, gain))| Voice { sample: sfx.into(), pos, gain: gain * 2.0, pitch: 0.92 + 0.16 * ((i * 7) % 11) as f32 / 10.0, cursor: 0.0, active: true, tail: usize::MAX, variation: 0 }).collect();
        for sample in [[Sample::Drip; DRIPS].as_slice(), &[Sample::Scream, Sample::Step, Sample::Step]].concat() {
            voices.push(Voice { sample, pos: [0.0; 3], gain: 0.0, pitch: 1.0, cursor: 0.0, active: false, tail: 0, variation: 0 });
        }
        let parameter = AudioRendererParameter { sample_rate: 48_000, sample_count: BLOCK as u32, mix_buffer_count: 2, voice_count: 2, sink_count: 1, revision: MAGIC, ..Default::default() };
        let mut work = vec![0; work_buffer_size(&parameter).unwrap() as usize];
        let mut renderer = Renderer::open(parameter, &mut work).unwrap();
        renderer.start();
        Sound { renderer, memory: HostMemory { base: BASE, bytes: vec![0; 0x1000] }, samples, steps, voices, spatial: Spatializer::new(&world.acoustic_rooms(), Mode::Headphones), configured: false, next_drip: 0, next_step: 0, output: vec![0; 0x2_0000] }
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
                self.next_step += 1;
                first + DRIPS + 1 + (self.next_step & 1)
            }
        };
        let variation = self.next_step * 5 % 8;
        let semitones = [-2.0, 1.0, -1.0, 2.0, 0.0, -1.5, 1.5, 0.5][variation];
        let decibels = [-3.0, 2.0, -1.0, 3.0, 0.0, 1.0, -2.0, 0.5][variation];
        let step = matches!(sample, Sample::Step | Sample::WetStep | Sample::GravelStep);
        self.voices[slot] = Voice { sample, pos, gain: gain * if step { 10.0f32.powf(decibels / 20.0) } else { 1.0 }, pitch: pitch * if step { 2.0f32.powf(semitones / 12.0) } else { 1.0 }, cursor: 0.0, active: true, tail: 800, variation };
    }

    pub fn listen(&mut self, _world: &World, eye: V3, yaw: f32, _dt: f32) {
        self.spatial.set_mode(if MODE.load(Ordering::Relaxed) == 0 { Mode::Headphones } else { Mode::Speakers });
        self.spatial.set_listener(eye, [yaw.sin(), 0.0, -yaw.cos()], [0.0, 1.0, 0.0]);
    }

    /// Sends one update and renders `periods` periods of interleaved stereo.
    pub fn render(&mut self, periods: usize) -> Vec<i16> {
        if !self.configured {
            let parcel = self.parcel();
            self.renderer.request_update(&parcel, &mut self.output).unwrap();
            self.configured = true;
        }
        let mut output = Vec::with_capacity(periods * BLOCK * 2);
        for _ in 0..periods {
            let mut mix = Mix::new();
            for (id, voice) in self.voices.iter_mut().enumerate().filter(|(_, voice)| voice.active || voice.tail > 0) {
                let pcm = if matches!(voice.sample, Sample::Step | Sample::WetStep | Sample::GravelStep) { &self.steps[(voice.sample as usize - Sample::Step as usize) * 8 + voice.variation] } else { &self.samples[voice.sample as usize] };
                let mut mono = [0.0; BLOCK];
                for sample in &mut mono {
                    if !voice.active { break; }
                    let at = voice.cursor as usize;
                    if at >= pcm.len() {
                        if looping(voice.sample) {
                            voice.cursor %= pcm.len() as f32;
                        } else {
                            voice.active = false;
                            break;
                        }
                    }
                    let at = voice.cursor as usize;
                    let next = (at + 1).min(pcm.len() - 1);
                    *sample = pcm[at] + (pcm[next] - pcm[at]) * voice.cursor.fract();
                    voice.cursor += voice.pitch;
                }
                if !voice.active { voice.tail = voice.tail.saturating_sub(1); }
                let mut stereo = [[0.0; 2]; BLOCK];
                self.spatial.render(&Source { id, position: voice.pos, gain: voice.gain }, &mono, &mut stereo);
                mix.add(&stereo);
            }
            for (frame, samples) in mix.finish().iter().enumerate() {
                for channel in 0..2 {
                    let at = channel * 0x400 + frame * 2;
                    self.memory.bytes[at..at + 2].copy_from_slice(&((samples[channel] * 32767.0) as i16).to_le_bytes());
                }
            }
            output.extend(self.renderer.step(&mut self.memory));
        }
        output
    }

    fn parcel(&self) -> Vec<u8> {
        let record = |len: usize, fields: &[(usize, &[u8])]| {
            let mut r = vec![0; len];
            fields.iter().for_each(|&(at, value)| put(&mut r, at, value));
            r
        };
        let mut pools = vec![0; 0x20 * 8];
        put(&mut pools, 0, &BASE.to_le_bytes());
        put(&mut pools, 8, &(self.memory.bytes.len() as u64).to_le_bytes());
        put(&mut pools, 0x10, &4u32.to_le_bytes());
        let channels = [record(0x70, &[(0, &0u32.to_le_bytes()), (4, &1f32.to_le_bytes())]), record(0x70, &[(0, &1u32.to_le_bytes()), (8, &1f32.to_le_bytes())])].concat();
        let mut voice_section = Vec::new();
        for i in 0..2 {
            let address = BASE + i as u64 * 0x400;
            let wave = record(0x38, &[(0, &address.to_le_bytes()), (8, &((BLOCK * 2) as u64).to_le_bytes()), (0x14, &(BLOCK as u32).to_le_bytes()), (0x18, &[1, 0, 0]), (0x1C, &(-1i32).to_le_bytes())]);
            voice_section.extend(record(0x170, &[(8, &[1, 1, 0, 2]), (0xC, &48_000u32.to_le_bytes()), (0x18, &1u32.to_le_bytes()), (0x1C, &1f32.to_le_bytes()), (0x20, &1f32.to_le_bytes()), (0x58, &0i32.to_le_bytes()), (0x5C, &u32::MAX.to_le_bytes()), (0x60, &wave), (0x140, &(i as i32).to_le_bytes()), (0x15C, &2u16.to_le_bytes())]));
        }
        let mixes = [record(0x20, &[(4, &1u32.to_le_bytes())]), record(0x930, &[(0, &1f32.to_le_bytes()), (4, &48_000u32.to_le_bytes()), (8, &2u32.to_le_bytes()), (0xC, &[1]), (0x924, &i32::MAX.to_le_bytes()), (0x928, &u32::MAX.to_le_bytes())])].concat();
        let sink = record(0x140, &[(0, &[1, 1]), (0x120, &2u32.to_le_bytes()), (0x124, &[0, 1])]);
        let sections = [record(0x10, &[(0, &MAGIC.to_le_bytes())]), pools, channels, voice_section, Vec::new(), mixes, sink, vec![0; 0x10]];
        let mut header = vec![0; 0x40];
        put(&mut header, 0, &MAGIC.to_le_bytes());
        for (slot, section) in [(0x4, 0), (0x8, 1), (0x10, 2), (0xC, 3), (0x14, 4), (0x18, 5), (0x1C, 6), (0x20, 7)] {
            put(&mut header, slot, &(sections[section].len() as u32).to_le_bytes());
        }
        let total = 0x40 + sections.iter().map(Vec::len).sum::<usize>();
        put(&mut header, 0x3C, &(total as u32).to_le_bytes());
        [header, sections.concat()].concat()
    }
}
