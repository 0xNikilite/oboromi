//! The music demo: a 16-bar loop driven the way a game drives audren. Four instruments are synthesised into PCM16
//! once, every note restarts a voice on its wave buffer through a RequestUpdate parcel, and the final mix runs a
//! low-pass biquad and a stereo delay before the device sink. `Sound` is that renderer setup, the bounce demo reuses it.
use super::renderer::{AudioRendererParameter, Renderer, work_buffer_size};
use super::voice::PCM_INT16;
use super::{HostMemory, play};
use std::f32::consts::{FRAC_1_SQRT_2, FRAC_PI_4, TAU};
use std::sync::Mutex;

const BASE: u64 = 0x10_0000;
const REV13: u32 = 0x3d56_4552;
const EFFECTS: usize = 2;
/// Periods queued ahead of wall time, so the sink never drains dry between frames.
const LEAD: u64 = 10;

fn record(len: usize, fields: &[(usize, &[u8])]) -> Vec<u8> {
    let mut record = vec![0; len];
    for &(at, value) in fields {
        record[at..at + value.len()].copy_from_slice(value);
    }
    record
}

#[derive(Clone, Copy, Default)]
struct Note {
    sample: usize,
    pitch: f32,
    volume: f32,
    pan: f32,
    used: bool,
    start: bool,
    ends: u64,
}

pub struct Sound {
    renderer: Renderer,
    memory: HostMemory,
    samples: Vec<(u64, u32)>,
    notes: Vec<Note>,
    sent: bool,
    pub periods: u64,
    skipped: u64,
    seconds: f32,
}

impl Sound {
    /// A renderer with `voices` mono voices over one memory pool holding `samples`, all 48 kHz PCM16.
    pub fn new(samples: &[Vec<i16>], voices: usize) -> Self {
        let (mut bytes, mut placed) = (Vec::new(), Vec::new());
        for sample in samples {
            placed.push((BASE + bytes.len() as u64, sample.len() as u32));
            bytes.extend(sample.iter().flat_map(|s| s.to_le_bytes()));
            bytes.resize(bytes.len().next_multiple_of(0x1000), 0);
        }
        let parameter = AudioRendererParameter {
            sample_rate: 48_000,
            sample_count: 240,
            mix_buffer_count: 2,
            voice_count: voices as u32,
            sink_count: 1,
            effect_count: EFFECTS as u32,
            revision: REV13,
            ..Default::default()
        };
        let mut work = vec![0; work_buffer_size(&parameter).unwrap() as usize];
        let mut renderer = Renderer::open(parameter, &mut work).unwrap();
        renderer.start();
        let memory = HostMemory { base: BASE, bytes };
        let notes = vec![Note::default(); voices];
        Self { renderer, memory, samples: placed, notes, sent: false, periods: 0, skipped: 0, seconds: 0.0 }
    }

    /// Restarts `voice` on `sample` from the next period, `pan` from -1 (left) to 1 (right).
    pub fn trigger(&mut self, voice: usize, sample: usize, pitch: f32, volume: f32, pan: f32) {
        let ends = self.periods + (self.samples[sample].1 as f32 / pitch / 240.0) as u64 + 1;
        self.notes[voice] = Note { sample, pitch, volume, pan, used: true, start: true, ends };
    }

    /// The voice whose note ends first, a silent one if any.
    pub fn free_voice(&self) -> usize {
        (0..self.notes.len()).min_by_key(|&v| self.notes[v].ends).unwrap()
    }

    fn parcel(&mut self) -> Vec<u8> {
        let (new, voices) = (u8::from(!self.sent), self.notes.len());
        let size = (self.memory.bytes.len() as u64).to_le_bytes();
        let attach = record(0x20, &[(0, &BASE.to_le_bytes()), (8, &size), (0x10, &4u32.to_le_bytes())]);
        let pools = [attach, vec![0; 0x20 * (EFFECTS + 4 * voices - 1)]].concat();
        let (mut channels, mut records) = (Vec::new(), Vec::new());
        for (i, note) in self.notes.iter_mut().enumerate() {
            let id = (i as u32).to_le_bytes();
            let angle = (note.pan.clamp(-1.0, 1.0) + 1.0) * FRAC_PI_4;
            let mix = [angle.cos().to_le_bytes(), angle.sin().to_le_bytes()].concat();
            channels.extend(record(0x70, &[(0, &id), (4, &mix), (0x64, &[1])]));
            let (address, samples) = self.samples[note.sample];
            let wave = record(
                0x38,
                &[(0, &address.to_le_bytes()), (8, &(u64::from(samples) * 2).to_le_bytes()), (0x14, &samples.to_le_bytes()), (0x1A, &[u8::from(!note.start)])],
            );
            records.extend(record(
                0x170,
                &[
                    (0, &id),
                    (8, &[u8::from(note.start), u8::from(note.used), 0, PCM_INT16]),
                    (0xC, &48_000u32.to_le_bytes()),
                    (0x18, &1u32.to_le_bytes()),
                    (0x1C, &note.pitch.to_le_bytes()),
                    (0x20, &note.volume.to_le_bytes()),
                    (0x5C, &u32::MAX.to_le_bytes()),
                    (0x60, &wave),
                    (0x140, &id),
                ],
            ));
            note.start = false;
        }
        // RBJ low-pass at 7 kHz; audren's denominator is the negated a1, a2.
        let omega = TAU * 7000.0 / 48_000.0;
        let (alpha, cos) = (omega.sin() * FRAC_1_SQRT_2, omega.cos());
        let b = [(1.0 - cos) * 0.5, 1.0 - cos, (1.0 - cos) * 0.5, 2.0 * cos, alpha - 1.0].map(|c| ((c / (1.0 + alpha) * 16384.0) as i16).to_le_bytes());
        let low_pass = record(0xC0, &[(0, &[6, new, 1]), (0x20, &[0, 1]), (0x26, &[0, 1]), (0x2C, &b.concat()), (0x36, &[2, u8::from(self.sent)])]);
        let gains = [0.6, 0.35, 0.5, 1.0, 0.6, 0.3].map(|gain: f32| ((gain * 16384.0) as u32).to_le_bytes()).concat();
        let delay = record(
            0xC0,
            &[(0, &[3, new, 1]), (0x18, &1u32.to_le_bytes()), (0x20, &[0, 1]), (0x26, &[0, 1]), (0x2C, &[2, 0, 2, 0]), (0x30, &400u32.to_le_bytes()), (0x34, &240u32.to_le_bytes()), (0x3C, &gains)],
        );
        let final_mix = record(
            0x930,
            &[(0, &1f32.to_le_bytes()), (4, &48_000u32.to_le_bytes()), (8, &2u32.to_le_bytes()), (0xC, &[1]), (0x924, &i32::MAX.to_le_bytes()), (0x928, &u32::MAX.to_le_bytes())],
        );
        let mixes = [record(0x20, &[(4, &1u32.to_le_bytes())]), final_mix].concat();
        let sink = record(0x140, &[(0, &[1, 1]), (0x120, &2u32.to_le_bytes()), (0x124, &[0, 1])]);
        let sections = [record(0x10, &[(0, &REV13.to_le_bytes())]), pools, channels, records, [low_pass, delay].concat(), mixes, sink, vec![0; 0x10]];
        let mut header = record(0x40, &[(0, &REV13.to_le_bytes())]);
        for (slot, section) in [0, 1, 3, 2, 4, 5, 6, 7].into_iter().enumerate() {
            header[4 + slot * 4..8 + slot * 4].copy_from_slice(&(sections[section].len() as u32).to_le_bytes());
        }
        let total = 0x40 + sections.iter().map(Vec::len).sum::<usize>();
        header[0x3C..].copy_from_slice(&(total as u32).to_le_bytes());
        self.sent = true;
        [header, sections.concat()].concat()
    }

    /// Sends this period's parcel and renders it.
    pub fn period(&mut self) -> &[i16] {
        let parcel = self.parcel();
        self.renderer.request_update(&parcel, &mut vec![0; 0x4000]).unwrap();
        self.periods += 1;
        self.renderer.step(&mut self.memory)
    }

    /// The periods wall time `seconds` asks for since the last call, at 200 a second whatever the frame rate. A
    /// restarted clock starts over.
    pub fn due(&mut self, seconds: f32) -> u64 {
        if seconds < self.seconds {
            (self.periods, self.skipped) = (0, 0);
        }
        self.seconds = seconds;
        let due = (f64::from(seconds) * 200.0) as u64 + LEAD;
        // A long stall (the first frame's shader compiles) would leave a backlog the sink then plays late.
        if due > self.periods + self.skipped + 4 * LEAD {
            self.skipped = due - LEAD - self.periods;
        }
        due.saturating_sub(self.periods + self.skipped)
    }
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

pub fn music() -> Sound {
    Sound::new(&instruments(), 8)
}

/// Starts this period's notes from the pattern table: the arpeggio throughout, bass from bar 3, drums from bar 5.
pub fn sequence(sound: &mut Sound) {
    if sound.periods % STEP != 0 {
        return;
    }
    let step = (sound.periods / STEP) as usize % 256;
    let (bar, beat) = (step / 16, step % 16);
    let chord = CHORDS[bar / 2 % 4];
    let semitones = |n: i32| 2f32.powf(n as f32 / 12.0);
    let side = if beat % 2 == 0 { -0.5 } else { 0.5 };
    sound.trigger(3 + beat % 5, PLUCK, semitones(chord[ARPEGGIO[beat]]), 0.22, side);
    if bar >= 2 && BASS_STEPS >> beat & 1 != 0 {
        sound.trigger(2, BASS, semitones(chord[0] - if chord[0] > 0 { 12 } else { 0 }), 0.35, 0.0);
    }
    if bar >= 4 && beat % 4 == 0 {
        sound.trigger(0, KICK, 1.0, 0.5, 0.0);
    }
    if bar >= 4 && beat % 2 == 0 && beat % 4 != 0 {
        sound.trigger(1, HAT, 1.0, 0.3, 0.3);
    }
}

pub fn play_music(seconds: f32) {
    static MUSIC: Mutex<Option<Sound>> = Mutex::new(None);
    let mut slot = MUSIC.lock().unwrap();
    let sound = slot.get_or_insert_with(music);
    let mut peak = [0u16; 2];
    for _ in 0..sound.due(seconds) {
        sequence(sound);
        let samples = sound.period();
        for frame in samples.chunks_exact(2) {
            for channel in 0..2 { peak[channel] = peak[channel].max(frame[channel].unsigned_abs()); }
        }
        play(&[samples]);
    }
    let (width, height) = (960, 540);
    let mut pixels = [12, 16, 24, 255].repeat(width * height);
    for channel in 0..2 {
        let top = height - 40 - usize::from(peak[channel]) * (height - 80) / 32768;
        for y in 40..height - 40 {
            let colour = if y >= top { [48, 210, 160, 255] } else { [28, 36, 48, 255] };
            for x in width / 3 * (channel + 1) - 36..width / 3 * (channel + 1) + 36 {
                pixels[(y * width + x) * 4..][..4].copy_from_slice(&colour);
            }
        }
    }
    crate::display::present(width as u32, height as u32, &pixels);
}
