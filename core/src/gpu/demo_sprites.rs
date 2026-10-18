//! The sprites2d scene (demos/scenes/sprites2d.md): a sun, three scrolling hill layers and the sprites drifting between
//! them, drawn back to front as one instanced triangle strip a frame by the NVIDIA driver's SM86 build of
//! demos/shaders/sprite2d.vert and sprite2d.frag from a host-written pushbuffer.
use super::engine::Gpu;
use super::frame::cache_root;
use super::memory::PAGE;
use super::{VkState, device};
use crate::tests::gpu::engine::*;
use crate::tests::gpu_test::inst::bra;
use std::f32::consts::{FRAC_1_SQRT_2, TAU};
use std::path::Path;
use std::sync::Mutex;

const BASE: u64 = 0x1000_0000;
const PUSHBUFFER: u64 = BASE;
const FRAME_SEGMENT: u64 = BASE + 0x8000;
const VERTEX_PROGRAM: u64 = BASE + 0x1_0000;
const PIXEL_PROGRAM: u64 = BASE + 0x1_8000;
const VERTEX_TABLE: u64 = BASE + 0x2_0000;
const CAMERA: u64 = BASE + 0x2_1000;
const PIXEL_TABLE: u64 = BASE + 0x2_2000;
const TINT: u64 = BASE + 0x2_3000;
const TIC: u64 = BASE + 0x3_0000;
const TSC: u64 = BASE + 0x3_8000;
const INSTANCES: u64 = BASE + 0x4_0000;
const ATLAS: u64 = BASE + 0x5_0000;
pub const TARGET: u64 = BASE + 0x105_0000;

const SET_BLEND_SEPARATE_FOR_ALPHA: u32 = 0x133c;
const SET_BLEND_ALPHA_DEST_COEFF: u32 = 0x1358;
const SET_BLEND: u32 = 0x1360;
const SET_VERTEX_STREAM_INSTANCE_A: u32 = 0x1880;
const DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_SUBSEQUENT: u32 = 0x1218;
const FUNC_ADD: u32 = 0x8006;
const ONE: u32 = 0x4001;
const SRC_ALPHA: u32 = 0x4302;
const ONE_MINUS_SRC_ALPHA: u32 = 0x4303;
const TRIANGLE_STRIP_OF_FOUR: u32 = 5 << 28 | 4 << 16;

// Both programs read their binding 1 block through the bindless cbuf handle at c[0x1d][0x8] and the pixel program its
// atlas through the texture handle at c[0x1d][0x0] (sprite2d vert.sass 0x30, frag.sass 0x60 and 0x80).
const DRIVER_BANK: u32 = 0x1d;

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
/// The scene is laid out in 1280x720 pixels whatever the target's size.
const SCENE: [f32; 2] = [1280.0, 720.0];
pub const SKY: [f32; 4] = [0.55, 0.7, 0.9, 1.0];
pub const SUNSET: [f32; 4] = [1.0, 0.9, 0.8, 1.0];
const SCROLL_PER_SECOND: f32 = 120.0;
const PARALLAX: [f32; 3] = [0.15, 0.4, 0.8];
const SPRITES_PER_LAYER: usize = 65;
pub const INSTANCE_COUNT: usize = 1 + PARALLAX.len() * (1 + SPRITES_PER_LAYER);
pub const ATLAS_SIZE: usize = 2048;
const CELL: usize = ATLAS_SIZE / 4;
const HILLS: [[u8; 3]; 3] = [[150, 182, 205], [92, 140, 112], [40, 82, 62]];
pub const SHAPES: [[u8; 3]; 4] = [[255, 214, 92], [236, 92, 92], [255, 255, 255], [112, 200, 255]];
const SUN: [f32; 4] = [1080.0, 90.0, 48.0, 48.0];

// Rows CELL * k.. hold hill k, opaque below a two-harmonic ridge with k + 1 periods across the row so it wraps seamlessly,
// shaded darker towards the bottom; the last CELL rows hold a disc, a ring, a star and a diamond. Edges are a few texels
// wide so the linear filter stays smooth when the 2048 texels are minified.
pub fn atlas() -> Vec<u8> {
    let scale = CELL as f32 / 64.0;
    let texel = |x: usize, y: usize| {
        let [cx, cy] = [x % CELL, y % CELL].map(|c| c as f32 + 0.5 - CELL as f32 / 2.0);
        let (rgb, distance, edge, shade) = match y / CELL {
            row @ 0..3 => {
                let k = row as f32;
                let phase = TAU * (k + 1.0) * (x as f32 + 0.5) / ATLAS_SIZE as f32;
                let ridge = (10.0 + 6.0 * k) * scale * (phase.sin() + 0.3 * (3.0 * phase + k).sin());
                (HILLS[row], ridge - cy, 3.0, 1.0 - (0.3 * (cy - ridge) / (CELL as f32 / 2.0)).clamp(0.0, 0.3))
            }
            _ => {
                let r = cx.hypot(cy);
                let distance = match x / CELL {
                    0 => r - 28.0 * scale,
                    1 => (r - 22.0 * scale).abs() - 6.0 * scale,
                    2 => r - (18.0 + 9.0 * (5.0 * cy.atan2(cx)).cos()) * scale,
                    _ => (cx.abs() + cy.abs() - 28.0 * scale) * FRAC_1_SQRT_2,
                };
                (SHAPES[x / CELL], distance, 10.0, 1.0)
            }
        };
        let [r, g, b] = rgb.map(|c| (f32::from(c) * shade).round() as u8);
        [r, g, b, ((0.5 - distance / edge).clamp(0.0, 1.0) * 255.0).round() as u8]
    };
    (0..ATLAS_SIZE * ATLAS_SIZE).flat_map(|i| texel(i % ATLAS_SIZE, i / ATLAS_SIZE)).collect()
}

/// Per instance: transform (centre, half size), uv rect (origin, size) and layer (position parallax, uv parallax,
/// rotation, alpha); the sun first, then each hill followed by the sprites in front of it.
pub fn instances(seconds: f32) -> Vec<[f32; 12]> {
    let scroll = seconds * SCROLL_PER_SECOND;
    let cell = |shape: usize| [0.25 * shape as f32, 0.75, 0.25, 0.25];
    let mut instances = vec![[SUN, cell(0), [0.0, 0.0, 0.0, 0.8]].concat().try_into().unwrap()];
    for (k, parallax) in PARALLAX.into_iter().enumerate() {
        let depth = k as f32;
        let hill = [640.0, [520.0, 560.0, 620.0][k], 640.0, [200.0, 160.0, 100.0][k], 0.0, 0.25 * depth, 3.0 - 0.5 * depth, 0.25, 0.0, parallax, 0.0, 1.0];
        instances.push(hill);
        for i in 0..SPRITES_PER_LAYER {
            let n = (k * SPRITES_PER_LAYER + i) as f32;
            let half = (6.0 + 6.0 * depth) * (1.0 + 0.6 * (n * 0.31).fract());
            // The host wraps each sprite back across the screen; the vertex program subtracts the parallax scroll.
            let period = SCENE[0] + 128.0;
            let x = ((n * 0.618_034).fract() * period - scroll * parallax).rem_euclid(period) - 64.0 + scroll * parallax;
            let y = 160.0 + (n * 0.414_214).fract() * 300.0 + 10.0 * (1.3 * seconds + n).sin();
            let spin = if i % 2 == 0 { 1.0 } else { -1.0 } * (0.5 + (n * 0.43).fract());
            let layer = [parallax, 0.0, 0.37 * n + spin * seconds, 0.6 + 0.175 * depth];
            instances.push([[x, y, half, half], cell(i % 4), layer].concat().try_into().unwrap());
        }
    }
    instances
}

fn load(buffer: u64, values: &[f32], words: &mut Vec<u32>) {
    inc(SET_CONSTANT_BUFFER_SELECTOR_A, &[16, 0, buffer as u32, 0], words);
    non_inc(LOAD_CONSTANT_BUFFER, &values.iter().map(|v| v.to_bits()).collect::<Vec<_>>(), words);
}

fn init_segment(width: u32, height: u32) -> Vec<u32> {
    const R32_G32_B32_A32_FLOAT: u32 = 0x01 << 21 | 7 << 27;
    const A8B8G8R8: u32 = 0xd5;
    const PITCH: u32 = 0x1000;
    let (x, y) = (width as f32 / 2.0, height as f32 / 2.0);
    let mut words = Vec::new();
    inc(SET_OBJECT, &[0xc797], &mut words);
    inc(SET_COLOR_TARGET_A, &[0, TARGET as u32, width * 4, height, A8B8G8R8, PITCH], &mut words);
    inc(SET_VIEWPORT_SCALE_X, &[x, y, 1.0, x, y, 0.0].map(f32::to_bits), &mut words);
    inc(SET_VIEWPORT_CLIP_HORIZONTAL, &[width << 16, height << 16, 0, 1.0f32.to_bits()], &mut words);
    inc(SET_SCISSOR_ENABLE, &[1, width << 16, height << 16], &mut words);
    inc(SET_COLOR_CLEAR_VALUE, &SKY.map(f32::to_bits), &mut words);
    inc(SET_CT_WRITE, &[0x1111], &mut words);
    inc(SET_BLEND_SEPARATE_FOR_ALPHA, &[1, FUNC_ADD, SRC_ALPHA, ONE_MINUS_SRC_ALPHA, FUNC_ADD, ONE], &mut words);
    inc(SET_BLEND_ALPHA_DEST_COEFF, &[ONE_MINUS_SRC_ALPHA], &mut words);
    inc(SET_BLEND, &[1], &mut words);
    inc(SET_VERTEX_STREAM_SIZE_A, &[0, (INSTANCE_COUNT * 48) as u32], &mut words);
    inc(SET_VERTEX_STREAM_A_FORMAT, &[0x1000 | 48, 0, INSTANCES as u32, 1], &mut words);
    inc(SET_VERTEX_STREAM_INSTANCE_A, &[1], &mut words);
    inc(SET_VERTEX_ATTRIBUTE_A, &[0, 16, 32].map(|offset| offset << 7 | R32_G32_B32_A32_FLOAT), &mut words);
    inc(SET_PIPELINE_SHADER + 64, &[0x11], &mut words);
    inc(SET_PIPELINE_SHADER + 64 + 0x10, &[0, 0, VERTEX_PROGRAM as u32], &mut words);
    inc(SET_PIPELINE_SHADER + 5 * 64, &[0x51], &mut words);
    inc(SET_PIPELINE_SHADER + 5 * 64 + 0x10, &[4, 0, PIXEL_PROGRAM as u32], &mut words);
    for (group, table) in [(0, VERTEX_TABLE), (4, PIXEL_TABLE)] {
        inc(SET_CONSTANT_BUFFER_SELECTOR_A, &[16, 0, table as u32], &mut words);
        inc(BIND_GROUP_CONSTANT_BUFFER + 32 * group, &[DRIVER_BANK << 4 | 1], &mut words);
    }
    inc(SET_TEX_HEADER_POOL_A, &[0, TIC as u32, 0], &mut words);
    inc(SET_TEX_SAMPLER_POOL_A, &[0, TSC as u32, 0], &mut words);
    load(CAMERA, &[0.0, 0.0, 2.0 / SCENE[0], 2.0 / SCENE[1]], &mut words);
    load(TINT, &SUNSET, &mut words);
    words
}

/// Guest memory holding the scene with its init segment run, drawing into a `width` x `height` target at `TARGET`.
pub fn sprites_guest(state: &'static VkState, cache: Option<(&Path, &str)>, width: u32, height: u32) -> Result<Gpu<'static>, String> {
    sprites_guest_with_programs(state, cache, width, height, [
        include_bytes!("../../tests/fixtures/driver-shaders/sprite2d.vert.bin"),
        include_bytes!("../../tests/fixtures/driver-shaders/sprite2d.frag.bin"),
    ])
}

pub(crate) fn sprites_guest_with_programs(state: &'static VkState, cache: Option<(&Path, &str)>, width: u32, height: u32, programs: [&[u8]; 2]) -> Result<Gpu<'static>, String> {
    let named = |e| format!("{e:?}");
    let len = (TARGET - BASE + u64::from(width * height) * 4).next_multiple_of(PAGE);
    let mut gpu = Gpu::new(state, len as usize, cache, *super::SPEED_HACKS.lock().unwrap()).map_err(|e| format!("{e:?}"))?;
    gpu.space.map(BASE, 0, len);
    // The rig's programs stop at their last EXIT; the self-branch a compiled program ends with marks the end.
    for (va, program) in [VERTEX_PROGRAM, PIXEL_PROGRAM].into_iter().zip(programs) {
        gpu.write(va, &[program, &bra(-16).to_le_bytes()].concat()).map_err(named)?;
    }
    // A bindless cbuf handle: the address >> 4 in bits 0..45 and the size >> 4 above (NVK nvk_bindless_cbuf).
    let camera = [0, 0, (CAMERA >> 4) as u32, 1 << 13];
    let tint = [0, 0, (TINT >> 4) as u32, 1 << 13];
    // A8B8G8R8 UNORM pitch-linear 2048x2048 at ATLAS with pitch 8192 (clc797tex.h), sampled with repeat wrap and linear filters.
    let tic = [0x58d2_4908, ATLAS as u32, 2 << 21, 8192 >> 5, 2047 | 1 << 23, 2047 | 1 << 31, 0, 0];
    let tsc = [0, 0x62, 0, 0, 0, 0, 0, 0];
    for (va, words) in [(VERTEX_TABLE, &camera[..]), (PIXEL_TABLE, &tint), (TIC, &tic), (TSC, &tsc)] {
        gpu.write(va, &bytes(words)).map_err(named)?;
    }
    gpu.write(ATLAS, &atlas()).map_err(named)?;
    let words = init_segment(width, height);
    gpu.write(PUSHBUFFER, &bytes(&words)).map_err(named)?;
    gpu.submit(&[entry(PUSHBUFFER, words.len())])?;
    Ok(gpu)
}

/// Writes the instances for `seconds` and runs the frame segment: scroll the camera, clear to the sky, draw.
pub fn sprites_frame(gpu: &mut Gpu, seconds: f32) -> Result<(), String> {
    let named = |e| format!("{e:?}");
    gpu.write(INSTANCES, &instances(seconds).iter().flatten().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>()).map_err(named)?;
    let mut words = Vec::new();
    load(CAMERA, &[seconds * SCROLL_PER_SECOND], &mut words);
    inc(CLEAR_SURFACE, &[0x3c], &mut words);
    inc(DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_FIRST, &[TRIANGLE_STRIP_OF_FOUR], &mut words);
    non_inc(DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_SUBSEQUENT, &[TRIANGLE_STRIP_OF_FOUR; INSTANCE_COUNT - 1], &mut words);
    gpu.write(FRAME_SEGMENT, &bytes(&words)).map_err(named)?;
    gpu.submit(&[entry(FRAME_SEGMENT, words.len())])
}

pub(super) static SPRITES: Mutex<Option<Gpu<'static>>> = Mutex::new(None);

/// Draws the sprites scene at `seconds` and presents it.
pub fn render_sprites_frame(seconds: f32) -> Result<(), String> {
    let mut slot = SPRITES.lock().unwrap();
    let gpu = match &mut *slot {
        Some(gpu) => gpu,
        empty => empty.insert(sprites_guest(device().map_err(|e| format!("{e:?}"))?, Some((&cache_root(), "demos")), WIDTH, HEIGHT)?),
    };
    sprites_frame(gpu, seconds)?;
    gpu.present(TARGET, WIDTH, HEIGHT).map_err(|e| format!("{e:?}"))
}
