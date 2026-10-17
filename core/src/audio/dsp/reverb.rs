#[derive(Clone, Copy)]
pub struct ReverbParams {
    pub channel_count: usize,
    pub sample_rate: u32,
    pub early_gain: f32,
    pub pre_delay_time_ms: f32,
    pub late_gain: f32,
    pub decay_time: f32,
    pub high_frequency_decay_ratio: f32,
    pub coloration: f32,
    pub reverb_gain: f32,
    pub out_gain: f32,
    pub dry_gain: f32,
}

struct Line {
    buffer: Vec<f32>,
    position: usize,
    length: usize,
}

impl Line {
    fn new(length: usize) -> Self {
        Self {
            buffer: vec![0.0; length.max(1)],
            position: 0,
            length: length.max(1),
        }
    }

    fn set_length(&mut self, length: usize) {
        self.length = length.clamp(1, self.buffer.len());
        self.position %= self.length;
    }

    fn tick(&mut self, input: f32) -> f32 {
        let output = self.buffer[self.position];
        self.buffer[self.position] = input;
        self.position = (self.position + 1) % self.length;
        output
    }
}

struct Comb {
    line: Line,
    feedback: f32,
    damping: f32,
    filtered: f32,
}

impl Comb {
    fn tick(&mut self, input: f32) -> f32 {
        let output = self.line.tick(0.0);
        self.filtered += (output - self.filtered) * (1.0 - self.damping);
        let position = if self.line.position == 0 {
            self.line.length - 1
        } else {
            self.line.position - 1
        };
        self.line.buffer[position] = input + self.filtered * self.feedback;
        output
    }
}

struct AllPass {
    line: Line,
    gain: f32,
}

impl AllPass {
    fn tick(&mut self, input: f32) -> f32 {
        let delayed = self.line.tick(0.0);
        let output = delayed - input * self.gain;
        let position = if self.line.position == 0 {
            self.line.length - 1
        } else {
            self.line.position - 1
        };
        self.line.buffer[position] = input + delayed * self.gain;
        output
    }
}

struct Channel {
    pre_delay: Line,
    early: [Line; 2],
    combs: [Comb; 4],
    all_pass: [AllPass; 2],
}

pub struct Reverb {
    params: ReverbParams,
    channels: Vec<Channel>,
}

impl Reverb {
    pub fn new(params: ReverbParams) -> Self {
        let channels = (0..params.channel_count.min(6))
            .map(|channel| make_channel(params.sample_rate, channel))
            .collect();
        let mut effect = Self { params, channels };
        effect.set(params);
        effect
    }

    pub fn set(&mut self, params: ReverbParams) {
        self.params = params;
        let pre_delay = (params.pre_delay_time_ms * params.sample_rate as f32 / 1000.0) as usize;
        for channel in &mut self.channels {
            channel.pre_delay.set_length(pre_delay.max(1));
            for comb in &mut channel.combs {
                let seconds = comb.line.length as f32 / params.sample_rate as f32;
                comb.feedback = 10.0_f32.powf(-3.0 * seconds / params.decay_time.max(0.001));
                comb.damping = (1.0 - params.high_frequency_decay_ratio).clamp(0.0, 0.99);
            }
            for all_pass in &mut channel.all_pass {
                all_pass.gain = (params.coloration * 0.7).clamp(0.0, 0.7);
            }
        }
    }

    pub fn process(&mut self, input: &[&[f32]], output: &mut [&mut [f32]], frames: usize) {
        let channels = self.channels.len().min(input.len()).min(output.len());
        for channel in 0..channels {
            for frame in 0..frames.min(input[channel].len()).min(output[channel].len()) {
                let sample = input[channel][frame];
                let predelayed = self.channels[channel].pre_delay.tick(sample);
                let early = self.channels[channel].early[0].tick(predelayed) * 0.6
                    + self.channels[channel].early[1].tick(predelayed) * 0.4;
                let late = self.channels[channel]
                    .combs
                    .iter_mut()
                    .map(|comb| comb.tick(predelayed))
                    .sum::<f32>()
                    * 0.25;
                let late = self.channels[channel]
                    .all_pass
                    .iter_mut()
                    .fold(late, |value, all_pass| all_pass.tick(value));
                let wet = (early * self.params.early_gain + late * self.params.late_gain)
                    * self.params.reverb_gain
                    * self.params.out_gain;
                output[channel][frame] = sample * self.params.dry_gain + wet;
            }
        }
    }
}

fn make_channel(sample_rate: u32, channel: usize) -> Channel {
    let scaled = |milliseconds: usize| (sample_rate as usize * milliseconds / 1000).max(1);
    let offsets = [29, 37, 41, 43];
    Channel {
        pre_delay: Line::new(scaled(350)),
        early: [
            Line::new(scaled(7 + channel)),
            Line::new(scaled(19 + channel)),
        ],
        combs: offsets.map(|delay| Comb {
            line: Line::new(scaled(delay + channel)),
            feedback: 0.0,
            damping: 0.0,
            filtered: 0.0,
        }),
        all_pass: [5, 11].map(|delay| AllPass {
            line: Line::new(scaled(delay + channel)),
            gain: 0.0,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> ReverbParams {
        ReverbParams {
            channel_count: 1,
            sample_rate: 1000,
            early_gain: 0.4,
            pre_delay_time_ms: 1.0,
            late_gain: 0.8,
            decay_time: 0.4,
            high_frequency_decay_ratio: 0.8,
            coloration: 0.5,
            reverb_gain: 1.0,
            out_gain: 0.5,
            dry_gain: 0.25,
        }
    }

    #[test]
    fn impulse_has_requested_mix_and_sixty_db_decay() {
        let mut input = [0.0; 900];
        input[0] = 1.0;
        let mut output = [0.0; 900];
        Reverb::new(params()).process(&[&input], &mut [&mut output], 900);
        assert_eq!(output[0], 0.25);
        assert!(output[1..400].iter().any(|sample| sample.abs() > 0.01));
        assert!(output[500..].iter().all(|sample| sample.abs() < 0.001));
    }
}
