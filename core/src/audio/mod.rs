//! the audio services a game drives: the renderer (audren) mixes voices through mixes, effects
//! and sinks once per 5 ms period, audout plays queued pcm, and both end in the gui's sink ring.

pub mod audout;
pub mod dsp;
pub mod effect;
pub mod mix;
pub mod parcel;
pub mod renderer;
pub mod sink;
pub mod sink_device;
pub mod voice;

pub use sink::{CHANNELS, RATE, generation, pull, push, test_tone, underruns};

/// guest memory as the renderer reads wave buffers and writes aux and circular-sink buffers.
pub trait GuestMemory {
    fn read(&self, va: u64, out: &mut [u8]);
    fn write(&mut self, va: u64, data: &[u8]);
}

/// guest memory over a host vec starting at `base`, for host-driven demos.
pub struct HostMemory {
    pub base: u64,
    pub bytes: Vec<u8>,
}

impl GuestMemory for HostMemory {
    fn read(&self, va: u64, out: &mut [u8]) {
        let at = (va - self.base) as usize;
        out.copy_from_slice(&self.bytes[at..at + out.len()]);
    }

    fn write(&mut self, va: u64, data: &[u8]) {
        let at = (va - self.base) as usize;
        self.bytes[at..at + data.len()].copy_from_slice(data);
    }
}

/// result descriptions of module 153, switchbrew "Error codes".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Error {
    OperationFailed = 2,
    InvalidSampleRate = 3,
    InsufficientBuffer = 4,
    TooManyUnreleasedBuffers = 8,
    InvalidChannelCount = 10,
    InvalidUpdateInfo = 41,
    NoMemoryPoolEntry = 42,
    CycleDetected = 43,
}

impl Error {
    pub fn result(self) -> u32 {
        153 | (self as u32) << 9
    }
}

/// runs a kernel over `buffer` in place; periods are at most 240 frames.
pub(crate) fn in_place(buffer: &mut [f32], kernel: impl FnOnce(&[f32], &mut [f32])) {
    let mut input = [0.0; 240];
    let input = &mut input[..buffer.len()];
    input.copy_from_slice(buffer);
    kernel(input, buffer);
}

/// adds `buffers[from] * volume` into `buffers[to]`.
pub(crate) fn mix_into(buffers: &mut [Vec<f32>], from: usize, to: usize, volume: f32) {
    let mut input = [0.0; 240];
    let input = &mut input[..buffers[from].len()];
    input.copy_from_slice(&buffers[from]);
    for (output, sample) in buffers[to].iter_mut().zip(input.iter()) {
        *output += sample * volume;
    }
}

