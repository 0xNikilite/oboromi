//! sinks: the device sink folds the final mix's input channels to the host's interleaved stereo
//! i16, upsampling a 32 kHz renderer to 48 kHz; the circular-buffer sink writes each input as a
//! run of i16 samples into a guest ring.

use super::GuestMemory;
use super::dsp::upsampler::{Upsampler, UpsamplerParams};
use super::parcel::{f32s, put, u32_at, u64_at};
use super::renderer::Pools;

// libnx audren.h AudioRendererSinkType
const DEVICE: u8 = 1;
const CIRCULAR_BUFFER: u8 = 2;
/// front, center, lfe and back weights folding 5.1 into stereo, ITU-R BS.775 without the lfe.
pub const DEFAULT_DOWNMIX: [f32; 4] = [
    1.0,
    std::f32::consts::FRAC_1_SQRT_2,
    0.0,
    std::f32::consts::FRAC_1_SQRT_2,
];

pub enum Sink {
    Unused,
    Device {
        inputs: Vec<usize>,
        downmix: [f32; 4],
        upsamplers: [Upsampler; 2],
    },
    Circular {
        address: u64,
        size: u32,
        inputs: Vec<usize>,
        written: u32,
        last_written: u32,
    },
}

/// one frame of `channels` (front left, front right, center, lfe, back left, back right) as stereo.
pub fn stereo(channels: &[f32], downmix: [f32; 4]) -> [f32; 2] {
    match *channels {
        [mono] => [mono, mono],
        [left, right, center, lfe, back_left, back_right] => {
            let shared = center * downmix[1] + lfe * downmix[2];
            [
                left * downmix[0] + shared + back_left * downmix[3],
                right * downmix[0] + shared + back_right * downmix[3],
            ]
        }
        _ => [
            channels.first().copied().unwrap_or(0.0),
            channels.get(1).copied().unwrap_or(0.0),
        ],
    }
}

pub fn to_i16(sample: f32) -> i16 {
    (sample * 32768.0).round() as i16
}

impl Sink {
    /// applies a 0x140-byte SinkInParameter and writes its 0x20-byte SinkOutStatus.
    pub fn update(&mut self, b: &[u8], out: &mut [u8], pools: &mut Pools) {
        let inputs = |count_at: usize, inputs_at: usize| {
            b[inputs_at..inputs_at + (u32_at(b, count_at) as usize).min(6)]
                .iter()
                .map(|&i| usize::from(i))
                .collect()
        };
        match (b[0], b[1] != 0, &mut *self) {
            (
                DEVICE,
                true,
                Sink::Device {
                    inputs: current,
                    downmix,
                    ..
                },
            ) => {
                *current = inputs(0x120, 0x124);
                *downmix = if b[0x12B] != 0 {
                    f32s(b, 0x12C)
                } else {
                    DEFAULT_DOWNMIX
                };
            }
            (DEVICE, true, _) => {
                let upsampler = || {
                    Upsampler::new(UpsamplerParams {
                        input_sample_rate: 32_000,
                    })
                };
                *self = Sink::Device {
                    inputs: Vec::new(),
                    downmix: DEFAULT_DOWNMIX,
                    upsamplers: [upsampler(), upsampler()],
                };
                return self.update(b, out, pools);
            }
            (CIRCULAR_BUFFER, true, Sink::Circular { .. }) => {}
            (CIRCULAR_BUFFER, true, _) => {
                let (address, size) = (u64_at(b, 0x20), u32_at(b, 0x28));
                *self = if pools.attach(address, u64::from(size)) {
                    Sink::Circular {
                        address,
                        size,
                        inputs: inputs(0x2C, 0x3C),
                        written: 0,
                        last_written: 0,
                    }
                } else {
                    Sink::Unused
                };
            }
            _ => *self = Sink::Unused,
        }
        if let Sink::Circular { last_written, .. } = self {
            put(out, 0, &last_written.to_le_bytes());
        }
    }

    /// adds this period of the final mix into `device` (48 kHz interleaved stereo), or writes it to the guest ring.
    pub fn render(
        &mut self,
        buffers: &[Vec<f32>],
        rate: u32,
        device: &mut [i16],
        memory: &mut dyn GuestMemory,
    ) {
        let frames = buffers.first().map_or(0, Vec::len);
        let valid = |inputs: &[usize]| inputs.iter().all(|&i| i < buffers.len());
        match self {
            Sink::Device {
                inputs,
                downmix,
                upsamplers,
            } if valid(inputs) => {
                let mut channels = [0.0; 6];
                let mut folded = [[0.0; 240]; 2];
                for f in 0..frames {
                    inputs
                        .iter()
                        .zip(&mut channels)
                        .for_each(|(&i, c)| *c = buffers[i][f]);
                    let [left, right] = stereo(&channels[..inputs.len()], *downmix);
                    (folded[0][f], folded[1][f]) = (left, right);
                }
                if rate != 48_000 {
                    for (upsampler, channel) in upsamplers.iter_mut().zip(&mut folded) {
                        let input = std::mem::replace(channel, [0.0; 240]);
                        upsampler.process(&input[..frames], channel);
                    }
                }
                for (frame, out) in device.chunks_exact_mut(2).enumerate() {
                    out[0] = out[0].saturating_add(to_i16(folded[0][frame]));
                    out[1] = out[1].saturating_add(to_i16(folded[1][frame]));
                }
            }
            Sink::Circular {
                address,
                size,
                inputs,
                written,
                last_written,
            } if valid(inputs) => {
                *last_written = *written;
                for &i in inputs.iter() {
                    let samples: Vec<u8> = buffers[i]
                        .iter()
                        .flat_map(|&s| to_i16(s).to_le_bytes())
                        .collect();
                    if *written as usize + samples.len() > *size as usize {
                        *written = 0;
                    }
                    if samples.len() <= *size as usize {
                        memory.write(*address + u64::from(*written), &samples);
                        *written += samples.len() as u32;
                    }
                }
            }
            _ => {}
        }
    }
}
