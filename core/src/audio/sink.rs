//! host output: a single-producer single-consumer ring of interleaved stereo
//! i16 at the switch output rate, drained by the gui's audio sink over ffi.

use std::f32::consts::TAU;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};

pub const RATE: u32 = 48_000;
pub const CHANNELS: u32 = 2;
/// the console renderer's update period.
pub const PERIOD: u32 = 240;
const CAPACITY: usize = 3 * PERIOD as usize;

struct Ring {
    frames: [AtomicU32; CAPACITY],
    written: AtomicU64,
    read: AtomicU64,
    underruns: AtomicU64,
}

impl Ring {
    const fn new() -> Self {
        Ring {
            frames: [const { AtomicU32::new(0) }; CAPACITY],
            written: AtomicU64::new(0),
            read: AtomicU64::new(0),
            underruns: AtomicU64::new(0),
        }
    }

    fn push(&self, samples: &[i16]) -> usize {
        let written = self.written.load(Relaxed);
        let free = CAPACITY - (written - self.read.load(Acquire)) as usize;
        let frames = samples.chunks_exact(2).take(free);
        let count = frames.len();
        for (i, frame) in frames.enumerate() {
            let packed = u32::from(frame[0] as u16) | u32::from(frame[1] as u16) << 16;
            self.frames[(written as usize + i) % CAPACITY].store(packed, Relaxed);
        }
        self.written.store(written + count as u64, Release);
        count
    }

    fn pull(&self, dst: &mut [i16]) -> usize {
        let read = self.read.load(Relaxed);
        let available = (self.written.load(Acquire) - read) as usize;
        let mut frames = dst.chunks_exact_mut(2);
        let count = frames.len().min(available);
        for (i, frame) in frames.by_ref().take(count).enumerate() {
            let packed = self.frames[(read as usize + i) % CAPACITY].load(Relaxed);
            frame.copy_from_slice(&[packed as i16, (packed >> 16) as i16]);
        }
        if frames.len() > 0 {
            frames.for_each(|frame| frame.fill(0));
            self.underruns.fetch_add(1, Relaxed);
        }
        self.read.store(read + count as u64, Release);
        count
    }
}

static RING: Ring = Ring::new();
static TONE_FRAMES: AtomicU64 = AtomicU64::new(0);

/// queues interleaved stereo samples, returns the frames that fit.
pub fn push(samples: &[i16]) -> usize {
    RING.push(samples)
}

/// fills `dst` with queued frames and zeros after them, returns the queued frames copied.
pub fn pull(dst: &mut [i16]) -> usize {
    RING.pull(dst)
}

/// frames ever pushed.
pub fn generation() -> u64 {
    RING.written.load(Acquire)
}

/// frames waiting for the consumer.
pub fn queued() -> usize {
    (RING.written.load(Acquire) - RING.read.load(Acquire)) as usize
}

/// pulls that ran out of queued frames.
pub fn underruns() -> u64 {
    RING.underruns.load(Relaxed)
}

fn tone(start: u64, frames: usize) -> Vec<i16> {
    let amplitude = 10f32.powf(-12.0 / 20.0) * f32::from(i16::MAX);
    (start..start + frames as u64)
        .flat_map(|n| {
            let phase = TAU * 440.0 * (n % u64::from(RATE)) as f32 / RATE as f32;
            [(amplitude * phase.sin()) as i16; 2]
        })
        .collect()
}

/// queues `seconds` more of a 440 hz sine at -12 dbfs, returns the frames that fit.
pub fn test_tone(seconds: f32) -> usize {
    let start = TONE_FRAMES.load(Relaxed);
    let frames = ((seconds * RATE as f32) as usize).min(CAPACITY);
    let pushed = push(&tone(start, frames));
    TONE_FRAMES.store(start + pushed as u64, Relaxed);
    pushed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_stops_at_capacity() {
        let ring = Ring::new();
        let samples: Vec<i16> = (0..2 * CAPACITY as i32 + 2).map(|i| (i - 20_000) as i16).collect();
        assert_eq!(ring.push(&samples), CAPACITY);
        let mut out = vec![0; 2 * CAPACITY];
        assert_eq!(ring.pull(&mut out), CAPACITY);
        assert_eq!(out, samples[..2 * CAPACITY]);
        assert_eq!(ring.underruns.load(Relaxed), 0);
    }

    #[test]
    fn underrun_zero_fills_and_counts() {
        let ring = Ring::new();
        ring.push(&[1, -1, 2, -2]);
        let mut out = [7; 8];
        assert_eq!(ring.pull(&mut out), 2);
        assert_eq!(out, [1, -1, 2, -2, 0, 0, 0, 0]);
        assert_eq!(ring.pull(&mut out), 0);
        assert_eq!(out, [0; 8]);
        assert_eq!(ring.underruns.load(Relaxed), 2);
    }

    #[test]
    fn wraps_around() {
        let ring = Ring::new();
        let mut out = vec![0; 2 * CAPACITY];
        ring.push(&vec![0; 2 * (CAPACITY - 3)]);
        ring.pull(&mut out[..2 * (CAPACITY - 3)]);
        let samples: Vec<i16> = (1..=20).collect();
        assert_eq!(ring.push(&samples), 10);
        assert_eq!(ring.pull(&mut out[..20]), 10);
        assert_eq!(out[..20], samples);
    }

    #[test]
    fn tone_is_440_hz_at_minus_12_dbfs() {
        let left: Vec<i16> = tone(0, RATE as usize).into_iter().step_by(2).collect();
        let crossings = left.windows(2).filter(|w| (w[0] < 0) != (w[1] < 0)).count();
        assert!((879..=881).contains(&crossings), "{crossings}");
        assert_eq!(left.iter().max(), Some(&8230));
    }
}
