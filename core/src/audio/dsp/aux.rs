#[derive(Clone, Copy)]
pub struct AuxiliaryBufferParams {
    pub buffer_storage_size: usize,
    pub mix_buffer_sample_count: usize,
}

struct Ring {
    samples: Vec<f32>,
    read: usize,
    write: usize,
    len: usize,
}

impl Ring {
    fn new(capacity: usize) -> Self {
        Self {
            samples: vec![0.0; capacity.max(1)],
            read: 0,
            write: 0,
            len: 0,
        }
    }

    fn push(&mut self, input: &[f32]) -> usize {
        let count = input.len().min(self.samples.len() - self.len);
        for &sample in &input[..count] {
            self.samples[self.write] = sample;
            self.write = (self.write + 1) % self.samples.len();
        }
        self.len += count;
        count
    }

    fn pop(&mut self, output: &mut [f32]) -> usize {
        let count = output.len().min(self.len);
        for sample in &mut output[..count] {
            *sample = self.samples[self.read];
            self.read = (self.read + 1) % self.samples.len();
        }
        self.len -= count;
        count
    }
}

pub struct AuxiliaryBuffer {
    params: AuxiliaryBufferParams,
    send: Ring,
    returned: Ring,
}

impl AuxiliaryBuffer {
    pub fn new(params: AuxiliaryBufferParams) -> Self {
        Self {
            params,
            send: Ring::new(params.buffer_storage_size),
            returned: Ring::new(params.buffer_storage_size),
        }
    }

    pub fn set(&mut self, params: AuxiliaryBufferParams) {
        self.params = params;
    }

    pub fn read_send(&mut self, output: &mut [f32]) -> usize {
        self.send.pop(output)
    }

    pub fn write_return(&mut self, input: &[f32]) -> usize {
        self.returned.push(input)
    }

    pub fn process(&mut self, input: &[f32], output: &mut [f32], frames: usize) {
        let count = frames
            .min(self.params.mix_buffer_sample_count)
            .min(input.len())
            .min(output.len());
        self.send.push(&input[..count]);
        let returned = self.returned.pop(&mut output[..count]);
        output[returned..count].fill(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_and_return_wrap_independently() {
        let mut aux = AuxiliaryBuffer::new(AuxiliaryBufferParams {
            buffer_storage_size: 6,
            mix_buffer_sample_count: 4,
        });
        assert_eq!(aux.write_return(&[9.0, 8.0, 7.0, 6.0]), 4);
        let mut output = [0.0; 4];
        aux.process(&[1.0, 2.0, 3.0, 4.0], &mut output, 4);
        assert_eq!(output, [9.0, 8.0, 7.0, 6.0]);
        let mut sent = [0.0; 3];
        assert_eq!(aux.read_send(&mut sent), 3);
        assert_eq!(sent, [1.0, 2.0, 3.0]);
        aux.process(&[5.0, 6.0, 7.0, 8.0], &mut output, 4);
        assert_eq!(aux.read_send(&mut sent), 3);
        assert_eq!(sent, [4.0, 5.0, 6.0]);
    }
}
