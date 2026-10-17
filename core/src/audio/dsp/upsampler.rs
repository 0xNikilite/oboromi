use std::f32::consts::PI;

#[derive(Clone, Copy)]
pub struct UpsamplerParams {
    pub input_sample_rate: u32,
}

pub struct Upsampler {
    params: UpsamplerParams,
    history: [f32; 20],
    phase: f64,
    started: bool,
}

impl Upsampler {
    pub const TAPS: usize = 20;

    pub fn new(params: UpsamplerParams) -> Self {
        Self {
            params,
            history: [0.0; 20],
            phase: 0.0,
            started: false,
        }
    }

    pub fn set(&mut self, params: UpsamplerParams) {
        self.params = params;
    }

    pub fn process(&mut self, input: &[f32], output: &mut [f32]) -> usize {
        let step = f64::from(self.params.input_sample_rate) / 48_000.0;
        let mut consumed = 0;
        for result in output {
            if !self.started || self.phase >= 1.0 {
                if consumed == input.len() {
                    break;
                }
                if self.started {
                    self.phase -= 1.0;
                }
                self.history.copy_within(1.., 0);
                self.history[19] = input[consumed];
                consumed += 1;
                self.started = true;
            }
            *result = self.filter(self.phase as f32);
            self.phase += step;
        }
        consumed
    }

    fn filter(&self, offset: f32) -> f32 {
        if offset == 0.0 {
            return self.history[9];
        }
        self.history
            .iter()
            .enumerate()
            .map(|(index, &sample)| {
                let x = 9.0 - index as f32 + offset;
                let sinc = if x.abs() < f32::EPSILON {
                    1.0
                } else {
                    (PI * x).sin() / (PI * x)
                };
                let position = x / 20.0 + 0.5;
                let window =
                    0.42 - 0.5 * (2.0 * PI * position).cos() + 0.08 * (4.0 * PI * position).cos();
                sample * sinc * window
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_khz_mix_consumes_forty_samples_per_update() {
        let input: Vec<_> = (0..80).map(|sample| sample as f32).collect();
        let mut output = [0.0; 480];
        let mut whole = Upsampler::new(UpsamplerParams {
            input_sample_rate: 8000,
        });
        assert_eq!(whole.process(&input, &mut output[..240]), 40);
        assert_eq!(whole.process(&input[40..], &mut output[240..]), 40);

        let mut expected = [0.0; 480];
        Upsampler::new(UpsamplerParams {
            input_sample_rate: 8000,
        })
        .process(&input, &mut expected);
        assert_eq!(output, expected);
    }

    #[test]
    fn thirty_two_khz_phase_survives_update_boundary() {
        let input: Vec<_> = (0..400).map(|sample| (sample as f32 * 0.1).sin()).collect();
        let mut output = [0.0; 480];
        let mut split = Upsampler::new(UpsamplerParams {
            input_sample_rate: 32_000,
        });
        let consumed = split.process(&input, &mut output[..240]);
        split.process(&input[consumed..], &mut output[240..]);
        let mut expected = [0.0; 480];
        Upsampler::new(UpsamplerParams {
            input_sample_rate: 32_000,
        })
        .process(&input, &mut expected);
        assert_eq!(output, expected);
    }
}
