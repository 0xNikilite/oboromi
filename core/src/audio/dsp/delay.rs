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
    buffers: Vec<Vec<f32>>,
    positions: [usize; 6],
    low_pass: [f32; 6],
    delay_samples: usize,
}

impl Delay {
    pub fn new(params: DelayParams) -> Self {
        let capacity = samples(params.sample_rate, params.delay_time_max_ms).max(1) + 1;
        let buffers = (0..params.channel_count.min(6))
            .map(|_| vec![0.0; capacity])
            .collect();
        let mut effect = Self {
            params,
            buffers,
            positions: [0; 6],
            low_pass: [0.0; 6],
            delay_samples: 1,
        };
        effect.set(params);
        effect
    }

    pub fn set(&mut self, params: DelayParams) {
        self.params = params;
        self.delay_samples = samples(params.sample_rate, params.delay_time_ms)
            .clamp(1, self.buffers.first().map_or(1, |buffer| buffer.len() - 1));
        for position in &mut self.positions {
            *position %= self.delay_samples;
        }
    }

    pub fn process(&mut self, input: &[&[f32]], output: &mut [&mut [f32]], frames: usize) {
        let channels = self
            .params
            .channel_count
            .min(input.len())
            .min(output.len())
            .min(6);
        for frame in 0..frames {
            let mut delayed = [0.0; 6];
            for (channel, sample) in delayed.iter_mut().enumerate().take(channels) {
                *sample = self.buffers[channel][self.positions[channel]];
            }
            let sum = delayed[..channels].iter().sum::<f32>();
            for channel in 0..channels {
                if frame >= input[channel].len() || frame >= output[channel].len() {
                    continue;
                }
                let other = if channels > 1 {
                    (sum - delayed[channel]) / (channels - 1) as f32
                } else {
                    0.0
                };
                let feedback = delayed[channel] * (1.0 - self.params.channel_spread)
                    + other * self.params.channel_spread;
                self.low_pass[channel] +=
                    (feedback - self.low_pass[channel]) * (1.0 - self.params.low_pass_amount);
                self.buffers[channel][self.positions[channel]] = input[channel][frame]
                    * self.params.in_gain
                    + self.low_pass[channel] * self.params.feedback_gain;
                self.positions[channel] = (self.positions[channel] + 1) % self.delay_samples;
                output[channel][frame] = input[channel][frame] * self.params.dry_gain
                    + delayed[channel] * self.params.out_gain;
            }
        }
    }
}

fn samples(sample_rate: u32, milliseconds: u32) -> usize {
    (u64::from(sample_rate) * u64::from(milliseconds) / 1000) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> DelayParams {
        DelayParams {
            channel_count: 1,
            sample_rate: 1000,
            delay_time_max_ms: 20,
            delay_time_ms: 10,
            in_gain: 1.0,
            feedback_gain: 0.5,
            out_gain: 0.75,
            dry_gain: 0.25,
            channel_spread: 0.0,
            low_pass_amount: 0.0,
        }
    }

    #[test]
    fn impulse_obeys_dry_wet_and_feedback_gains() {
        let mut input = [0.0; 120];
        input[0] = 1.0;
        let mut output = [0.0; 120];
        Delay::new(params()).process(&[&input], &mut [&mut output], 120);
        assert_eq!(output[0], 0.25);
        assert_eq!(output[10], 0.75);
        assert_eq!(output[20], 0.375);
        assert!(output[110].abs() < 0.001);
    }
}
