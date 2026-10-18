//! Recorded dungeon sources are spatialized on the host in renderer-sized blocks, then the stereo pair is submitted
//! as ordinary PCM voices through audren.
use super::demo_dungeon::{Box3, Rng, Sfx, V3, World};
use crate::audio::HostMemory;
use crate::audio::parcel::put;
use crate::audio::renderer::{AudioRendererParameter, Renderer, work_buffer_size};
use crate::audio::spatial::{BLOCK, Mix, Mode, Source, Spatializer};
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

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
    Room,
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

fn recording(name: &str) -> Vec<f32> {
    std::fs::read(super::demo::asset_dir("dungeon-audio").join(format!("{name}.pcm")))
        .unwrap_or_else(|e| panic!("dungeon audio {name}: {e}; run demos/assets/fetch.sh"))
        .chunks_exact(2)
        .map(|bytes| i16::from_le_bytes(bytes.try_into().unwrap()) as f32 / 32768.0)
        .collect()
}

fn sample(kind: Sample) -> Vec<f32> {
    let name = match kind {
        Sample::Torch => "torch",
        Sample::Fire => "fire",
        Sample::River => "river",
        Sample::Wind => "wind",
        Sample::Heart => "heart",
        Sample::Creak => "creak",
        Sample::Bats | Sample::Scream => return vec![0.0],
        Sample::Room => "room",
        Sample::Drip => "drip0",
        Sample::Step | Sample::WetStep | Sample::GravelStep => "stone0",
    };
    recording(name)
}

const SAMPLES: [Sample; 13] = [Sample::Torch, Sample::Fire, Sample::River, Sample::Wind, Sample::Heart, Sample::Creak, Sample::Bats, Sample::Room, Sample::Drip, Sample::Scream, Sample::Step, Sample::WetStep, Sample::GravelStep];

fn looping(kind: Sample) -> bool {
    !matches!(kind, Sample::Creak | Sample::Drip | Sample::Scream | Sample::Step | Sample::WetStep | Sample::GravelStep)
}

fn drip_timing(rng: &mut Rng) -> (f32, u64) {
    (0.97 + rng.next() * 0.06, ((2.0 + rng.next() * 3.0) * RATE) as u64)
}

fn far_drip_delay(rng: &mut Rng) -> u64 {
    ((10.0 + rng.next() * 10.0) * RATE) as u64
}

#[allow(dead_code)]
pub fn step_cadence(walk_speed: f32) -> f32 {
    walk_speed.max(0.0) / 0.54
}

fn limit(sample: f32) -> f32 {
    let magnitude = sample.abs();
    sample.signum() * if magnitude <= 0.72 { magnitude } else { 0.72 + 0.12 * (1.0 - (-(magnitude - 0.72) / 0.12).exp()) }
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
    step_set: usize,
}

pub const DRIPS: usize = 4;

struct Engine {
    renderer: Renderer,
    memory: HostMemory,
    samples: Vec<Vec<f32>>,
    drips: Vec<Vec<f32>>,
    steps: Vec<Vec<f32>>,
    voices: Vec<Voice>,
    spatial: Spatializer,
    bridge: Box3,
    reveal: Box3,
    rng: Rng,
    drip_at: u64,
    next_drip: usize,
    in_reveal: bool,
    reveal_at: u64,
    last_eye: V3,
    mix_gain: f32,
    rendered: u64,
    creak: Option<usize>,
    creak_played: bool,
    ambient: bool,
    configured: bool,
    next_step: usize,
    output: Vec<u8>,
}

static MODE: AtomicU8 = AtomicU8::new(0);

pub fn set_mode(mode: Mode) {
    MODE.store(u8::from(mode == Mode::Speakers), Ordering::Relaxed);
}

impl Engine {
    /// The renderer with one looping voice per source, then the drip, scream and footstep voices.
    fn new(world: &World, sources: &[(Sfx, V3, f32)]) -> Engine {
        let samples = SAMPLES.map(sample).into();
        let drips = (0..3).map(|i| recording(&format!("drip{i}"))).collect();
        let steps = ["stone", "wet", "gravel", "wood"].into_iter().flat_map(|name| (0..6).map(move |i| recording(&format!("{name}{i}")))).collect();
        let ambient = sources == world.sources;
        let creak = sources.iter().position(|source| source.0 == Sfx::Creak);
        let mut voices: Vec<Voice> = sources.iter().enumerate().map(|(i, &(sfx, pos, gain))| {
            let active = sfx != Sfx::Creak && (!ambient || !matches!(sfx, Sfx::Torch | Sfx::Fire | Sfx::River));
            Voice { sample: sfx.into(), pos, gain: gain * if sfx == Sfx::Heart { 0.12 } else { 1.2 }, pitch: 0.99 + 0.02 * ((i * 7) % 11) as f32 / 10.0, cursor: 0.0, active, tail: if active { usize::MAX } else { 0 }, variation: 0, step_set: 0 }
        }).collect();
        if ambient {
            voices.extend(world.lights.iter().filter(|light| light.flame >= 0.3).enumerate().map(|(i, light)| Voice { sample: if light.flame > 1.0 { Sample::Fire } else { Sample::Torch }, pos: light.pos, gain: if light.flame > 1.0 { 2.64 } else { 0.66 }, pitch: 0.99 + 0.02 * ((i * 7) % 11) as f32 / 10.0, cursor: 0.0, active: true, tail: usize::MAX, variation: 0, step_set: 0 }));
            voices.extend((0..4).map(|i| Voice { sample: Sample::River, pos: [world.river.lo[0] + (i as f32 + 0.5) / 4.0 * (world.river.hi[0] - world.river.lo[0]), world.river.hi[1], (world.river.lo[2] + world.river.hi[2]) * 0.5], gain: 1.56, pitch: 0.99 + i as f32 * 0.006, cursor: 0.0, active: true, tail: usize::MAX, variation: 0, step_set: 0 }));
        }
        if ambient { for (i, room) in world.acoustic_rooms().into_iter().enumerate() {
            voices.push(Voice { sample: Sample::Room, pos: std::array::from_fn(|axis| (room.min[axis] + room.max[axis]) * 0.5), gain: if i == world.cave { 0.28 } else { 0.08 }, pitch: 1.0, cursor: 0.0, active: true, tail: usize::MAX, variation: 0, step_set: 0 });
        } }
        let cave = world.zones[world.cave].space;
        let centre: V3 = std::array::from_fn(|i| (cave.lo[i] + cave.hi[i]) * 0.5);
        let mut positions: Vec<_> = world.drips.iter().map(|&(tip, floor)| [tip[0], floor, tip[2]]).collect();
        positions.sort_by(|a, b| super::demo_dungeon::length(super::demo_dungeon::sub(*a, centre)).total_cmp(&super::demo_dungeon::length(super::demo_dungeon::sub(*b, centre))));
        for (i, sample) in [[Sample::Drip; DRIPS].as_slice(), &[Sample::Scream, Sample::Step, Sample::Step]].concat().into_iter().enumerate() {
            let position = positions.get(i).copied().unwrap_or(centre);
            voices.push(Voice { sample, pos: position, gain: 0.0, pitch: 1.0, cursor: 0.0, active: false, tail: 0, variation: 0, step_set: 0 });
        }
        let parameter = AudioRendererParameter { sample_rate: 48_000, sample_count: BLOCK as u32, mix_buffer_count: 2, voice_count: 2, sink_count: 1, revision: MAGIC, ..Default::default() };
        let mut work = vec![0; work_buffer_size(&parameter).unwrap() as usize];
        let mut renderer = Renderer::open(parameter, &mut work).unwrap();
        renderer.start();
        Engine { renderer, memory: HostMemory { base: BASE, bytes: vec![0; 0x1000] }, samples, drips, steps, voices, spatial: Spatializer::new(&world.acoustic_rooms(), Mode::Headphones), bridge: world.bridge, reveal: world.zones[world.cave].space, rng: Rng(0xd21f_740b), drip_at: (12.0 * RATE) as u64, next_drip: 0, in_reveal: false, reveal_at: 0, last_eye: [0.0; 3], mix_gain: 0.46, rendered: 0, creak, creak_played: false, ambient, configured: false, next_step: 0, output: vec![0; 0x2_0000] }
    }

    pub fn move_source(&mut self, i: usize, pos: V3) {
        self.voices[i].pos = pos;
    }

    /// Plays a drip ping, the scream or a footstep at `pos`.
    pub fn play(&mut self, sample: Sample, pos: V3, gain: f32, pitch: f32) {
        let first = self.voices.len() - DRIPS - 3;
        if sample == Sample::Scream || self.ambient && sample == Sample::Drip { return; }
        let slot = match sample {
            Sample::Drip => {
                if self.rendered < self.drip_at { return; }
                let slot = first + self.next_drip;
                self.next_drip = (self.next_drip + 1) % DRIPS;
                self.drip_at = self.rendered + drip_timing(&mut self.rng).1;
                slot
            }
            _ => {
                self.next_step += 1;
                first + DRIPS + 1 + (self.next_step & 1)
            }
        };
        let variation = self.next_step * 5 % 6;
        let decibels = [-2.0, 1.0, -1.0, 2.0, 0.0, 0.5][variation];
        let step = matches!(sample, Sample::Step | Sample::WetStep | Sample::GravelStep);
        let step_set = if self.bridge.contains(pos) { 3 } else { match sample { Sample::WetStep => 1, Sample::GravelStep => 2, _ => 0 } };
        let pos = if sample == Sample::Drip { self.voices[slot].pos } else { pos };
        self.voices[slot] = Voice { sample, pos, gain: gain * if step { 10.0f32.powf(decibels / 20.0) } else { 1.0 }, pitch: if step { 1.0 } else { pitch }, cursor: 0.0, active: true, tail: 800, variation, step_set };
    }

    fn listen(&mut self, eye: V3, yaw: f32, dt: f32) {
        let in_reveal = self.reveal.contains(eye);
        if in_reveal != self.in_reveal {
            self.in_reveal = in_reveal;
            if in_reveal { self.reveal_at = self.rendered; }
            self.drip_at = self.rendered + if in_reveal { (2.0 * RATE) as u64 } else { far_drip_delay(&mut self.rng) };
        }
        let reveal_age = (self.rendered - self.reveal_at) as f32 / RATE;
        let moving = dt > 0.0 && super::demo_dungeon::length(super::demo_dungeon::sub(eye, self.last_eye)) / dt > 0.1;
        let target = if !in_reveal { 0.46 } else if reveal_age < 6.0 { 0.46 + reveal_age / 6.0 * 0.54 } else if moving { 1.0 } else { 0.58 };
        self.mix_gain += (target - self.mix_gain) * (dt / 2.0).min(1.0);
        self.last_eye = eye;
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
            let first = self.voices.len() - DRIPS - 3;
            if self.ambient {
                if self.rendered >= self.drip_at {
                    let voice = &mut self.voices[first + self.next_drip];
                    voice.gain = if self.in_reveal { 0.34 } else { 0.055 };
                    let (pitch, delay) = drip_timing(&mut self.rng);
                    voice.pitch = pitch;
                    voice.cursor = 0.0;
                    voice.active = true;
                    voice.tail = 800;
                    voice.variation = self.next_drip % 3;
                    self.next_drip = (self.next_drip + 1) % DRIPS;
                    self.drip_at = self.rendered + if self.in_reveal { delay } else { far_drip_delay(&mut self.rng) };
                }
                if !self.creak_played && self.rendered >= (32.5 * RATE) as u64 {
                    if let Some(creak) = self.creak { self.voices[creak].active = true; }
                    self.creak_played = true;
                }
            }
            let mut mix = Mix::new();
            self.spatial.begin_block();
            for (id, voice) in self.voices.iter_mut().enumerate().filter(|(_, voice)| voice.active || voice.tail > 0) {
                let pcm = if voice.sample == Sample::Drip { &self.drips[voice.variation] } else if matches!(voice.sample, Sample::Step | Sample::WetStep | Sample::GravelStep) { &self.steps[voice.step_set * 6 + voice.variation] } else { &self.samples[voice.sample as usize] };
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
                let suspense = self.rendered >= (30.0 * RATE) as u64 && self.rendered < (36.0 * RATE) as u64;
                let gain = if voice.sample == Sample::Heart && !suspense { 0.0 } else { voice.gain };
                self.spatial.render(&Source { id, position: voice.pos, gain }, &mono, &mut stereo);
                mix.add(&stereo);
            }
            let mut reverb = [[0.0; 2]; BLOCK];
            self.spatial.finish_block(&mut reverb);
            mix.add(&reverb);
            for (frame, samples) in mix.finish().iter().enumerate() {
                for channel in 0..2 {
                    let at = channel * 0x400 + frame * 2;
                    self.memory.bytes[at..at + 2].copy_from_slice(&(limit(samples[channel] * 36.0 * self.mix_gain).mul_add(32767.0, 0.0) as i16).to_le_bytes());
                }
            }
            output.extend(self.renderer.step(&mut self.memory));
            self.rendered += BLOCK as u64;
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

const COMMANDS: usize = 1024;
static WORKER_NANOS: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq)]
enum Command {
    Move { source: usize, pos: V3 },
    Play { sample: Sample, pos: V3, gain: f32, pitch: f32 },
    Listen { eye: V3, yaw: f32, dt: f32 },
}

struct CommandRing {
    slots: [UnsafeCell<MaybeUninit<Command>>; COMMANDS],
    written: AtomicUsize,
    read: AtomicUsize,
}

// Sound owns the only producer and its worker owns the only consumer; release/acquire publishes each slot.
unsafe impl Sync for CommandRing {}

impl CommandRing {
    fn new() -> Self {
        Self {
            slots: std::array::from_fn(|_| UnsafeCell::new(MaybeUninit::uninit())),
            written: AtomicUsize::new(0),
            read: AtomicUsize::new(0),
        }
    }

    fn push(&self, command: Command) -> bool {
        let written = self.written.load(Ordering::Relaxed);
        if written.wrapping_sub(self.read.load(Ordering::Acquire)) == COMMANDS {
            return false;
        }
        // SAFETY: only the producer writes this slot, and it cannot lap the consumer.
        unsafe { (*self.slots[written % COMMANDS].get()).write(command) };
        self.written.store(written.wrapping_add(1), Ordering::Release);
        true
    }

    fn pop(&self) -> Option<Command> {
        let read = self.read.load(Ordering::Relaxed);
        if read == self.written.load(Ordering::Acquire) {
            return None;
        }
        // SAFETY: the acquire load observed the producer's initialized slot.
        let command = unsafe { (*self.slots[read % COMMANDS].get()).assume_init_read() };
        self.read.store(read.wrapping_add(1), Ordering::Release);
        Some(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drips_stay_sparse_and_natural() {
        let mut rng = Rng(7);
        let mut cycle = 0;
        for i in 0..1000 {
            let (pitch, delay) = drip_timing(&mut rng);
            assert!((0.97..=1.03).contains(&pitch));
            assert!(((2.0 * RATE) as u64..=(5.0 * RATE) as u64).contains(&delay));
            cycle += delay;
            if i % DRIPS == DRIPS - 1 {
                assert!(((8.0 * RATE) as u64..=(20.0 * RATE) as u64).contains(&cycle));
                cycle = 0;
            }
        }
        for _ in 0..1000 { assert!(((10.0 * RATE) as u64..=(20.0 * RATE) as u64).contains(&far_drip_delay(&mut rng))); }
    }

    #[test]
    fn walking_speed_sets_step_cadence() {
        assert!((step_cadence(0.7) - 1.3).abs() < 0.01);
        assert!(step_cadence(0.6) < step_cadence(0.7));
        assert_eq!(step_cadence(0.0), 0.0);
    }

    #[test]
    fn command_ring_hands_off_in_order() {
        let ring = Arc::new(CommandRing::new());
        let consumer = Arc::clone(&ring);
        let thread = std::thread::spawn(move || {
            for source in 0..COMMANDS * 4 {
                loop {
                    if let Some(command) = consumer.pop() {
                        assert_eq!(command, Command::Move { source, pos: [source as f32; 3] });
                        break;
                    }
                    std::thread::yield_now();
                }
            }
        });
        for source in 0..COMMANDS * 4 {
            let command = Command::Move { source, pos: [source as f32; 3] };
            while !ring.push(command) {
                std::thread::yield_now();
            }
        }
        thread.join().unwrap();
    }
}

struct Worker {
    commands: Arc<CommandRing>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    fn new(mut engine: Engine) -> Self {
        let commands = Arc::new(CommandRing::new());
        let stop = Arc::new(AtomicBool::new(false));
        let thread_commands = Arc::clone(&commands);
        let thread_stop = Arc::clone(&stop);
        let thread = std::thread::spawn(move || {
            let period = Duration::from_millis(5);
            let mut deadline = Instant::now();
            while !thread_stop.load(Ordering::Acquire) {
                while let Some(command) = thread_commands.pop() {
                    match command {
                        Command::Move { source, pos } => engine.move_source(source, pos),
                        Command::Play { sample, pos, gain, pitch } => engine.play(sample, pos, gain, pitch),
                        Command::Listen { eye, yaw, dt } => engine.listen(eye, yaw, dt),
                    }
                }
                if crate::audio::queued() <= 3 * BLOCK {
                    let start = Instant::now();
                    let output = engine.render(1);
                    crate::audio::play(&[&output]);
                    WORKER_NANOS.fetch_add(start.elapsed().as_nanos() as u64, Ordering::Relaxed);
                }
                deadline += period;
                let now = Instant::now();
                if deadline > now {
                    std::thread::sleep(deadline - now);
                } else {
                    deadline = now;
                }
            }
        });
        Self { commands, stop, thread: Some(thread) }
    }

    fn send(&self, command: Command) {
        assert!(self.commands.push(command), "audio command ring full");
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

pub struct Sound {
    engine: Option<Engine>,
    worker: Option<Worker>,
}

impl Sound {
    pub fn new(world: &World, sources: &[(Sfx, V3, f32)], threaded: bool, eye: V3, yaw: f32) -> Self {
        let mut engine = Engine::new(world, sources);
        engine.listen(eye, yaw, 0.0);
        if threaded {
            Self { engine: None, worker: Some(Worker::new(engine)) }
        } else {
            Self { engine: Some(engine), worker: None }
        }
    }

    pub fn move_source(&mut self, source: usize, pos: V3) {
        if let Some(worker) = &self.worker { worker.send(Command::Move { source, pos }); }
        else { self.engine.as_mut().unwrap().move_source(source, pos); }
    }

    pub fn play(&mut self, sample: Sample, pos: V3, gain: f32, pitch: f32) {
        if let Some(worker) = &self.worker { worker.send(Command::Play { sample, pos, gain, pitch }); }
        else { self.engine.as_mut().unwrap().play(sample, pos, gain, pitch); }
    }

    pub fn listen(&mut self, _world: &World, eye: V3, yaw: f32, dt: f32) {
        if let Some(worker) = &self.worker { worker.send(Command::Listen { eye, yaw, dt }); }
        else { self.engine.as_mut().unwrap().listen(eye, yaw, dt); }
    }

    pub fn render(&mut self, periods: usize) -> Vec<i16> {
        self.engine.as_mut().map_or_else(Vec::new, |engine| engine.render(periods))
    }

    pub fn worker_millis(&self) -> f64 {
        if self.worker.is_some() { WORKER_NANOS.swap(0, Ordering::Relaxed) as f64 / 1e6 } else { 0.0 }
    }
}
