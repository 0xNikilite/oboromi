//! the mix graph: voices add into mix buffers, every submix runs its effects and sends onward
//! through its volume matrix or a splitter, sorted so each mix runs after everything feeding it,
//! and the final mix feeds the sinks.

use super::dsp::depop::Depop;
use super::dsp::volume::{VolumeRamp, VolumeRampParams};
use super::effect::Effect;
use super::parcel::{
    DESTINATION_V1, DESTINATION_V2, MIX_DIRTY_HEADER, MIX_IN, Reader, SPLITTER_HEADER, SPLITTER_IN,
    f32_at, f32s, i32_at, u32_at,
};
use super::renderer::{REV_DIRTY_MIXES, REV_SPLITTER_BIQUADS, REV_SPLITTER_PREVIOUS_VOLUME_RESET};
use super::voice::{ChannelResource, UNUSED_MIX, Voice};
use super::{Error, GuestMemory, mix_into};

#[derive(Clone, Default)]
pub struct Mix {
    pub used: bool,
    volume: f32,
    pub buffer_count: usize,
    pub offset: usize,
    volumes: Vec<[f32; 24]>,
    destination_mix: i32,
    destination_splitter: u32,
    effects: Vec<usize>,
}

#[derive(Clone, Copy, Default)]
pub struct Destination {
    mix_id: i32,
    volume: [f32; 24],
    previous: [f32; 24],
    used: bool,
}

pub struct Graph {
    pub mixes: Vec<Mix>,
    splitters: Vec<Vec<usize>>,
    destinations: Vec<Destination>,
    order: Vec<usize>,
}

impl Graph {
    pub fn new(mixes: usize, splitters: usize, destinations: usize) -> Self {
        Self {
            mixes: vec![Mix::default(); mixes],
            splitters: vec![Vec::new(); splitters],
            destinations: vec![Destination::default(); destinations],
            order: Vec::new(),
        }
    }

    /// the splitter section, present only when the renderer was opened with splitters.
    pub fn update_splitters(&mut self, reader: &mut Reader, rev: u32) -> Result<(), Error> {
        if self.splitters.is_empty() || self.destinations.is_empty() {
            return Ok(());
        }
        let header = reader.take(SPLITTER_HEADER)?;
        if &header[..4] != b"SNDH" {
            return Err(Error::InvalidUpdateInfo);
        }
        for _ in 0..u32_at(header, 4) {
            let info = reader.take(SPLITTER_IN)?;
            if &info[..4] != b"SNDI" {
                reader.pos -= SPLITTER_IN;
                break;
            }
            let ids = reader.take(u32_at(info, 0xC) as usize * 4)?;
            reader.take(0xC)?;
            let ids = ids
                .chunks_exact(4)
                .map(|id| i32_at(id, 0) as usize)
                .filter(|&id| id < self.destinations.len());
            if let Some(splitter) = self.splitters.get_mut(i32_at(info, 4) as usize) {
                *splitter = ids.collect();
            }
        }
        let (len, used_at) = if rev >= REV_SPLITTER_BIQUADS {
            (DESTINATION_V2, 0x84)
        } else {
            (DESTINATION_V1, 0x6C)
        };
        for _ in 0..u32_at(header, 8) {
            let b = reader.take(len)?;
            if &b[..4] != b"SNDD" {
                reader.pos -= len;
                break;
            }
            let Some(destination) = self.destinations.get_mut(i32_at(b, 4) as usize) else {
                continue;
            };
            let used = b[used_at] != 0;
            destination.mix_id = i32_at(b, 0x68);
            destination.volume = f32s(b, 8);
            let reset = if rev >= REV_SPLITTER_PREVIOUS_VOLUME_RESET {
                b[used_at + 1] != 0
            } else {
                used && !destination.used
            };
            if reset {
                destination.previous = destination.volume;
            }
            destination.used = used;
        }
        reader.pos = reader.pos.next_multiple_of(0x10);
        Ok(())
    }

    /// the mix section: every mix, or from REV7 a header and only the dirty mixes.
    pub fn update_mixes(
        &mut self,
        reader: &mut Reader,
        size: u32,
        rev: u32,
        mix_buffer_count: usize,
        effects: &[Effect],
    ) -> Result<(), Error> {
        let dirty = rev >= REV_DIRTY_MIXES;
        let (count, size) = if dirty {
            let header = reader.take(MIX_DIRTY_HEADER)?;
            (
                u32_at(header, 4) as usize,
                size.checked_sub(MIX_DIRTY_HEADER as u32)
                    .ok_or(Error::InvalidUpdateInfo)?,
            )
        } else {
            (self.mixes.len(), size)
        };
        for (i, b) in reader.records(size, count, MIX_IN)?.enumerate() {
            let id = if dirty { i32_at(b, 0x10) as usize } else { i };
            let destination_mix = i32_at(b, 0x924);
            let buffer_count = u32_at(b, 8) as usize;
            let valid_destination = id == 0
                || destination_mix == UNUSED_MIX
                || (destination_mix as usize) < self.mixes.len();
            if id >= self.mixes.len() || buffer_count > 24 || !valid_destination {
                return Err(Error::InvalidUpdateInfo);
            }
            let mix = &mut self.mixes[id];
            mix.used = b[0xC] != 0;
            mix.volume = f32_at(b, 0);
            mix.buffer_count = buffer_count;
            mix.volumes = (0..24).map(|row| f32s(b, 0x24 + row * 0x60)).collect();
            mix.destination_mix = destination_mix;
            mix.destination_splitter = u32_at(b, 0x928);
        }
        let mut offset = 0;
        for mix in self.mixes.iter_mut().filter(|mix| mix.used) {
            mix.offset = offset;
            offset += mix.buffer_count;
        }
        if offset > mix_buffer_count {
            return Err(Error::InvalidUpdateInfo);
        }
        for (id, mix) in self.mixes.iter_mut().enumerate() {
            mix.effects = (0..effects.len())
                .filter(|&e| effects[e].mix_id == id as i32)
                .collect();
            mix.effects.sort_by_key(|&e| effects[e].order);
        }
        self.sort()
    }

    /// the mixes `mix` sends to, with the splitter slot and destination when sent through one.
    fn targets(&self, mix: usize) -> Vec<(usize, Option<(usize, usize)>)> {
        let mix = &self.mixes[mix];
        if mix.destination_mix != UNUSED_MIX {
            return vec![(mix.destination_mix as usize, None)];
        }
        let splitter = self
            .splitters
            .get(mix.destination_splitter as usize)
            .map_or(&[][..], Vec::as_slice);
        let used = splitter
            .iter()
            .enumerate()
            .filter(|&(_, &d)| self.destinations[d].used);
        used.map(|(slot, &d)| (self.destinations[d].mix_id as usize, Some((slot, d))))
            .collect()
    }

    fn sort(&mut self) -> Result<(), Error> {
        fn visit(
            graph: &Graph,
            mix: usize,
            marks: &mut [u8],
            order: &mut Vec<usize>,
        ) -> Result<(), Error> {
            match marks[mix] {
                1 => return Err(Error::CycleDetected),
                2 => return Ok(()),
                _ => marks[mix] = 1,
            }
            if mix != 0 {
                for (target, _) in graph.targets(mix) {
                    if graph.mixes.get(target).is_some_and(|m| m.used) {
                        visit(graph, target, marks, order)?;
                    }
                }
            }
            marks[mix] = 2;
            order.push(mix);
            Ok(())
        }
        let (mut marks, mut order) = (vec![0; self.mixes.len()], Vec::new());
        for mix in (0..self.mixes.len()).filter(|&m| self.mixes[m].used) {
            visit(self, mix, &mut marks, &mut order)?;
        }
        order.reverse();
        self.order = order;
        Ok(())
    }

    /// adds a voice's channels into their destinations' buffers, or on a stop hands the last
    /// samples to the destinations' depop.
    pub fn mix_voice(
        &self,
        voice: &mut Voice,
        resources: &[ChannelResource],
        buffers: &mut [Vec<f32>],
        depop: &mut [Depop],
        stopped: bool,
    ) {
        let frames = buffers.first().map_or(0, Vec::len);
        for c in 0..voice.channel_count {
            let channel = &mut voice.channels[c];
            let resource = &resources[voice.resources[c]];
            let mut route = |mix: &Mix, volume: &[f32; 24], previous: &[f32; 24]| {
                for i in 0..mix.buffer_count {
                    if stopped {
                        depop[mix.offset + i].trigger(channel.last[i]);
                    } else if volume[i] != 0.0 || previous[i] != 0.0 {
                        let mut ramped = [0.0; 240];
                        VolumeRamp::new(VolumeRampParams {
                            previous_volume: previous[i],
                            volume: volume[i],
                        })
                        .process(&channel.output, &mut ramped, frames);
                        for (output, sample) in buffers[mix.offset + i].iter_mut().zip(&ramped) {
                            *output += sample;
                        }
                        channel.last[i] = ramped[frames - 1];
                    }
                }
            };
            if voice.mix_id != UNUSED_MIX {
                if let Some(mix) = self.mixes.get(voice.mix_id as usize).filter(|m| m.used) {
                    route(mix, &resource.mix, &resource.previous);
                }
            } else if let Some(splitter) = self.splitters.get(voice.splitter_id as usize) {
                for &d in splitter.iter().skip(c).step_by(voice.channel_count) {
                    let d = &self.destinations[d];
                    if let Some(mix) = self
                        .mixes
                        .get(d.mix_id as usize)
                        .filter(|m| d.used && m.used)
                    {
                        route(mix, &d.volume, &d.previous);
                    }
                }
            }
            if stopped {
                channel.last = [0.0; 24];
            }
        }
    }

    pub fn finish_ramps(&mut self) {
        for destination in &mut self.destinations {
            destination.previous = destination.volume;
        }
    }

    /// leaves to the final mix: depop, the mix's effects in processing order, then the send.
    pub fn render(
        &self,
        effects: &mut [Effect],
        buffers: &mut [Vec<f32>],
        depop: &mut [Depop],
        memory: &mut dyn GuestMemory,
    ) {
        let frames = buffers.first().map_or(0, Vec::len);
        for &m in &self.order {
            let mix = &self.mixes[m];
            for i in mix.offset..mix.offset + mix.buffer_count {
                depop[i].process(&mut buffers[i], frames);
            }
            for &e in &mix.effects {
                effects[e].process(mix.offset, buffers, memory);
            }
            if m == 0 {
                for buffer in &mut buffers[mix.offset..mix.offset + mix.buffer_count] {
                    buffer.iter_mut().for_each(|s| *s *= mix.volume);
                }
                continue;
            }
            for (target, splitter) in self.targets(m) {
                let Some(to) = self
                    .mixes
                    .get(target)
                    .filter(|to| to.used && mix.buffer_count > 0)
                else {
                    continue;
                };
                let mut send = |i: usize, j: usize, volume: f32| {
                    if volume != 0.0 {
                        mix_into(buffers, mix.offset + i, to.offset + j, mix.volume * volume);
                    }
                };
                for j in 0..to.buffer_count {
                    match splitter {
                        None => (0..mix.buffer_count).for_each(|i| send(i, j, mix.volumes[i][j])),
                        Some((slot, d)) => {
                            send(slot % mix.buffer_count, j, self.destinations[d].volume[j])
                        }
                    }
                }
            }
        }
    }
}
