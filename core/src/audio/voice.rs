//! voices: four queued wave buffers decoded in order, pitched to the renderer rate, filtered by up
//! to two biquads and volume-ramped once per period, ready for the mix routing.

use super::dsp::adpcm::{AdpcmContext, AdpcmParams, GcAdpcmDecoder};
use super::dsp::biquad::{Biquad, BiquadParams};
use super::dsp::resampler::{Resampler, ResamplerParams, SrcQuality};
use super::dsp::volume::{VolumeRamp, VolumeRampParams};
use super::parcel::{f32_at, i32_at, put, u16_at, u32_at, u64_at};
use super::renderer::{
    Pools, REV_DECODING_FLAGS, REV_FLUSH, REV_LOOP_CONTEXT, REV_SPLITTER, REV_WAVE_BUFFER_V2,
};
use super::{Error, GuestMemory, in_place};

// switchbrew "SampleFormat"
pub const PCM_INT16: u8 = 2;
pub const PCM_FLOAT: u8 = 5;
pub const ADPCM: u8 = 6;
pub const UNUSED_MIX: i32 = i32::MAX;
pub const UNUSED_SPLITTER: u32 = u32::MAX;
const ADPCM_FRAME: usize = 14;
const RESET_PLAYED_AT_LOOP: u16 = 1;
const SKIP_PITCH_AND_SRC: u16 = 2;
const RESAMPLER_LOOKAHEAD: usize = 17;

#[derive(Clone, Copy)]
pub struct ChannelResource {
    pub mix: [f32; 24],
    pub previous: [f32; 24],
}

#[derive(Clone, Copy, Default, PartialEq)]
enum Play {
    Started,
    #[default]
    Stopped,
    Stopping,
    Paused,
}

#[derive(Clone, Copy, Default)]
struct WaveBuffer {
    address: u64,
    start: u32,
    end: u32,
    looping: bool,
    end_of_stream: bool,
    loop_count: i32,
    loop_start: u32,
    loop_end: u32,
    context: u64,
    queued: bool,
    valid: bool,
}

pub struct Channel {
    pending: Vec<f32>,
    resampler: Resampler,
    filters: [Biquad; 2],
    pub output: Vec<f32>,
    pub last: [f32; 24],
}

pub enum Rendered {
    Silent,
    Stopped,
    Playing,
}

pub struct Voice {
    pub in_use: bool,
    pub new: bool,
    play: Play,
    previous_play: Play,
    format: u8,
    sample_rate: u32,
    pub channel_count: usize,
    pitch: f32,
    volume: f32,
    previous_volume: f32,
    biquads: [Option<BiquadParams>; 2],
    filtering: [bool; 2],
    pub mix_id: i32,
    pub splitter_id: u32,
    pub resources: [usize; 6],
    coefficients: Option<u64>,
    flags: u16,
    quality: SrcQuality,
    waves: [WaveBuffer; 4],
    index: usize,
    offset: u32,
    played: u64,
    consumed: u32,
    loops: i32,
    flush: u32,
    pub channels: [Channel; 6],
    adpcm: GcAdpcmDecoder,
    raw: Vec<u8>,
}

pub fn biquad_q14(numerator: [i16; 3], denominator: [i16; 2]) -> BiquadParams {
    let q14 = |v: i16| f32::from(v) / 16384.0;
    BiquadParams {
        b0: q14(numerator[0]),
        b1: q14(numerator[1]),
        b2: q14(numerator[2]),
        a1: -q14(denominator[0]),
        a2: -q14(denominator[1]),
        new_state: false,
    }
}

/// a 0xC-byte BiquadFilterParameter: enable, then q14 numerator[3] and denominator[2].
pub fn biquad_parameter(b: &[u8]) -> Option<BiquadParams> {
    let i16_at = |at| u16_at(b, at) as i16;
    (b[0] != 0).then(|| biquad_q14([i16_at(2), i16_at(4), i16_at(6)], [i16_at(8), i16_at(10)]))
}

fn sample_bytes(format: u8, channels: usize, samples: u32) -> Option<u64> {
    let samples = u64::from(samples);
    match format {
        PCM_INT16 => Some(samples * 2 * channels as u64),
        PCM_FLOAT => Some(samples * 4 * channels as u64),
        ADPCM => Some(samples.div_ceil(ADPCM_FRAME as u64) * 8),
        _ => None,
    }
}

impl Default for Voice {
    fn default() -> Self {
        let resampler = ResamplerParams {
            sample_rate: 48_000,
            pitch: 1.0,
            quality: SrcQuality::Medium,
        };
        let filter = BiquadParams {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            new_state: true,
        };
        Self {
            in_use: false,
            new: false,
            play: Play::Stopped,
            previous_play: Play::Stopped,
            format: 0,
            sample_rate: 0,
            channel_count: 0,
            pitch: 1.0,
            volume: 0.0,
            previous_volume: 0.0,
            biquads: [None; 2],
            filtering: [false; 2],
            mix_id: UNUSED_MIX,
            splitter_id: UNUSED_SPLITTER,
            resources: [0; 6],
            coefficients: None,
            flags: 0,
            quality: SrcQuality::Medium,
            waves: [WaveBuffer::default(); 4],
            index: 0,
            offset: 0,
            played: 0,
            consumed: 0,
            loops: 0,
            flush: 0,
            channels: std::array::from_fn(|_| Channel {
                pending: Vec::new(),
                resampler: Resampler::new(resampler),
                filters: [Biquad::new(filter), Biquad::new(filter)],
                output: Vec::new(),
                last: [0.0; 24],
            }),
            adpcm: GcAdpcmDecoder::new(AdpcmParams {
                coefficients: [0; 16],
            }),
            raw: Vec::new(),
        }
    }
}

impl Voice {
    /// applies a 0x170-byte VoiceInParameter and writes its 0x10-byte VoiceOutStatus.
    pub fn update(
        &mut self,
        b: &[u8],
        out: &mut [u8],
        rev: u32,
        voice_count: usize,
        pools: &mut Pools,
    ) -> Result<(), Error> {
        self.in_use = b[9] != 0;
        if !self.in_use {
            return Ok(());
        }
        let channel_count = u32_at(b, 0x18) as usize;
        let resources: [i32; 6] = std::array::from_fn(|i| i32_at(b, 0x140 + i * 4));
        if !(1..=6).contains(&channel_count)
            || resources[..channel_count]
                .iter()
                .any(|&r| r < 0 || r as usize >= voice_count)
        {
            return Err(Error::InvalidUpdateInfo);
        }
        if b[8] != 0 {
            *self = Self {
                in_use: true,
                new: true,
                ..Self::default()
            };
        }
        let previous_play = self.play;
        self.play = match (b[0xA], self.play) {
            (0, _) => Play::Started,
            (1, Play::Stopped) => Play::Stopped,
            (1, _) => Play::Stopping,
            _ => Play::Paused,
        };
        self.previous_play = previous_play;
        self.format = b[0xB];
        self.sample_rate = u32_at(b, 0xC);
        self.channel_count = channel_count;
        self.pitch = f32_at(b, 0x1C);
        self.volume = f32_at(b, 0x20);
        self.biquads = std::array::from_fn(|i| biquad_parameter(&b[0x24 + i * 0xC..]));
        if rev >= REV_FLUSH {
            self.flush += u32::from(b[0x159]);
        }
        self.mix_id = i32_at(b, 0x58);
        self.splitter_id = if rev >= REV_SPLITTER {
            u32_at(b, 0x5C)
        } else {
            UNUSED_SPLITTER
        };
        self.resources = resources.map(|r| r.max(0) as usize);
        self.flags = if rev >= REV_DECODING_FLAGS {
            u16_at(b, 0x15C)
        } else {
            0
        };
        self.quality = match b[0x15E] {
            1 => SrcQuality::High,
            2 => SrcQuality::Low,
            _ => SrcQuality::Medium,
        };
        if self.format == ADPCM {
            let (address, size) = (u64_at(b, 0x48), u64_at(b, 0x50));
            self.coefficients = (size >= 0x20 && pools.attach(address, size)).then_some(address);
        }
        for (i, wave) in self.waves.iter_mut().enumerate() {
            let w = &b[0x60 + i * 0x38..][..0x38];
            if w[0x1A] != 0 {
                continue;
            }
            let (address, size, start, end) =
                (u64_at(w, 0), u64_at(w, 8), u32_at(w, 0x10), u32_at(w, 0x14));
            let fits = sample_bytes(self.format, channel_count, end)
                .is_some_and(|bytes| start <= end && bytes <= size);
            if !fits {
                pools.errors.push((Error::NoMemoryPoolEntry, address));
                continue;
            }
            let v2 = rev >= REV_WAVE_BUFFER_V2;
            let context = (rev >= REV_LOOP_CONTEXT && self.format == ADPCM && u64_at(w, 0x20) != 0)
                .then(|| u64_at(w, 0x20));
            if !pools.attach(address, size)
                || context.is_some_and(|c| !pools.attach(c, u64_at(w, 0x28)))
            {
                continue;
            }
            *wave = WaveBuffer {
                address,
                start,
                end,
                looping: w[0x18] != 0,
                end_of_stream: w[0x19] != 0,
                loop_count: if v2 { i32_at(w, 0x1C) } else { -1 },
                loop_start: if v2 { u32_at(w, 0x30) } else { 0 },
                loop_end: if v2 { u32_at(w, 0x34) } else { 0 },
                context: context.unwrap_or(0),
                queued: true,
                valid: false,
            };
        }
        if !self.new {
            put(out, 0, &self.played.to_le_bytes());
            put(out, 8, &self.consumed.to_le_bytes());
        }
        Ok(())
    }

    fn finish_wave(&mut self) {
        let wave = &mut self.waves[self.index];
        wave.valid = false;
        if wave.end_of_stream {
            self.played = 0;
        }
        self.loops = 0;
        self.consumed += 1;
        self.index = (self.index + 1) % 4;
    }

    fn stop(&mut self) {
        for i in 0..4 {
            if self.waves[i].valid {
                self.consumed += 1;
            }
            self.waves[i].valid = false;
            self.waves[i].queued = false;
        }
        (self.offset, self.played, self.index) = (0, 0, 0);
        for channel in &mut self.channels {
            channel.pending.clear();
            channel.resampler = Resampler::new(ResamplerParams {
                sample_rate: 48_000,
                pitch: 1.0,
                quality: self.quality,
            });
        }
        self.adpcm.restore(AdpcmContext::default());
    }

    /// decodes, pitches, filters and ramps one period into each channel's `output`.
    pub fn render(&mut self, memory: &dyn GuestMemory, rate: u32, frames: usize) -> Rendered {
        if !self.in_use || self.channel_count == 0 {
            return Rendered::Silent;
        }
        if self.new {
            self.new = false;
            self.previous_volume = self.volume;
            for channel in &mut self.channels {
                channel.last = [0.0; 24];
            }
        }
        for _ in 0..std::mem::take(&mut self.flush) {
            if self.waves[self.index].valid {
                self.finish_wave();
            }
        }
        let was_playing = self.previous_play == Play::Started;
        match self.play {
            Play::Started => {
                for wave in &mut self.waves {
                    wave.valid |= std::mem::take(&mut wave.queued);
                }
                if !self.waves.iter().any(|wave| wave.valid) {
                    return Rendered::Silent;
                }
            }
            Play::Stopping => {
                self.stop();
                self.play = Play::Stopped;
                return if was_playing {
                    Rendered::Stopped
                } else {
                    Rendered::Silent
                };
            }
            Play::Stopped | Play::Paused => {
                return if was_playing {
                    Rendered::Stopped
                } else {
                    Rendered::Silent
                };
            }
        }
        let skip_src = self.flags & SKIP_PITCH_AND_SRC != 0;
        let step = self.sample_rate as f32 / rate as f32 * self.pitch;
        let need = if skip_src {
            frames
        } else {
            (frames as f32 * step).ceil() as usize + RESAMPLER_LOOKAHEAD
        };
        self.decode(memory, need);
        let ramp = VolumeRampParams {
            previous_volume: self.previous_volume,
            volume: self.volume,
        };
        let resampler = ResamplerParams {
            sample_rate: self.sample_rate,
            pitch: self.pitch * 48_000.0 / rate as f32,
            quality: self.quality,
        };
        for channel in &mut self.channels[..self.channel_count] {
            channel.pending.resize(channel.pending.len().max(need), 0.0);
            channel.output.clear();
            channel.output.resize(frames, 0.0);
            let consumed = if skip_src {
                channel.output.copy_from_slice(&channel.pending[..frames]);
                frames
            } else {
                channel.resampler.set(resampler);
                channel
                    .resampler
                    .process(&channel.pending, &mut channel.output)
            };
            channel.pending.drain(..consumed);
            for (i, filter) in channel.filters.iter_mut().enumerate() {
                if let Some(params) = self.biquads[i] {
                    filter.set(BiquadParams {
                        new_state: !self.filtering[i],
                        ..params
                    });
                    in_place(&mut channel.output, |input, output| {
                        filter.process(input, output, frames)
                    });
                }
            }
            in_place(&mut channel.output, |input, output| {
                VolumeRamp::new(ramp).process(input, output, frames)
            });
        }
        self.filtering = self.biquads.map(|b| b.is_some());
        self.previous_volume = self.volume;
        Rendered::Playing
    }

    fn decode(&mut self, memory: &dyn GuestMemory, need: usize) {
        let mut coefficients = [0u8; 32];
        if let Some(address) = self.coefficients {
            memory.read(address, &mut coefficients);
        }
        self.adpcm.set(AdpcmParams {
            coefficients: std::array::from_fn(|i| u16_at(&coefficients, i * 2) as i16),
        });
        while self.channels[0].pending.len() < need {
            let wave = self.waves[self.index];
            if !wave.valid || (self.format == ADPCM && self.coefficients.is_none()) {
                break;
            }
            if self.offset == 0 {
                let mut context = self.adpcm.context();
                if wave.context != 0 {
                    let mut raw = [0u8; 6];
                    memory.read(wave.context, &mut raw);
                    context = AdpcmContext {
                        predictor_scale: raw[0],
                        history_sample_1: u16_at(&raw, 2) as i16,
                        history_sample_2: u16_at(&raw, 4) as i16,
                    };
                }
                self.adpcm.restore(context);
            }
            let looped = self.loops > 0
                && wave.loop_start != 0
                && wave.loop_end != 0
                && wave.loop_start <= wave.loop_end;
            let (start, end) = if looped {
                (wave.loop_start, wave.loop_end)
            } else {
                (wave.start, wave.end)
            };
            let position = (start + self.offset) as usize;
            let mut count = (end.saturating_sub(start + self.offset) as usize)
                .min(need - self.channels[0].pending.len());
            if self.format == ADPCM {
                count = (position + count)
                    .next_multiple_of(ADPCM_FRAME)
                    .min(end as usize)
                    - position;
            }
            let decoded = self.read(memory, wave.address, position, count);
            self.offset += decoded as u32;
            self.played += decoded as u64;
            if self.offset >= end - start || decoded == 0 {
                self.offset = 0;
                if !wave.looping {
                    self.finish_wave();
                    continue;
                }
                self.loops += 1;
                if wave.loop_count >= 0 && (decoded == 0 || self.loops > wave.loop_count) {
                    self.finish_wave();
                }
                if decoded == 0 {
                    break;
                }
                if self.flags & RESET_PLAYED_AT_LOOP != 0 {
                    self.played = 0;
                }
            }
        }
    }

    fn read(
        &mut self,
        memory: &dyn GuestMemory,
        address: u64,
        position: usize,
        count: usize,
    ) -> usize {
        let channels = self.channel_count;
        match self.format {
            PCM_INT16 | PCM_FLOAT => {
                let width = if self.format == PCM_INT16 { 2 } else { 4 };
                self.raw.resize(count * width * channels, 0);
                memory.read(
                    address + (position * width * channels) as u64,
                    &mut self.raw,
                );
                for (i, sample) in self.raw.chunks_exact(width).enumerate() {
                    let value = if width == 2 {
                        f32::from(u16_at(sample, 0) as i16) / 32768.0
                    } else {
                        f32_at(sample, 0)
                    };
                    self.channels[i % channels].pending.push(value);
                }
                count
            }
            ADPCM => {
                let skip = position % ADPCM_FRAME;
                self.raw.resize((skip + count).div_ceil(ADPCM_FRAME) * 8, 0);
                memory.read(address + (position / ADPCM_FRAME * 8) as u64, &mut self.raw);
                let mut decoded = vec![0.0; skip + count];
                let written = self
                    .adpcm
                    .process(&self.raw, &mut decoded, skip + count)
                    .samples;
                self.channels[0]
                    .pending
                    .extend_from_slice(&decoded[skip..written]);
                written - skip
            }
            _ => 0,
        }
    }
}
