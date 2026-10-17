//! mix effects, run per mix in processing order on that mix's buffers: buffer mix, aux send and
//! return through guest rings, delay and biquad; the other types report their state and pass
//! their input through.

use super::dsp::biquad::{Biquad, BiquadParams};
use super::dsp::delay::{Delay, DelayParams};
use super::parcel::{f32_at, i32_at, u16_at, u32_at, u64_at};
use super::renderer::{Pools, REV_BIQUAD_STATE_CLEAR_FIX};
use super::voice::biquad_q14;
use super::{GuestMemory, mix_into};

const BUFFER_MIX: u8 = 1;
const AUX: u8 = 2;
const DELAY: u8 = 3;
const BIQUAD: u8 = 6;
const ENABLED: u8 = 3;
const DISABLED: u8 = 4;

enum Kernel {
    None,
    Biquads(Vec<Biquad>),
    Delay(Delay),
    Aux {
        send_info: u64,
        send: u64,
        return_info: u64,
        returned: u64,
    },
}

pub struct Effect {
    kind: u8,
    enabled: bool,
    pub mix_id: i32,
    pub order: u32,
    params: Vec<u8>,
    kernel: Kernel,
}

fn q14(value: u32) -> f32 {
    value as f32 / 16384.0
}

impl Default for Effect {
    fn default() -> Self {
        Self {
            kind: 0,
            enabled: false,
            mix_id: -1,
            order: 0,
            params: vec![0; 0xA0],
            kernel: Kernel::None,
        }
    }
}

impl Effect {
    /// applies a 0xC0-byte EffectInParameter and writes the state byte of its out status.
    pub fn update(&mut self, b: &[u8], out: &mut [u8], rev: u32, rate: u32, pools: &mut Pools) {
        if b[0] != self.kind || b[1] != 0 {
            *self = Self {
                kind: b[0],
                ..Self::default()
            };
        }
        self.enabled = b[2] != 0;
        self.mix_id = i32_at(b, 4);
        self.order = u32_at(b, 0x18);
        self.params.copy_from_slice(&b[0x20..0xC0]);
        let p = &self.params;
        match self.kind {
            BIQUAD => {
                let i16_at = |at| u16_at(p, at) as i16;
                let status = p[0x17];
                let params = BiquadParams {
                    new_state: status == 0 || (status == 1 && rev < REV_BIQUAD_STATE_CLEAR_FIX),
                    ..biquad_q14(
                        [i16_at(0xC), i16_at(0xE), i16_at(0x10)],
                        [i16_at(0x12), i16_at(0x14)],
                    )
                };
                match &mut self.kernel {
                    Kernel::Biquads(filters) => {
                        filters.iter_mut().for_each(|filter| filter.set(params))
                    }
                    _ => {
                        self.kernel = Kernel::Biquads((0..6).map(|_| Biquad::new(params)).collect())
                    }
                }
            }
            DELAY => {
                let params = DelayParams {
                    channel_count: usize::from(u16_at(p, 0xE)),
                    sample_rate: rate,
                    delay_time_max_ms: u32_at(p, 0x10),
                    delay_time_ms: u32_at(p, 0x14),
                    in_gain: q14(u32_at(p, 0x1C)),
                    feedback_gain: q14(u32_at(p, 0x20)),
                    out_gain: q14(u32_at(p, 0x24)),
                    dry_gain: q14(u32_at(p, 0x28)),
                    channel_spread: q14(u32_at(p, 0x2C)),
                    low_pass_amount: q14(u32_at(p, 0x30)),
                };
                match &mut self.kernel {
                    Kernel::Delay(delay) => delay.set(params),
                    _ => {
                        self.kernel = Kernel::Delay(Delay::new(DelayParams {
                            channel_count: usize::from(u16_at(p, 0xC)).max(params.channel_count),
                            ..params
                        }))
                    }
                }
            }
            AUX => {
                let ring = u64::from(u32_at(p, 0x38)) * 4;
                let (send_info, send, return_info, returned) = (
                    u64_at(p, 0x40),
                    u64_at(p, 0x48),
                    u64_at(p, 0x50),
                    u64_at(p, 0x58),
                );
                let mapped = pools.attach(send_info, 0x80)
                    && pools.attach(send, ring)
                    && pools.attach(return_info, 0x80)
                    && pools.attach(returned, ring);
                self.kernel = if mapped {
                    Kernel::Aux {
                        send_info,
                        send,
                        return_info,
                        returned,
                    }
                } else {
                    Kernel::None
                };
            }
            _ => {}
        }
        out[0] = self.state();
    }

    fn state(&self) -> u8 {
        if self.enabled { ENABLED } else { DISABLED }
    }

    pub fn process(
        &mut self,
        offset: usize,
        buffers: &mut [Vec<f32>],
        memory: &mut dyn GuestMemory,
    ) {
        let p = &self.params;
        let frames = buffers.first().map_or(0, Vec::len);
        let (wide, channels) = match self.kind {
            BUFFER_MIX => (true, u32_at(p, 0x90) as usize),
            AUX => (true, u32_at(p, 0x30) as usize),
            BIQUAD => (false, usize::from(p[0x16])),
            _ => (false, usize::from(u16_at(p, 0xE))),
        };
        let (inputs, outputs) = if wide {
            (&p[..24], &p[0x18..0x30])
        } else {
            (&p[..6], &p[6..12])
        };
        let channels = channels.min(inputs.len());
        let index = |slot: &[u8], c: usize| offset + usize::from(slot[c]);
        if (0..channels).any(|c| index(inputs, c).max(index(outputs, c)) >= buffers.len()) {
            return;
        }
        if self.kind == BUFFER_MIX {
            for c in (0..channels).filter(|_| self.enabled) {
                mix_into(
                    buffers,
                    index(inputs, c),
                    index(outputs, c),
                    f32_at(p, 0x30 + c * 4),
                );
            }
            return;
        }
        let mut dry: Vec<Vec<f32>> = (0..channels)
            .map(|c| buffers[index(inputs, c)].clone())
            .collect();
        let mut wet = dry.clone();
        match &mut self.kernel {
            Kernel::Biquads(filters) if self.enabled => {
                for (c, filter) in filters.iter_mut().enumerate().take(channels) {
                    filter.process(&dry[c], &mut wet[c], frames);
                }
            }
            Kernel::Delay(delay) if self.enabled => {
                let input: Vec<&[f32]> = dry.iter().map(Vec::as_slice).collect();
                let mut output: Vec<&mut [f32]> = wet.iter_mut().map(Vec::as_mut_slice).collect();
                delay.process(&input, &mut output, frames);
            }
            Kernel::Aux {
                send_info,
                send,
                return_info,
                returned,
            } if self.enabled => {
                let count = u32_at(p, 0x38) as usize;
                let write = read_u32(memory, *send_info + 4) as usize;
                let read = read_u32(memory, *return_info) as usize;
                for c in 0..channels {
                    ring(memory, *send, count, write + c * frames, &mut dry[c], true);
                    ring(
                        memory,
                        *returned,
                        count,
                        read + c * frames,
                        &mut wet[c],
                        false,
                    );
                }
                let advance = |at: usize| ((at + channels * frames) % count.max(1)) as u32;
                memory.write(*send_info + 4, &advance(write).to_le_bytes());
                memory.write(*return_info, &advance(read).to_le_bytes());
            }
            _ => {}
        }
        for c in 0..channels {
            buffers[index(outputs, c)].copy_from_slice(&wet[c]);
        }
    }
}

fn read_u32(memory: &dyn GuestMemory, va: u64) -> u32 {
    let mut value = [0; 4];
    memory.read(va, &mut value);
    u32::from_le_bytes(value)
}

/// moves `samples` to or from a guest ring of `count` i32 samples, starting at sample `at`.
fn ring(
    memory: &mut dyn GuestMemory,
    base: u64,
    count: usize,
    at: usize,
    samples: &mut [f32],
    write: bool,
) {
    for (i, sample) in samples.iter_mut().enumerate().take(count) {
        let va = base + ((at + i) % count * 4) as u64;
        if write {
            memory.write(va, &((*sample * 32768.0) as i32).to_le_bytes());
        } else {
            *sample = read_u32(memory, va) as i32 as f32 / 32768.0;
        }
    }
}
