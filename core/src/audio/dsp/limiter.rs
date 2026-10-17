#[derive(Clone, Copy)]
pub struct LimiterParams {
    pub channel_count: usize,
    pub sample_rate: u32,
    pub look_ahead_time_max_us: u32,
    pub look_ahead_time_us: u32,
    pub attack_coefficient: f32,
    pub release_coefficient: f32,
    pub threshold: f32,
    pub input_gain: f32,
    pub output_gain: f32,
    pub statistics_enabled: bool,
    pub statistics_reset: bool,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct LimiterStatistics {
    pub processed_sample_count: u64,
    pub peak_input: f32,
    pub peak_output: f32,
    pub maximum_gain_reduction: f32,
}

pub struct Limiter {
    params: LimiterParams,
    delay: Vec<Vec<f32>>,
    positions: [usize; 6],
    gains: [f32; 6],
    look_ahead: usize,
    statistics: LimiterStatistics,
}

impl Limiter {
    pub fn new(params: LimiterParams) -> Self {
        let capacity = micros(params.sample_rate, params.look_ahead_time_max_us).max(1) + 1;
        let delay = (0..params.channel_count.min(6))
            .map(|_| vec![0.0; capacity])
            .collect();
        let mut effect = Self {
            params,
            delay,
            positions: [0; 6],
            gains: [1.0; 6],
            look_ahead: 1,
            statistics: LimiterStatistics::default(),
        };
        effect.set(params);
        effect
    }

    pub fn set(&mut self, params: LimiterParams) {
        if params.statistics_reset {
            self.statistics = LimiterStatistics::default();
        }
        self.params = params;
        self.look_ahead = micros(params.sample_rate, params.look_ahead_time_us)
            .clamp(1, self.delay.first().map_or(1, |line| line.len() - 1));
        for position in &mut self.positions {
            *position %= self.look_ahead;
        }
    }

    pub fn statistics(&self) -> LimiterStatistics {
        self.statistics
    }

    pub fn process(&mut self, input: &[&[f32]], output: &mut [&mut [f32]], frames: usize) {
        let channels = self.delay.len().min(input.len()).min(output.len());
        for channel in 0..channels {
            let count = frames.min(input[channel].len()).min(output[channel].len());
            for frame in 0..count {
                let sample = input[channel][frame] * self.params.input_gain;
                let delayed = self.delay[channel][self.positions[channel]];
                self.delay[channel][self.positions[channel]] = sample;
                self.positions[channel] = (self.positions[channel] + 1) % self.look_ahead;
                let target =
                    (self.params.threshold / sample.abs().max(self.params.threshold)).min(1.0);
                let coefficient = if target < self.gains[channel] {
                    self.params.attack_coefficient
                } else {
                    self.params.release_coefficient
                };
                self.gains[channel] += (target - self.gains[channel]) * coefficient;
                let limited = (delayed * self.gains[channel] * self.params.output_gain)
                    .clamp(-self.params.threshold, self.params.threshold);
                output[channel][frame] = limited;
                if self.params.statistics_enabled {
                    self.statistics.processed_sample_count += 1;
                    self.statistics.peak_input = self.statistics.peak_input.max(sample.abs());
                    self.statistics.peak_output = self.statistics.peak_output.max(limited.abs());
                    self.statistics.maximum_gain_reduction = self
                        .statistics
                        .maximum_gain_reduction
                        .max(1.0 - self.gains[channel]);
                }
            }
        }
    }
}

fn micros(sample_rate: u32, microseconds: u32) -> usize {
    (u64::from(sample_rate) * u64::from(microseconds) / 1_000_000) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_peaks_and_reports_statistics() {
        let params = LimiterParams {
            channel_count: 1,
            sample_rate: 48_000,
            look_ahead_time_max_us: 1000,
            look_ahead_time_us: 100,
            attack_coefficient: 1.0,
            release_coefficient: 0.1,
            threshold: 0.5,
            input_gain: 1.0,
            output_gain: 1.0,
            statistics_enabled: true,
            statistics_reset: false,
        };
        let input = [1.0; 16];
        let mut output = [0.0; 16];
        let mut limiter = Limiter::new(params);
        limiter.process(&[&input], &mut [&mut output], 16);
        assert!(output.iter().all(|sample| sample.abs() <= 0.5));
        let statistics = limiter.statistics();
        assert_eq!(statistics.processed_sample_count, 16);
        assert_eq!(statistics.peak_input, 1.0);
        assert_eq!(statistics.peak_output, 0.5);
        assert_eq!(statistics.maximum_gain_reduction, 0.5);
    }
}
