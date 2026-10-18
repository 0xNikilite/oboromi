//! The pushbuffer, shader program and texture descriptor encoders shared by the host tests and the guest program, the
//! way the NVN driver lays out a frame. Everything goes into caller-provided storage at caller-provided GPU virtual
//! addresses; allocation, submission and presentation stay with the caller.
#![no_std]

pub mod sass;
pub mod threed;

/// The host class methods every subchannel shares (clc76f.h).
pub mod host {
    pub const SET_OBJECT: usize = 0x0000;
    pub const SEMAPHORE_A: usize = 0x0010;
    pub const SEMAPHORE_B: usize = 0x0014;
    pub const SEMAPHORE_C: usize = 0x0018;
    pub const SEMAPHORE_D: usize = 0x001c;
}

use sass::{ald, ast, bra, exit, ffma, fmul, ipa, mov_cbuf, mov_imm};
use threed::*;

pub type Vertex = ([f32; 2], [u8; 4]);

pub const TRIANGLE_VERTICES: [Vertex; 3] = [([0.0, -0.8], [255, 0, 0, 255]), ([0.7, 0.4], [0, 255, 0, 255]), ([-0.7, 0.4], [0, 0, 255, 255])];

pub const fn header(op: u32, method: usize, count: u32) -> u32 {
    op << 29 | count << 16 | (method / 4) as u32
}

fn packet(op: u32, method: usize, values: &[u32], words: &mut impl Extend<u32>) {
    words.extend([header(op, method, values.len() as u32)]);
    words.extend(values.iter().copied());
}

pub fn inc(method: usize, values: &[u32], words: &mut impl Extend<u32>) {
    packet(1, method, values, words);
}

pub fn non_inc(method: usize, values: &[u32], words: &mut impl Extend<u32>) {
    packet(3, method, values, words);
}

pub fn immediate(method: usize, value: u32, words: &mut impl Extend<u32>) {
    words.extend([header(4, method, value)]);
}

pub fn one_inc(method: usize, values: &[u32], words: &mut impl Extend<u32>) {
    packet(5, method, values, words);
}

pub fn entry(address: u64, words: usize) -> [u8; 8] {
    let hi = ((address >> 32) as u32 & 0xff) | (words as u32) << 10;
    (u64::from(hi) << 32 | u64::from(address as u32)).to_le_bytes()
}

pub const fn split(va: u64) -> [u32; 2] {
    [(va >> 32) as u32, va as u32]
}

pub fn vertex(&([x, y], rgba): &Vertex) -> [u8; 12] {
    let mut bytes = [0; 12];
    bytes[..4].copy_from_slice(&x.to_le_bytes());
    bytes[4..8].copy_from_slice(&y.to_le_bytes());
    bytes[8..].copy_from_slice(&rgba);
    bytes
}

/// The shader program header (SPHV4, 32 words on SM86) the code follows.
pub const SPH_BYTES: usize = 0x80;

pub fn program(out: &mut [u8], header_bits: impl IntoIterator<Item = usize>, code: &[u128]) {
    let (header, rest) = out.split_at_mut(SPH_BYTES);
    header.fill(0);
    for bit in header_bits {
        header[bit / 8] |= 1 << (bit % 8);
    }
    for (i, inst) in code.iter().enumerate() {
        rest[16 * i..][..16].copy_from_slice(&inst.to_le_bytes());
    }
}

// The SPHV4 bits (clc797sph.h, version 4 at bit 7): type 1 vertex reading generics 0 and 1, writing position and generic
// 0; type 2 pixel reading generic 0 perspective-correct and writing RGBA of target 0. Position is c[0][0x0..0x10] times
// (x, y) at depth c[0][0x10].
const VERTEX_CODE: [u128; 16] = [
    ald(0, 0x80, 2),
    ald(4, 0x90, 4),
    mov_cbuf(8, 0, 0),
    mov_cbuf(9, 0, 4),
    mov_cbuf(10, 0, 8),
    mov_cbuf(11, 0, 12),
    fmul(12, 0, 8),
    ffma(12, 1, 9, 12),
    fmul(13, 0, 10),
    ffma(13, 1, 11, 13),
    mov_cbuf(14, 0, 16),
    mov_imm(15, 1.0f32.to_bits()),
    ast(0x70, 12, 4),
    ast(0x80, 4, 4),
    exit(),
    bra(-16),
];
const PIXEL_CODE: [u128; 6] = [ipa(0, 0x80, 0), ipa(1, 0x84, 0), ipa(2, 0x88, 0), ipa(3, 0x8c, 0), exit(), bra(-16)];
pub const VERTEX_PROGRAM_BYTES: usize = SPH_BYTES + 16 * VERTEX_CODE.len();
pub const PIXEL_PROGRAM_BYTES: usize = SPH_BYTES + 16 * PIXEL_CODE.len();

pub fn vertex_program(out: &mut [u8]) {
    program(out, [0, 7, 10].into_iter().chain(192..200).chain(428..436), &VERTEX_CODE);
}

pub fn pixel_program(out: &mut [u8]) {
    program(out, [1, 7, 10, 12, 193, 195, 197, 199, 576, 577, 578, 579], &PIXEL_CODE);
}

pub struct Addresses {
    pub vertex_program: u64,
    pub pixel_program: u64,
    pub vertices: u64,
    pub cbuf: u64,
    pub report: u64,
    pub target: u64,
}

/// The state every frame here starts from: both programs, one stream of `Vertex`, colour target 0, a full viewport and
/// scissor, and `matrix` inline in constant buffer 0 of the vertex stage.
pub fn frame_state(width: u32, height: u32, at: &Addresses, matrix: [f32; 4], words: &mut impl Extend<u32>) {
    const R32_G32_FLOAT: u32 = 0x04 << 21 | 7 << 27;
    const R8_G8_B8_A8_UNORM: u32 = 0x0a << 21 | 2 << 27;
    const A8B8G8R8: u32 = 0xd5;
    const PITCH: u32 = 0x1000;
    let [vs, fs, vertices, cbuf, target] = [at.vertex_program, at.pixel_program, at.vertices, at.cbuf, at.target].map(split);
    let (x, y) = (width as f32 / 2.0, height as f32 / 2.0);
    inc(host::SET_OBJECT, &[u32::from(CLASS_ID)], words);
    inc(SET_VERTEX_STREAM_SIZE_A, &[0, 0x1000], words);
    inc(SET_VERTEX_STREAM_A_FORMAT, &[0x1000 | 12, vertices[0], vertices[1]], words);
    inc(SET_VERTEX_ATTRIBUTE_A, &[R32_G32_FLOAT, 8 << 7 | R8_G8_B8_A8_UNORM], words);
    inc(SET_COLOR_TARGET_A, &[target[0], target[1], width * 4, height, A8B8G8R8, PITCH], words);
    inc(SET_VIEWPORT_SCALE_X, &[x, y, 0.5, x, y, 0.5].map(f32::to_bits), words);
    inc(SET_VIEWPORT_CLIP_HORIZONTAL, &[width << 16, height << 16, 0, 1.0f32.to_bits()], words);
    inc(SET_SCISSOR_ENABLE, &[1, width << 16, height << 16], words);
    inc(SET_PIPELINE_SHADER + 64, &[0x11], words);
    inc(SET_PIPELINE_SHADER + 64 + 0x10, &[0, vs[0], vs[1]], words);
    inc(SET_PIPELINE_SHADER + 5 * 64, &[0x51], words);
    inc(SET_PIPELINE_SHADER + 5 * 64 + 0x10, &[4, fs[0], fs[1]], words);
    inc(SET_CONSTANT_BUFFER_SELECTOR_A, &[32, cbuf[0], cbuf[1], 0], words);
    non_inc(LOAD_CONSTANT_BUFFER, &matrix.map(f32::to_bits), words);
    inc(BIND_GROUP_CONSTANT_BUFFER, &[1], words);
    inc(SET_CT_WRITE, &[0x1111], words);
}

/// Clears the target to white, draws the triangle with `matrix` inline in the constant buffer, then releases `payload`.
pub fn triangle_frame(width: u32, height: u32, at: &Addresses, matrix: [f32; 4], payload: u32, words: &mut impl Extend<u32>) {
    const ONE_WORD: u32 = 1 << 28;
    let report = split(at.report);
    frame_state(width, height, at, matrix, words);
    inc(SET_COLOR_CLEAR_VALUE, &[1.0f32.to_bits(); 4], words);
    inc(CLEAR_SURFACE, &[0x3c], words);
    inc(DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_FIRST, &[4 << 28 | 3 << 16], words);
    inc(SET_REPORT_SEMAPHORE_A, &[report[0], report[1], payload, ONE_WORD], words);
}

pub struct Scene<'a> {
    pub pushbuffer: &'a mut [u32],
    pub vertex_program: &'a mut [u8],
    pub pixel_program: &'a mut [u8],
    pub vertices: &'a mut [u8],
}

struct Words<'a> {
    storage: &'a mut [u32],
    len: usize,
}

impl Extend<u32> for Words<'_> {
    fn extend<I: IntoIterator<Item = u32>>(&mut self, words: I) {
        for word in words {
            self.storage[self.len] = word;
            self.len += 1;
        }
    }
}

/// Writes one triangle frame, returning the pushbuffer words it used.
pub fn triangle_scene(out: &mut Scene, at: &Addresses, [width, height]: [u32; 2], vertices: &[Vertex; 3], matrix: [f32; 4], payload: u32) -> usize {
    vertex_program(out.vertex_program);
    pixel_program(out.pixel_program);
    for (i, v) in vertices.iter().enumerate() {
        out.vertices[12 * i..][..12].copy_from_slice(&vertex(v));
    }
    let mut words = Words { storage: out.pushbuffer, len: 0 };
    triangle_frame(width, height, at, matrix, payload, &mut words);
    words.len
}

/// A pitch-linear 2D A8B8G8R8 unorm texture of `pitch` bytes per row (clc697tex.h).
pub const fn pitch_rgba8_tic(address: u64, width: u32, height: u32, pitch: u32) -> [u32; 8] {
    const A8B8G8R8_UNORM: u32 = 8 | 0b010_010_010_010 << 7;
    const IDENTITY_SWIZZLE: u32 = 0o5432 << 19;
    const PITCH: u32 = 2 << 21;
    const TWO_D: u32 = 1 << 23;
    const NORMALIZED: u32 = 1 << 31;
    let [hi, lo] = split(address);
    [A8B8G8R8_UNORM | IDENTITY_SWIZZLE, lo, hi | PITCH, pitch >> 5, (width - 1) | TWO_D, (height - 1) | NORMALIZED, 0, 0]
}

/// Nearest filtering clamped to the edge.
pub const NEAREST_CLAMP_TSC: [u32; 8] = [2 | 2 << 3, 1 | 1 << 4, 0, 0, 0, 0, 0, 0];
