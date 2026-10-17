use std::f32::consts::PI;

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
    history: [f32; 16],
}

impl Resampler {
    pub const LOW_TAPS: usize = 2;
    pub const MEDIUM_TAPS: usize = 4;
    pub const HIGH_TAPS: usize = 32;

    pub fn new(params: ResamplerParams) -> Self {
        Self {
            params,
            phase: 0.0,
            history: [0.0; 16],
        }
    }

    pub fn set(&mut self, params: ResamplerParams) {
        self.params = params;
    }

    pub fn process(&mut self, input: &[f32], output: &mut [f32]) -> usize {
        let step = f64::from(self.params.sample_rate) / 48_000.0 * f64::from(self.params.pitch);
        let lookahead = match self.params.quality {
            SrcQuality::Low => 1,
            SrcQuality::Medium => 2,
            SrcQuality::High => 16,
        };
        let mut written = 0;
        while written < output.len() && self.phase.floor() as usize + lookahead < input.len() {
            let center = self.phase.floor() as isize;
            let fraction = (self.phase - self.phase.floor()) as f32;
            output[written] = match self.params.quality {
                SrcQuality::Low => {
                    let a = self.sample(input, center);
                    a + (self.sample(input, center + 1) - a) * fraction
                }
                SrcQuality::Medium => cubic(
                    self.sample(input, center - 1),
                    self.sample(input, center),
                    self.sample(input, center + 1),
                    self.sample(input, center + 2),
                    fraction,
                ),
                SrcQuality::High => self.sinc(input, center, fraction),
            };
            self.phase += step;
            written += 1;
        }
        let consumed = (self.phase.floor() as usize).min(input.len());
        let mut history = [0.0; 16];
        for (index, sample) in history.iter_mut().enumerate() {
            *sample = self.sample(input, consumed as isize + index as isize - 16);
        }
        self.history = history;
        self.phase -= consumed as f64;
        consumed
    }

    fn sample(&self, input: &[f32], index: isize) -> f32 {
        if index >= 0 {
            input.get(index as usize).copied().unwrap_or(0.0)
        } else {
            self.history[(index + 16).clamp(0, 15) as usize]
        }
    }

    fn sinc(&self, input: &[f32], center: isize, fraction: f32) -> f32 {
        let cutoff = (48_000.0 / self.params.sample_rate as f32 / self.params.pitch).min(1.0);
        let mut sum = 0.0;
        let mut weight = 0.0;
        for tap in -15..=16 {
            let distance = tap as f32 - fraction;
            let x = PI * distance * cutoff;
            let kernel = if x.abs() < f32::EPSILON {
                cutoff
            } else {
                x.sin() / (PI * distance)
            };
            let window =
                0.42 + 0.5 * (PI * distance / 16.0).cos() + 0.08 * (PI * distance / 8.0).cos();
            let coefficient = kernel * window;
            sum += self.sample(input, center + tap) * coefficient;
            weight += coefficient;
        }
        sum / weight
    }
}

fn cubic(a: f32, b: f32, c: f32, d: f32, x: f32) -> f32 {
    b + 0.5 * x * (c - a + x * (2.0 * a - 5.0 * b + 4.0 * c - d + x * (3.0 * (b - c) + d - a)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    fn source(len: usize) -> Vec<f32> {
        (0..len)
            .map(|i| (TAU * 1000.0 * i as f32 / 44_100.0).sin())
            .collect()
    }

    fn high() -> Resampler {
        Resampler::new(ResamplerParams {
            sample_rate: 44_100,
            pitch: 1.0,
            quality: SrcQuality::High,
        })
    }

    #[test]
    fn high_quality_sine_error_is_below_minus_60_db() {
        let source = source(1000);
        let mut output = [0.0; 800];
        high().process(&source, &mut output);
        let error = output
            .iter()
            .enumerate()
            .skip(32)
            .map(|(i, &sample)| {
                let expected = (TAU * 1000.0 * i as f32 / 48_000.0).sin();
                (sample - expected).powi(2)
            })
            .sum::<f32>()
            / 768.0;
        assert!(10.0 * error.log10() < -60.0);
    }

    #[test]
    fn phase_is_continuous_across_updates() {
        let source = source(500);
        let mut whole = [0.0; 480];
        high().process(&source, &mut whole);
        let mut split = [0.0; 480];
        let mut resampler = high();
        let consumed = resampler.process(&source, &mut split[..240]);
        resampler.process(&source[consumed..], &mut split[240..]);
        assert_eq!(split, whole);
    }
}
