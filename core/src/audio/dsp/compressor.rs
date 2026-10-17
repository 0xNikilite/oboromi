#[derive(Clone, Copy)]
pub struct CompressorParams {
    pub channel_count: usize,
    pub threshold: f32,
    pub ratio: f32,
    pub input_gain: f32,
    pub attack_coefficient: f32,
    pub release_coefficient: f32,
    pub output_gain: f32,
    pub makeup_gain_enabled: bool,
    pub statistics_enabled: bool,
    pub statistics_reset: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct CompressorStatistics {
    pub maximum_mean: f32,
    pub minimum_gain: f32,
    pub last_samples: [f32; 6],
}

impl Default for CompressorStatistics {
    fn default() -> Self {
        Self {
            maximum_mean: 0.0,
            minimum_gain: 1.0,
            last_samples: [0.0; 6],
        }
    }
}

pub struct Compressor {
    params: CompressorParams,
    envelope: f32,
    gain: f32,
    statistics: CompressorStatistics,
}

impl Compressor {
    pub fn new(params: CompressorParams) -> Self {
        Self {
            params,
            envelope: 0.0,
            gain: 1.0,
            statistics: CompressorStatistics::default(),
        }
    }

    pub fn set(&mut self, params: CompressorParams) {
        if params.statistics_reset {
            self.statistics = CompressorStatistics::default();
        }
        self.params = params;
    }

    pub fn statistics(&self) -> CompressorStatistics {
        self.statistics
    }

    pub fn process(&mut self, input: &[&[f32]], output: &mut [&mut [f32]], frames: usize) {
        let channels = self
            .params
            .channel_count
            .min(input.len())
            .min(output.len())
            .min(6);
        let makeup = if self.params.makeup_gain_enabled {
            self.params.threshold * 0.5 * (1.0 / self.params.ratio - 1.0) - 3.0
        } else {
            0.0
        };
        let output_gain = 10.0_f32.powf((self.params.output_gain + makeup) / 20.0);
        for frame in 0..frames {
            if (0..channels)
                .any(|channel| frame >= input[channel].len() || frame >= output[channel].len())
            {
                break;
            }
            let mean = (0..channels)
                .map(|channel| (input[channel][frame] * self.params.input_gain).powi(2))
                .sum::<f32>()
                / channels.max(1) as f32;
            let coefficient = if mean > self.envelope {
                self.params.attack_coefficient
            } else {
                self.params.release_coefficient
            };
            self.envelope += (mean - self.envelope) * coefficient;
            let level = 10.0 * self.envelope.max(1.0e-10).log10();
            let reduction = if level > self.params.threshold {
                (1.0 / self.params.ratio - 1.0) * (level - self.params.threshold)
            } else {
                0.0
            };
            let target = 10.0_f32.powf(reduction / 20.0);
            self.gain += (target - self.gain) * coefficient;
            let gain = self.gain * output_gain;
            for channel in 0..channels {
                let sample = input[channel][frame] * self.params.input_gain;
                output[channel][frame] = sample * gain;
                if self.params.statistics_enabled {
                    self.statistics.last_samples[channel] = sample.abs();
                }
            }
            if self.params.statistics_enabled {
                self.statistics.maximum_mean = self.statistics.maximum_mean.max(mean);
                self.statistics.minimum_gain = self.statistics.minimum_gain.min(gain);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratio_reduces_level_above_threshold_and_updates_statistics() {
        let params = CompressorParams {
            channel_count: 2,
            threshold: -12.0,
            ratio: 4.0,
            input_gain: 1.0,
            attack_coefficient: 1.0,
            release_coefficient: 1.0,
            output_gain: 0.0,
            makeup_gain_enabled: false,
            statistics_enabled: true,
            statistics_reset: false,
        };
        let input = [1.0; 4];
        let mut left = [0.0; 4];
        let mut right = [0.0; 4];
        let mut compressor = Compressor::new(params);
        compressor.process(&[&input, &input], &mut [&mut left, &mut right], 4);
        assert!((left[3] - 10.0_f32.powf(-9.0 / 20.0)).abs() < 0.0001);
        assert_eq!(left, right);
        let statistics = compressor.statistics();
        assert_eq!(statistics.maximum_mean, 1.0);
        assert_eq!(statistics.minimum_gain, left[3]);
        assert_eq!(statistics.last_samples[..2], [1.0, 1.0]);
    }
}
