mod audout;
mod renderer;

use crate::audio::HostMemory;
use crate::audio::renderer::{AudioRendererParameter, Renderer};

pub const BASE: u64 = 0x10000;

pub fn magic(revision: u32) -> u32 {
    0x56_4552 | (0x30 + revision) << 24
}

pub fn record(len: usize, fields: &[(usize, &[u8])]) -> Vec<u8> {
    let mut record = vec![0; len];
    for &(at, value) in fields {
        record[at..at + value.len()].copy_from_slice(value);
    }
    record
}

pub fn parameter(revision: u32, effects: u32) -> AudioRendererParameter {
    AudioRendererParameter {
        sample_rate: 48_000,
        sample_count: 240,
        mix_buffer_count: 2,
        voice_count: 1,
        sink_count: 1,
        effect_count: effects,
        revision: magic(revision),
        ..Default::default()
    }
}

/// a 48 kHz mono voice on wave buffer 0, routed to both final-mix buffers at `mix` volumes.
pub struct Scene {
    pub revision: u32,
    pub format: u8,
    pub samples: u32,
    pub looping: bool,
    pub coefficients: Option<u64>,
    pub mix: [f32; 2],
    pub effects: Vec<Vec<u8>>,
}

impl Scene {
    pub fn parcel(&self, new: bool) -> Vec<u8> {
        let pools = [record(
            0x20,
            &[
                (0, &BASE.to_le_bytes()),
                (8, &0x10000u64.to_le_bytes()),
                (0x10, &4u32.to_le_bytes()),
            ],
        )];
        let pools = [&pools[..], &vec![vec![0; 0x20]; self.effects.len() + 3]]
            .concat()
            .concat();
        let mix = [self.mix[0].to_le_bytes(), self.mix[1].to_le_bytes()].concat();
        let channel = record(0x70, &[(4, &mix), (0x64, &[1])]);
        let (address, size) = self.coefficients.map_or((0, 0), |at| (at, 0x20));
        let wave = record(
            0x38,
            &[
                (0, &BASE.to_le_bytes()),
                (8, &0x8000u64.to_le_bytes()),
                (0x14, &self.samples.to_le_bytes()),
                (0x18, &[u8::from(self.looping)]),
                (0x1A, &[u8::from(!new)]),
            ],
        );
        let mut voice = record(
            0x170,
            &[
                (8, &[u8::from(new), 1, 0, self.format]),
                (0xC, &48_000u32.to_le_bytes()),
                (0x18, &1u32.to_le_bytes()),
                (0x1C, &1f32.to_le_bytes()),
                (0x20, &1f32.to_le_bytes()),
                (0x48, &address.to_le_bytes()),
                (0x50, &(size as u64).to_le_bytes()),
                (0x5C, &u32::MAX.to_le_bytes()),
                (0x60, &wave),
            ],
        );
        if self.revision >= 15 {
            voice.splice(0x3C..0x3C, [0; 0x18]);
        }
        let final_mix = record(
            0x930,
            &[
                (0, &1f32.to_le_bytes()),
                (4, &48_000u32.to_le_bytes()),
                (8, &2u32.to_le_bytes()),
                (0xC, &[1]),
                (0x924, &i32::MAX.to_le_bytes()),
                (0x928, &u32::MAX.to_le_bytes()),
            ],
        );
        let mixes = if self.revision >= 7 {
            [record(0x20, &[(4, &1u32.to_le_bytes())]), final_mix].concat()
        } else {
            final_mix
        };
        let sink = record(
            0x140,
            &[(0, &[1, 1]), (0x120, &2u32.to_le_bytes()), (0x124, &[0, 1])],
        );
        let sections = [
            record(0x10, &[(0, &magic(self.revision).to_le_bytes())]),
            pools,
            channel,
            voice,
            self.effects.concat(),
            mixes,
            sink,
            vec![0; 0x10],
        ];
        let sizes = [
            0x10,
            sections[1].len(),
            sections[3].len(),
            sections[2].len(),
            sections[4].len(),
            sections[5].len(),
            sections[6].len(),
            0x10,
        ];
        let mut header = vec![0; 0x40];
        header[..4].copy_from_slice(&magic(self.revision).to_le_bytes());
        for (i, size) in sizes.iter().enumerate() {
            header[4 + i * 4..8 + i * 4].copy_from_slice(&(*size as u32).to_le_bytes());
        }
        let total = 0x40 + sections.iter().map(Vec::len).sum::<usize>();
        header[0x3C..].copy_from_slice(&(total as u32).to_le_bytes());
        [header, sections.concat()].concat()
    }

    /// opens a renderer, sends the first parcel and renders `periods`, returning the device output and the update's output parcel.
    pub fn play(&self, memory: &mut HostMemory, periods: usize) -> (Renderer, Vec<i16>, Vec<u8>) {
        let parameter = parameter(self.revision, self.effects.len() as u32);
        let mut work =
            vec![0; crate::audio::renderer::work_buffer_size(&parameter).unwrap() as usize];
        let mut renderer = Renderer::open(parameter, &mut work).unwrap();
        let mut output = vec![0; 0x1000];
        renderer
            .request_update(&self.parcel(true), &mut output)
            .unwrap();
        renderer.start();
        let samples = (0..periods)
            .flat_map(|_| renderer.step(memory).to_vec())
            .collect();
        (renderer, samples, output)
    }
}

pub fn memory(data: &[u8]) -> HostMemory {
    let mut bytes = vec![0; 0x10000];
    bytes[..data.len()].copy_from_slice(data);
    HostMemory { base: BASE, bytes }
}
