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

    pub fn set(&mut self, params: VolumeRampParams) {
        self.params = params;
    }

    pub fn process(&mut self, input: &[f32], output: &mut [f32], frames: usize) {
        let count = frames.min(input.len()).min(output.len());
        let step = if count > 1 {
            (self.params.volume - self.params.previous_volume) / (count - 1) as f32
        } else {
            0.0
        };
        for (index, (&sample, result)) in input.iter().zip(output).take(count).enumerate() {
            *result = sample * (self.params.previous_volume + step * index as f32);
        }
        self.params.previous_volume = self.params.volume;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consecutive_ramps_share_the_boundary_value() {
        let mut ramp = VolumeRamp::new(VolumeRampParams {
            previous_volume: 0.0,
            volume: 1.0,
        });
        let mut first = [0.0; 240];
        ramp.process(&[1.0; 240], &mut first, 240);
        ramp.set(VolumeRampParams {
            previous_volume: first[239],
            volume: 0.5,
        });
        let mut second = [0.0; 240];
        ramp.process(&[1.0; 240], &mut second, 240);
        assert_eq!(first[239], second[0]);
        assert_eq!(second[239], 0.5);
    }
}
