//! audren's IAudioRenderer: RequestUpdate parcels update the memory pools, voices, effects, mixes
//! and sinks, and each `step` renders one period (240 frames at 48 kHz, 5 ms) through them.

use super::dsp::depop::{Depop, DepopParams};
use super::effect::Effect;
use super::mix::Graph;
use super::parcel::*;
use super::sink_device::Sink;
use super::voice::{ChannelResource, Rendered, Voice};
use super::{Error, GuestMemory};

// switchbrew "BehaviorInfo Revision": the revision each behaviour starts at.
pub const REV_SPLITTER: u32 = 2;
pub const REV_LOOP_CONTEXT: u32 = 2;
pub const REV_FLUSH: u32 = 5;
pub const REV_DECODING_FLAGS: u32 = 5;
pub const REV_ELAPSED_FRAMES: u32 = 5;
pub const REV_PERFORMANCE_V2: u32 = 5;
pub const REV_SPLITTER_FIX: u32 = 5;
pub const REV_BIQUAD_STATE_CLEAR_FIX: u32 = 6;
pub const REV_DIRTY_MIXES: u32 = 7;
pub const REV_WAVE_BUFFER_V2: u32 = 8;
pub const REV_EFFECT_INFO_V2: u32 = 9;
pub const REV_SPLITTER_BIQUADS: u32 = 12;
pub const REV_SPLITTER_PREVIOUS_VOLUME_RESET: u32 = 13;
pub const LATEST: u32 = 13;

/// nn::audio::detail::AudioRendererParameterInternal, the 0x34 bytes OpenAudioRenderer takes (libnx audren.c).
#[derive(Clone, Copy, Debug, Default)]
pub struct AudioRendererParameter {
    pub sample_rate: u32,
    pub sample_count: u32,
    pub mix_buffer_count: u32,
    pub sub_mix_count: u32,
    pub voice_count: u32,
    pub sink_count: u32,
    pub effect_count: u32,
    pub performance_frame_count: u32,
    pub splitter_count: u32,
    pub splitter_destination_count: u32,
    pub revision: u32,
}

/// the revision number of a 'REV' magic, or 'REv' on Switch 2 (switchbrew "BehaviorInfo Revision").
pub fn revision(magic: u32) -> Result<u32, Error> {
    let number = (magic >> 24).wrapping_sub(u32::from(b'0'));
    match magic & 0xFF_FFFF {
        0x56_4552 | 0x76_4552 if (1..=LATEST).contains(&number) => Ok(number),
        _ => Err(Error::OperationFailed),
    }
}

/// GetWorkBufferSize: the server state the console keeps in the work buffer, laid out as the
/// reference emulators reserve it.
pub fn work_buffer_size(p: &AudioRendererParameter) -> Result<u64, Error> {
    let rev = revision(p.revision)?;
    let [
        samples,
        buffers,
        mixes,
        voices,
        sinks,
        effects,
        splitters,
        destinations,
        frames,
    ] = [
        p.sample_count,
        p.mix_buffer_count,
        p.sub_mix_count + 1,
        p.voice_count,
        p.sink_count,
        p.effect_count,
        p.splitter_count,
        p.splitter_destination_count,
        p.performance_frame_count,
    ]
    .map(u64::from);
    let mut size = 0u64;
    let mut reserve =
        |count: u64, unit: u64, align: u64| size = size.next_multiple_of(align) + count * unit;
    reserve(samples * (6 + buffers), 4, 0x10);
    reserve(240 * (6 + buffers) * (sinks + mixes - 1), 4, 0x10);
    reserve(buffers.next_multiple_of(0x40), 4, 0x40);
    reserve(voices, 0x200, 0x10);
    reserve(voices, 4, 0x10);
    reserve(voices, 0xD0, 0x10);
    reserve(voices, 0xD8, 0x10);
    reserve(mixes, 0x940, 0x10);
    reserve(effects * mixes, 4, 0x10);
    reserve(mixes, 4, 0x10);
    if rev >= REV_SPLITTER {
        let edges = (mixes * mixes).next_multiple_of(0x40);
        reserve(
            1,
            (mixes * mixes * 4 + 0xC * mixes + 3 * edges).next_multiple_of(0x10),
            1,
        );
    }
    reserve(effects + voices * 4, 0x20, 0x10);
    if rev >= REV_SPLITTER {
        reserve(splitters, 0x20, 0x10);
        reserve(
            destinations,
            if rev >= REV_SPLITTER_BIQUADS {
                0x110
            } else {
                0xE0
            },
            0x10,
        );
        if rev >= REV_SPLITTER_FIX {
            reserve(destinations, 4, 0x10);
        }
    }
    if rev >= REV_SPLITTER_BIQUADS && splitters > 0 && destinations > 0 {
        reserve(destinations * 4, 0x20, 0x10);
    }
    reserve(voices, 0xD8, 0x10);
    if frames > 0 {
        let (entry, detail, header) = if rev >= REV_PERFORMANCE_V2 {
            (0x18, 0x18, 0x30)
        } else {
            (0x10, 0x10, 0x18)
        };
        let frame = entry * (voices + effects + mixes + sinks) + detail * 100 + header;
        reserve(1, (frame * (frames + 1) + 0xC).next_multiple_of(0x100), 1);
    }
    Ok(size.next_multiple_of(0x1000))
}

// libnx audren.h AudioRendererMemPoolState
const REQUEST_DETACH: u32 = 2;
const DETACHED: u32 = 3;
const REQUEST_ATTACH: u32 = 4;
const ATTACHED: u32 = 5;

#[derive(Default)]
pub struct Pools {
    entries: Vec<(u64, u64, bool)>,
    force_mapping: bool,
    pub errors: Vec<(Error, u64)>,
}

impl Pools {
    /// whether `size` bytes at `address` lie in one attached pool; misses become error infos.
    pub fn attach(&mut self, address: u64, size: u64) -> bool {
        let inside = self.entries.iter().any(|&(base, len, attached)| {
            attached && address >= base && address.saturating_add(size) <= base + len
        });
        if !inside {
            self.errors.push((Error::NoMemoryPoolEntry, address));
        }
        inside || self.force_mapping
    }

    /// applies a 0x20-byte MemoryPoolInParameter; pools are page-aligned (libnx AUDREN_MEMPOOL_ALIGNMENT).
    fn update(&mut self, i: usize, b: &[u8], out: &mut [u8]) -> Result<(), Error> {
        let (address, size, state) = (u64_at(b, 0), u64_at(b, 8), u32_at(b, 0x10));
        if state != REQUEST_ATTACH && state != REQUEST_DETACH {
            return Ok(());
        }
        if address == 0 || address % 0x1000 != 0 || size == 0 || size % 0x1000 != 0 {
            return Err(Error::InvalidUpdateInfo);
        }
        let entry = &mut self.entries[i];
        if state == REQUEST_DETACH && (entry.0, entry.1) != (address, size) {
            return Err(Error::InvalidUpdateInfo);
        }
        *entry = (address, size, state == REQUEST_ATTACH);
        put(
            out,
            0,
            &if state == REQUEST_ATTACH {
                ATTACHED
            } else {
                DETACHED
            }
            .to_le_bytes(),
        );
        Ok(())
    }
}

pub struct Renderer {
    params: AudioRendererParameter,
    rev: u32,
    pools: Pools,
    resources: Vec<ChannelResource>,
    voices: Vec<Voice>,
    effects: Vec<Effect>,
    graph: Graph,
    sinks: Vec<Sink>,
    buffers: Vec<Vec<f32>>,
    depop: Vec<Depop>,
    device: Vec<i16>,
    started: bool,
    rendered: u64,
    signalled: u64,
}

impl Renderer {
    pub fn open(params: AudioRendererParameter, work_buffer: &mut [u8]) -> Result<Self, Error> {
        let rev = revision(params.revision)?;
        if !matches!(
            (params.sample_rate, params.sample_count),
            (48_000, 240) | (32_000, 160)
        ) {
            return Err(Error::InvalidSampleRate);
        }
        if params.mix_buffer_count == 0 || (work_buffer.len() as u64) < work_buffer_size(&params)? {
            return Err(Error::InsufficientBuffer);
        }
        let voices = params.voice_count as usize;
        let splitters = if rev >= REV_SPLITTER {
            params.splitter_count as usize
        } else {
            0
        };
        Ok(Self {
            params,
            rev,
            pools: Pools {
                entries: vec![(0, 0, false); params.effect_count as usize + voices * 4],
                ..Pools::default()
            },
            resources: vec![
                ChannelResource {
                    mix: [0.0; 24],
                    previous: [0.0; 24]
                };
                voices
            ],
            voices: (0..voices).map(|_| Voice::default()).collect(),
            effects: (0..params.effect_count)
                .map(|_| Effect::default())
                .collect(),
            graph: Graph::new(
                params.sub_mix_count as usize + 1,
                splitters,
                params.splitter_destination_count as usize,
            ),
            sinks: (0..params.sink_count).map(|_| Sink::Unused).collect(),
            buffers: vec![
                vec![0.0; params.sample_count as usize];
                params.mix_buffer_count as usize
            ],
            depop: (0..params.mix_buffer_count)
                .map(|_| {
                    Depop::new(DepopParams {
                        sample_rate: params.sample_rate,
                    })
                })
                .collect(),
            device: vec![0; 480],
            started: false,
            rendered: 0,
            signalled: 0,
        })
    }

    pub fn start(&mut self) {
        self.started = true;
    }

    pub fn stop(&mut self) {
        self.started = false;
    }

    /// periods rendered since open, the elapsed frame count games read back.
    pub fn rendered_updates(&self) -> u64 {
        self.rendered
    }

    /// whether a period rendered since the last poll, which the service's system event signals.
    pub fn pending_event(&mut self) -> bool {
        let pending = self.signalled != self.rendered;
        self.signalled = self.rendered;
        pending
    }

    pub fn request_update(&mut self, input: &[u8], output: &mut [u8]) -> Result<(), Error> {
        let (rev, voice_count) = (self.rev, self.voices.len());
        let mut r = Reader::new(input);
        let header = r.take(HEADER)?;
        let size = |slot: usize| u32_at(header, slot);
        let mut w = Writer::new(output, self.params.revision)?;
        let behaviour = r.take(BEHAVIOUR)?;
        if size(0x4) != BEHAVIOUR as u32 || u32_at(behaviour, 0) != self.params.revision {
            return Err(Error::InvalidUpdateInfo);
        }
        self.pools.force_mapping = u64_at(behaviour, 8) & 1 != 0;
        self.pools.errors.clear();
        for (i, b) in r
            .records(size(0x8), self.pools.entries.len(), POOL_IN)?
            .enumerate()
        {
            self.pools.update(i, b, w.record(0x8, POOL_OUT)?)?;
        }
        for (b, resource) in r
            .records(size(0x10), voice_count, CHANNEL_IN)?
            .zip(&mut self.resources)
        {
            resource.mix = f32s(b, 4);
        }
        for (b, voice) in r
            .records(size(0xC), voice_count, VOICE_IN)?
            .zip(&mut self.voices)
        {
            voice.update(
                b,
                w.record(0xC, VOICE_OUT)?,
                rev,
                voice_count,
                &mut self.pools,
            )?;
            for &resource in voice.resources[..voice.channel_count]
                .iter()
                .filter(|_| voice.new)
            {
                self.resources[resource].previous = self.resources[resource].mix;
            }
        }
        let effect_out = if rev >= REV_EFFECT_INFO_V2 {
            EFFECT_OUT_V2
        } else {
            EFFECT_OUT_V1
        };
        for (b, effect) in r
            .records(size(0x14), self.effects.len(), EFFECT_IN)?
            .zip(&mut self.effects)
        {
            effect.update(
                b,
                w.record(0x14, effect_out)?,
                rev,
                self.params.sample_rate,
                &mut self.pools,
            );
        }
        if rev >= REV_SPLITTER {
            self.graph.update_splitters(&mut r, rev)?;
        }
        self.graph
            .update_mixes(&mut r, size(0x18), rev, self.buffers.len(), &self.effects)?;
        for (b, sink) in r
            .records(size(0x1C), self.sinks.len(), SINK_IN)?
            .zip(&mut self.sinks)
        {
            sink.update(b, w.record(0x1C, SINK_OUT)?, &mut self.pools);
        }
        drop(r.records(size(0x20), 1, PERFORMANCE)?);
        w.record(0x20, PERFORMANCE)?;
        let errors = w.record(0x4, ERROR_INFO_OUT)?;
        for (i, &(error, extra)) in self.pools.errors.iter().take(10).enumerate() {
            put(errors, i * 0x10, &error.result().to_le_bytes());
            put(errors, i * 0x10 + 8, &extra.to_le_bytes());
        }
        put(
            errors,
            0xA0,
            &(self.pools.errors.len().min(10) as u32).to_le_bytes(),
        );
        if rev >= REV_ELAPSED_FRAMES {
            put(
                w.record(0x28, RENDER_INFO_OUT)?,
                0,
                &self.rendered.to_le_bytes(),
            );
        }
        if r.pos != size(0x3C) as usize {
            return Err(Error::InvalidUpdateInfo);
        }
        Ok(())
    }

    /// renders one period and returns it as 48 kHz interleaved stereo for the sink ring, empty while stopped.
    pub fn step(&mut self, memory: &mut dyn GuestMemory) -> &[i16] {
        if !self.started {
            return &[];
        }
        let (rate, frames) = (self.params.sample_rate, self.params.sample_count as usize);
        self.buffers.iter_mut().for_each(|buffer| buffer.fill(0.0));
        self.device.fill(0);
        for voice in &mut self.voices {
            let stopped = match voice.render(&*memory, rate, frames) {
                Rendered::Silent => continue,
                Rendered::Stopped => true,
                Rendered::Playing => false,
            };
            self.graph.mix_voice(
                voice,
                &self.resources,
                &mut self.buffers,
                &mut self.depop,
                stopped,
            );
        }
        self.resources
            .iter_mut()
            .for_each(|resource| resource.previous = resource.mix);
        self.graph.finish_ramps();
        self.graph.render(
            &mut self.effects,
            &mut self.buffers,
            &mut self.depop,
            memory,
        );
        for sink in &mut self.sinks {
            sink.render(&self.buffers, rate, &mut self.device, memory);
        }
        self.rendered += 1;
        &self.device
    }
}
