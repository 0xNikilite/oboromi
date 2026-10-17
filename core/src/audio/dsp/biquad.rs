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
    z1: f32,
    z2: f32,
}

impl Biquad {
    pub fn new(params: BiquadParams) -> Self {
        Self {
            params,
            z1: 0.0,
            z2: 0.0,
        }
    }

    pub fn set(&mut self, params: BiquadParams) {
        if params.new_state {
            self.z1 = 0.0;
            self.z2 = 0.0;
        }
        self.params = params;
    }

    pub fn process(&mut self, input: &[f32], output: &mut [f32], frames: usize) {
        for (&sample, result) in input.iter().zip(output).take(frames) {
            let filtered = self.params.b0 * sample + self.z1;
            self.z1 = self.params.b1 * sample - self.params.a1 * filtered + self.z2;
            self.z2 = self.params.b2 * sample - self.params.a2 * filtered;
            *result = filtered;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{FRAC_1_SQRT_2, TAU};

    fn low_pass(frequency: f32) -> BiquadParams {
        let omega = TAU * frequency / 48_000.0;
        let alpha = omega.sin() * FRAC_1_SQRT_2;
        let a0 = 1.0 + alpha;
        BiquadParams {
            b0: (1.0 - omega.cos()) * 0.5 / a0,
            b1: (1.0 - omega.cos()) / a0,
            b2: (1.0 - omega.cos()) * 0.5 / a0,
            a1: -2.0 * omega.cos() / a0,
            a2: (1.0 - alpha) / a0,
            new_state: false,
        }
    }

    #[test]
    fn low_pass_is_minus_3_db_at_cutoff() {
        let input: Vec<_> = (0..4096)
            .map(|i| (TAU * 4000.0 * i as f32 / 48_000.0).sin())
            .collect();
        let mut output = vec![0.0; input.len()];
        Biquad::new(low_pass(4000.0)).process(&input, &mut output, input.len());
        let gain = output[512..].iter().map(|x| x * x).sum::<f32>()
            / input[512..].iter().map(|x| x * x).sum::<f32>();
        assert!((10.0 * gain.log10() + 3.01).abs() < 0.05);
    }

    #[test]
    fn state_is_continuous_across_updates() {
        let input: Vec<_> = (0..480).map(|i| (i as f32 * 0.03).sin()).collect();
        let mut whole = [0.0; 480];
        Biquad::new(low_pass(6000.0)).process(&input, &mut whole, 480);
        let mut split = [0.0; 480];
        let mut filter = Biquad::new(low_pass(6000.0));
        filter.process(&input[..240], &mut split[..240], 240);
        filter.process(&input[240..], &mut split[240..], 240);
        assert_eq!(split, whole);

        let mut reset = low_pass(6000.0);
        reset.new_state = true;
        filter.set(reset);
        let mut impulse = [0.0; 1];
        filter.process(&[1.0], &mut impulse, 1);
        assert_eq!(impulse[0], reset.b0);
    }
}
