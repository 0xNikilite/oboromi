#[derive(Clone, Copy)]
pub struct AdpcmParams {
    pub coefficients: [i16; 16],
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct AdpcmContext {
    pub predictor_scale: u8,
    pub history_sample_1: i16,
    pub history_sample_2: i16,
}

pub struct GcAdpcmDecoder {
    params: AdpcmParams,
    context: AdpcmContext,
    frame: [u8; 8],
    frame_sample: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AdpcmProgress {
    pub samples: usize,
    pub bytes: usize,
}

impl GcAdpcmDecoder {
    pub fn new(params: AdpcmParams) -> Self {
        Self {
            params,
            context: AdpcmContext::default(),
            frame: [0; 8],
            frame_sample: 14,
        }
    }

    pub fn set(&mut self, params: AdpcmParams) {
        self.params = params;
    }

    pub fn context(&self) -> AdpcmContext {
        self.context
    }

    pub fn restore(&mut self, context: AdpcmContext) {
        self.context = context;
        self.frame_sample = 14;
    }

    // Nintendo DSP-ADPCM stores 14 signed nibbles after each predictor/scale byte.
    pub fn process(&mut self, input: &[u8], output: &mut [f32], samples: usize) -> AdpcmProgress {
        let mut written = 0;
        let mut consumed = 0;
        while written < samples && written < output.len() {
            if self.frame_sample == 14 {
                let Some(frame) = input.get(consumed..consumed + 8) else {
                    break;
                };
                self.frame.copy_from_slice(frame);
                consumed += 8;
                self.frame_sample = 0;
            }
            let header = self.frame[0];
            let predictor = usize::from(header >> 4).min(7) * 2;
            let scale = 1_i32 << (header & 0xf);
            self.context.predictor_scale = header;
            let byte = self.frame[1 + self.frame_sample / 2];
            let nibble = if self.frame_sample & 1 == 0 {
                byte >> 4
            } else {
                byte & 0xf
            };
            let nibble = i32::from((nibble as i8) << 4) >> 4;
            let predicted = i32::from(self.params.coefficients[predictor])
                * i32::from(self.context.history_sample_1)
                + i32::from(self.params.coefficients[predictor + 1])
                    * i32::from(self.context.history_sample_2);
            let sample = ((nibble * scale << 11) + predicted + 1024) >> 11;
            let sample = sample.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
            self.context.history_sample_2 = self.context.history_sample_1;
            self.context.history_sample_1 = sample;
            output[written] = f32::from(sample) / 32768.0;
            self.frame_sample += 1;
            written += 1;
        }
        AdpcmProgress {
            samples: written,
            bytes: consumed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    fn encode(input: &[i16], coefficients: [i16; 16]) -> Vec<u8> {
        let mut result = Vec::new();
        let (mut y1, mut y2) = (0_i32, 0_i32);
        for samples in input.chunks(14) {
            let predictor = 1_usize;
            let (c1, c2) = (i32::from(coefficients[2]), i32::from(coefficients[3]));
            let scale = (0..=12)
                .find(|shift| {
                    let mut h1 = y1;
                    let mut h2 = y2;
                    samples.iter().all(|&sample| {
                        let predicted = (c1 * h1 + c2 * h2 + 1024) >> 11;
                        let quantized = ((i32::from(sample) - predicted) as f32
                            / (1_i32 << shift) as f32)
                            .round() as i32;
                        h2 = h1;
                        h1 = predicted + quantized.clamp(-8, 7) * (1_i32 << shift);
                        (-8..=7).contains(&quantized)
                    })
                })
                .unwrap();
            result.push(((predictor as u8) << 4) | scale);
            for pair in samples.chunks(2) {
                let mut byte = 0;
                for (half, &sample) in pair.iter().enumerate() {
                    let predicted = (c1 * y1 + c2 * y2 + 1024) >> 11;
                    let nibble = ((i32::from(sample) - predicted) as f32 / (1_i32 << scale) as f32)
                        .round() as i32;
                    let nibble = nibble.clamp(-8, 7);
                    y2 = y1;
                    y1 = (predicted + nibble * (1_i32 << scale)).clamp(-32768, 32767);
                    byte |= ((nibble as u8) & 0xf) << (4 - half * 4);
                }
                result.push(byte);
            }
            result.resize(result.len() + 7 - samples.len().div_ceil(2), 0);
        }
        result
    }

    #[test]
    fn waveform_round_trips_within_quantization() {
        let mut coefficients = [0; 16];
        coefficients[2] = 2048;
        let source: Vec<_> = (0..42)
            .map(|i| (4000.0 * (TAU * i as f32 / 96.0).sin()) as i16)
            .collect();
        let encoded = encode(&source, coefficients);
        let mut decoded = vec![0.0; source.len()];
        let progress = GcAdpcmDecoder::new(AdpcmParams { coefficients }).process(
            &encoded,
            &mut decoded,
            source.len(),
        );
        assert_eq!(progress.samples, source.len());
        assert!(
            source
                .iter()
                .zip(decoded)
                .all(|(&expected, actual)| (f32::from(expected) / 32768.0 - actual).abs() < 0.004)
        );
    }

    #[test]
    fn loop_context_restores_history() {
        let mut coefficients = [0; 16];
        coefficients[2] = 2048;
        let source: Vec<_> = (0..28).map(|i| i * 80).collect();
        let encoded = encode(&source, coefficients);
        let params = AdpcmParams { coefficients };
        let mut decoder = GcAdpcmDecoder::new(params);
        let mut first = [0.0; 14];
        decoder.process(&encoded[..8], &mut first, 14);
        let loop_context = decoder.context();
        let mut expected = [0.0; 14];
        decoder.process(&encoded[8..], &mut expected, 14);
        decoder.restore(loop_context);
        let mut replay = [0.0; 14];
        decoder.process(&encoded[8..], &mut replay, 14);
        assert_eq!(replay, expected);
    }

    #[test]
    fn partial_frame_continues_in_the_next_update() {
        let mut coefficients = [0; 16];
        coefficients[2] = 2048;
        let source: Vec<_> = (0..28).map(|i| i * 80).collect();
        let encoded = encode(&source, coefficients);
        let mut decoder = GcAdpcmDecoder::new(AdpcmParams { coefficients });
        let mut output = [0.0; 28];
        let first = decoder.process(&encoded, &mut output[..16], 16);
        assert_eq!(
            first,
            AdpcmProgress {
                samples: 16,
                bytes: 16
            }
        );
        let second = decoder.process(&encoded[first.bytes..], &mut output[16..], 12);
        assert_eq!(
            second,
            AdpcmProgress {
                samples: 12,
                bytes: 0
            }
        );

        let mut expected = [0.0; 28];
        GcAdpcmDecoder::new(AdpcmParams { coefficients }).process(&encoded, &mut expected, 28);
        assert_eq!(output, expected);
    }
}
