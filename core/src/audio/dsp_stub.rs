//! stand-in for the dsp kernels until audio/dsp lands, matching their interface.

pub mod adpcm {
    #[derive(Clone, Copy)]
    pub struct AdpcmParams {
        pub coefficients: [i16; 16],
    }

    #[derive(Clone, Copy, Default)]
    pub struct AdpcmContext {
        pub predictor_scale: u8,
        pub history_sample_1: i16,
        pub history_sample_2: i16,
    }

    pub struct GcAdpcmDecoder {
        params: AdpcmParams,
        context: AdpcmContext,
    }

    impl GcAdpcmDecoder {
        pub fn new(params: AdpcmParams) -> Self {
            Self {
                params,
                context: AdpcmContext::default(),
            }
        }
        pub fn set(&mut self, params: AdpcmParams) {
            self.params = params;
        }
        pub fn restore(&mut self, context: AdpcmContext) {
            self.context = context;
        }
        pub fn process(&mut self, input: &[u8], output: &mut [f32], samples: usize) -> usize {
            let mut written = 0;
            for frame in input.chunks_exact(8) {
                let predictor = usize::from(frame[0] >> 4) * 2;
                let (c1, c2) = (
                    i32::from(self.params.coefficients[predictor]),
                    i32::from(self.params.coefficients[predictor + 1]),
                );
                for i in 0..14 {
                    if written == samples.min(output.len()) {
                        return written;
                    }
                    let nibble =
                        i32::from(((frame[1 + i / 2] >> (4 - (i & 1) * 4)) << 4) as i8 >> 4);
                    let history = c1 * i32::from(self.context.history_sample_1)
                        + c2 * i32::from(self.context.history_sample_2);
                    let sample = (((nibble << (frame[0] & 0xF)) << 11) + history + 1024 >> 11)
                        .clamp(-32768, 32767) as i16;
                    (self.context.history_sample_2, self.context.history_sample_1) =
                        (self.context.history_sample_1, sample);
                    output[written] = f32::from(sample) / 32768.0;
                    written += 1;
                }
            }
            written
        }
    }
}

pub mod biquad {
    #[derive(Clone, Copy)]
    pub struct BiquadParams {
        pub b0: f32,
        pub b1: f32,
        pub b2: f32,
        pub a1: f32,
        pub a2: f32,
        pub new_state: bool,
    }

    pub struct Biquad {
        params: BiquadParams,
        z: [f32; 2],
    }

    impl Biquad {
        pub fn new(params: BiquadParams) -> Self {
            Self {
                params,
                z: [0.0; 2],
            }
        }
        pub fn set(&mut self, params: BiquadParams) {
            if params.new_state {
                self.z = [0.0; 2];
            }
            self.params = params;
        }
        pub fn process(&mut self, input: &[f32], output: &mut [f32], frames: usize) {
            let p = self.params;
            for (&x, y) in input.iter().zip(output).take(frames) {
                *y = p.b0 * x + self.z[0];
                self.z = [p.b1 * x - p.a1 * *y + self.z[1], p.b2 * x - p.a2 * *y];
            }
        }
    }
}

pub mod resampler {
    #[derive(Clone, Copy)]
    pub enum SrcQuality {
        Low,
        Medium,
        High,
    }

    #[derive(Clone, Copy)]
    pub struct ResamplerParams {
        pub sample_rate: u32,
        pub pitch: f32,
        pub quality: SrcQuality,
    }

    pub struct Resampler {
        params: ResamplerParams,
        phase: f64,
    }

    impl Resampler {
        pub fn new(params: ResamplerParams) -> Self {
            Self { params, phase: 0.0 }
        }
        pub fn set(&mut self, params: ResamplerParams) {
            self.params = params;
        }
        pub fn process(&mut self, input: &[f32], output: &mut [f32]) -> usize {
            let step = f64::from(self.params.sample_rate) / 48_000.0 * f64::from(self.params.pitch);
            let mut written = 0;
            while written < output.len() && (self.phase as usize) + 1 < input.len() {
                let (i, fraction) = (self.phase as usize, (self.phase.fract()) as f32);
                output[written] = input[i] + (input[i + 1] - input[i]) * fraction;
                self.phase += step;
                written += 1;
            }
            let consumed = (self.phase as usize).min(input.len());
            self.phase -= consumed as f64;
            consumed
        }
    }
}

pub mod volume {
    #[derive(Clone, Copy)]
    pub struct VolumeRampParams {
        pub previous_volume: f32,
        pub volume: f32,
    }

    pub struct VolumeRamp {
        params: VolumeRampParams,
    }

    impl VolumeRamp {
        pub fn new(params: VolumeRampParams) -> Self {
            Self { params }
        }
        pub fn process(&mut self, input: &[f32], output: &mut [f32], frames: usize) {
            let step =
                (self.params.volume - self.params.previous_volume) / (frames.max(2) - 1) as f32;
            for (i, (&x, y)) in input.iter().zip(output).take(frames).enumerate() {
                *y = x * (self.params.previous_volume + step * i as f32);
            }
        }
    }
}

pub mod depop {
    #[derive(Clone, Copy)]
    pub struct DepopParams {
        pub sample_rate: u32,
    }

    pub struct Depop {
        value: f32,
    }

    impl Depop {
        pub fn new(_: DepopParams) -> Self {
            Self { value: 0.0 }
        }
        pub fn trigger(&mut self, last_sample: f32) {
            self.value += last_sample;
        }
        pub fn process(&mut self, output: &mut [f32], frames: usize) {
            for sample in output.iter_mut().take(frames) {
                self.value *= 0.96;
                *sample += self.value;
            }
        }
    }
}

pub mod delay {
    #[derive(Clone, Copy)]
    pub struct DelayParams {
        pub channel_count: usize,
        pub sample_rate: u32,
        pub delay_time_max_ms: u32,
        pub delay_time_ms: u32,
        pub in_gain: f32,
        pub feedback_gain: f32,
        pub out_gain: f32,
        pub dry_gain: f32,
        pub channel_spread: f32,
        pub low_pass_amount: f32,
    }

    pub struct Delay {
        params: DelayParams,
        lines: Vec<Vec<f32>>,
        position: usize,
    }

    impl Delay {
        pub fn new(params: DelayParams) -> Self {
            let len = (params.sample_rate * params.delay_time_ms / 1000).max(1) as usize;
            Self {
                params,
                lines: vec![vec![0.0; len]; params.channel_count],
                position: 0,
            }
        }
        pub fn set(&mut self, params: DelayParams) {
            self.params = params;
        }
        pub fn process(&mut self, input: &[&[f32]], output: &mut [&mut [f32]], frames: usize) {
            let p = self.params;
            for f in 0..frames {
                for (c, line) in self
                    .lines
                    .iter_mut()
                    .enumerate()
                    .take(input.len().min(output.len()))
                {
                    let delayed = line[self.position];
                    line[self.position] = input[c][f] * p.in_gain + delayed * p.feedback_gain;
                    output[c][f] = input[c][f] * p.dry_gain + delayed * p.out_gain;
                }
                self.position = (self.position + 1) % self.lines.first().map_or(1, Vec::len);
            }
        }
    }
}
