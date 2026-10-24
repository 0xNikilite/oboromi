#![allow(dead_code)]

use crate::mmu::Memory;
use std::time::Instant;

pub const SCREEN_WIDTH: usize = 1280;
pub const SCREEN_HEIGHT: usize = 720;
pub const FRAMEBUFFER_SIZE: usize = SCREEN_WIDTH * SCREEN_HEIGHT * 4; // 3,686,400 bytes (RGBA8)
pub const DEFAULT_FRAMEBUFFER_ADDR: u64 = 0x90000000;

pub struct DisplayConfig {
    pub width: usize,
    pub height: usize,
    pub title: String,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            width: SCREEN_WIDTH,
            height: SCREEN_HEIGHT,
            title: "Ounce-Core | Nintendo Switch 2 (T239) Display".to_string(),
        }
    }
}

/// The Display Server composites guest video memory (VRAM) and telemetry HUD overlays.
pub struct DisplayServer {
    pub fb_addr: u64,
    pub width: usize,
    pub height: usize,
    pub host_buffer: Vec<u32>,
    pub frame_count: u64,
    fps_timer: Instant,
    pub last_fps: f64,
}

impl DisplayServer {
    pub fn new(fb_addr: u64, config: DisplayConfig) -> Self {
        let host_buffer = vec![0u32; config.width * config.height];
        Self {
            fb_addr,
            width: config.width,
            height: config.height,
            host_buffer,
            frame_count: 0,
            fps_timer: Instant::now(),
            last_fps: 0.0,
        }
    }

    /// Pulls the guest framebuffer from virtual memory and composites the telemetry HUD bar.
    pub fn present_frame(&mut self, mmu: &Memory) -> bool {
        let total_pixels = self.width * self.height;
        let mut raw_bytes = vec![0u8; total_pixels * 4];

        if mmu.read_bytes(self.fb_addr, &mut raw_bytes).is_ok() {
            for i in 0..total_pixels {
                let r = raw_bytes[i * 4] as u32;
                let g = raw_bytes[i * 4 + 1] as u32;
                let b = raw_bytes[i * 4 + 2] as u32;
                self.host_buffer[i] = (r << 16) | (g << 8) | b;
            }
        }

        self.draw_hud_overlay(mmu);

        self.frame_count += 1;
        let elapsed = self.fps_timer.elapsed();
        if elapsed.as_secs_f32() >= 1.0 {
            self.last_fps = self.frame_count as f64 / elapsed.as_secs_f64();
            self.frame_count = 0;
            self.fps_timer = Instant::now();
        }

        true
    }

    /// Renders a translucent top status bar visualizing memory bucket allocation & VSync heartbeat.
    fn draw_hud_overlay(&mut self, mmu: &Memory) {
        let hud_h = 16usize.min(self.height);
        let stats = mmu.bucket_stats();
        let total = stats.total_bytes.max(1);

        let code_w = ((stats.code_bytes * 240) / total).clamp(4, 240);
        let vram_w = ((stats.vram_bytes * 240) / total).clamp(4, 240);
        let stack_w = ((stats.stack_tls_bytes * 240) / total).clamp(4, 240);

        for y in 0..hud_h {
            for x in 0..self.width {
                let idx = y * self.width + x;
                let bg = self.host_buffer[idx];
                let r = ((bg >> 16) & 0xFF) / 3;
                let g = ((bg >> 8) & 0xFF) / 3;
                let b = (bg & 0xFF) / 3;
                let mut color = (r << 16) | (g << 8) | b;

                if (4..12).contains(&y) {
                    if (12..12 + code_w).contains(&x) {
                        color = 0x0000_E5FF; // Cyan = Executable Code
                    } else if (12 + code_w..12 + code_w + vram_w).contains(&x) {
                        color = 0x00FF_4081; // Magenta = VRAM / Framebuffer
                    } else if (12 + code_w + vram_w..12 + code_w + vram_w + stack_w).contains(&x) {
                        color = 0x0000_E676; // Emerald = Stack/TLS
                    }
                }
                self.host_buffer[idx] = color;
            }
        }
    }
}
