#![allow(dead_code)]

use crate::mmu::Memory;

pub const AUDIO_SAMPLE_RATE: u32 = 48_000;
pub const AUDIO_CHANNELS: u32 = 2; // Stereo

/// Emulates the Horizon OS `audout:u` / `audren` 48kHz Stereo 16-bit PCM Audio Engine.
#[derive(Debug)]
pub struct AudioOutServer {
    pub sample_rate: u32,
    pub channels: u32,
    pub is_playing: bool,
    pub buffers_queued: u64,
    pub total_samples_processed: u64,
    pub peak_amplitude: i16,
    ring_buffer: Vec<i16>,
}

impl Default for AudioOutServer {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioOutServer {
    pub fn new() -> Self {
        Self {
            sample_rate: AUDIO_SAMPLE_RATE,
            channels: AUDIO_CHANNELS,
            is_playing: true,
            buffers_queued: 0,
            total_samples_processed: 0,
            peak_amplitude: 0,
            ring_buffer: Vec::with_capacity(48_000),
        }
    }

    /// Reads 16-bit signed PCM samples from guest memory and queues them to the audio mixer.
    pub fn append_pcm_buffer(&mut self, mmu: &Memory, sample_addr: u64, sample_count: usize) {
        if sample_addr == 0 || sample_count == 0 {
            return;
        }

        let mut peak = self.peak_amplitude;
        for i in 0..sample_count {
            let addr = sample_addr + (i * 2) as u64;
            if let Ok(raw) = mmu.read_u16(addr) {
                let sample = raw as i16;
                let abs_val = sample.saturating_abs();
                if abs_val > peak {
                    peak = abs_val;
                }
                if self.ring_buffer.len() < 96_000 {
                    self.ring_buffer.push(sample);
                }
            }
        }

        self.peak_amplitude = peak;
        self.buffers_queued += 1;
        self.total_samples_processed += sample_count as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mmu::Permission;

    #[test]
    fn test_audio_append_pcm_buffer() {
        let mut mmu = Memory::new();
        let mut audio = AudioOutServer::new();
        let pcm_addr = 0x80060000u64;
        mmu.map_range(pcm_addr, 4096, Permission::READ_WRITE);

        // Write 4 stereo samples (8 i16s) with peak amplitude 12000
        let samples: [i16; 8] = [2000, -2000, 6000, -6000, 12000, -12000, 4000, -4000];
        for (i, &s) in samples.iter().enumerate() {
            mmu.write_u16(pcm_addr + (i * 2) as u64, s as u16).unwrap();
        }

        audio.append_pcm_buffer(&mmu, pcm_addr, samples.len());
        assert_eq!(audio.buffers_queued, 1);
        assert_eq!(audio.total_samples_processed, 8);
        assert_eq!(audio.peak_amplitude, 12000);
    }
}
