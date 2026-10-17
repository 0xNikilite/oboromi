//! audout's IAudioOut: the game appends tagged pcm16 buffers and polls which ones played; each
//! `step` plays one 240-frame period of them as 48 kHz stereo for the sink ring.

use super::Error;
use super::sink_device::{DEFAULT_DOWNMIX, stereo, to_i16};
use std::collections::VecDeque;

const RATE: u32 = 48_000;
const FRAMES: usize = 240;
const BUFFER_LIMIT: usize = 32;

pub struct AudioOut {
    channels: usize,
    queue: VecDeque<(Vec<u8>, u64)>,
    position: usize,
    released: Vec<u64>,
    started: bool,
    period: Vec<i16>,
}

impl AudioOut {
    /// OpenAudioOut with an AudioOutParameter; zero picks 48 kHz and two channels.
    pub fn open(rate: u32, channels: u16) -> Result<Self, Error> {
        if rate != 0 && rate != RATE {
            return Err(Error::InvalidSampleRate);
        }
        let channels = if channels == 0 {
            2
        } else {
            usize::from(channels)
        };
        if ![1, 2, 6].contains(&channels) {
            return Err(Error::InvalidChannelCount);
        }
        Ok(Self {
            channels,
            queue: VecDeque::new(),
            position: 0,
            released: Vec::new(),
            started: false,
            period: vec![0; FRAMES * 2],
        })
    }

    pub fn start(&mut self) {
        self.started = true;
    }

    pub fn stop(&mut self) {
        self.started = false;
    }

    pub fn append(&mut self, buffer: &[u8], tag: u64) -> Result<(), Error> {
        if self.queue.len() + self.released.len() >= BUFFER_LIMIT {
            return Err(Error::TooManyUnreleasedBuffers);
        }
        self.queue.push_back((buffer.to_vec(), tag));
        Ok(())
    }

    /// tags of the buffers played since the last call, in append order.
    pub fn released(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.released)
    }

    /// plays one period, silence once the queue runs dry, nothing while stopped.
    pub fn step(&mut self) -> &[i16] {
        if !self.started {
            return &[];
        }
        let frame_bytes = self.channels * 2;
        let mut channels = [0.0; 6];
        for frame in 0..FRAMES {
            self.release_played();
            let Some((buffer, _)) = self.queue.front() else {
                self.period[frame * 2..].fill(0);
                break;
            };
            let samples = &buffer[self.position..self.position + frame_bytes];
            for (channel, sample) in channels.iter_mut().zip(samples.chunks_exact(2)) {
                *channel = f32::from(i16::from_le_bytes([sample[0], sample[1]])) / 32768.0;
            }
            self.position += frame_bytes;
            let [left, right] = stereo(&channels[..self.channels], DEFAULT_DOWNMIX);
            self.period[frame * 2..frame * 2 + 2].copy_from_slice(&[to_i16(left), to_i16(right)]);
        }
        self.release_played();
        &self.period
    }

    fn release_played(&mut self) {
        while self
            .queue
            .front()
            .is_some_and(|(buffer, _)| self.position + self.channels * 2 > buffer.len())
        {
            self.released.push(self.queue.pop_front().unwrap().1);
            self.position = 0;
        }
    }
}
