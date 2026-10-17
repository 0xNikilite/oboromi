#[derive(Clone, Copy)]
pub struct DepopParams {
    pub sample_rate: u32,
}

pub struct Depop {
    decay: f32,
    value: f32,
}

impl Depop {
    pub fn new(params: DepopParams) -> Self {
        Self {
            decay: decay(params.sample_rate),
            value: 0.0,
        }
    }

    pub fn set(&mut self, params: DepopParams) {
        self.decay = decay(params.sample_rate);
    }

    pub fn trigger(&mut self, last_sample: f32) {
        self.value += last_sample;
    }

    pub fn process(&mut self, output: &mut [f32], frames: usize) {
        for sample in output.iter_mut().take(frames) {
            self.value *= self.decay;
            *sample += self.value;
        }
    }
}

fn decay(sample_rate: u32) -> f32 {
    if sample_rate == 32_000 {
        0.943695
    } else {
        0.962189
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_removal_decays_without_a_hard_boundary() {
        let mut depop = Depop::new(DepopParams {
            sample_rate: 48_000,
        });
        depop.trigger(1.0);
        let mut output = [0.0; 240];
        depop.process(&mut output, 240);
        assert!((1.0 - output[0]).abs() < 0.04);
        assert!(
            output
                .windows(2)
                .all(|samples| samples[1].abs() < samples[0].abs())
        );
        assert!(output[239].abs() < 0.0001);
    }
}
