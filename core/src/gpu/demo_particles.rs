//! The particles scene (demos/scenes/particles.md): each frame a QMD launch runs the NVIDIA driver's SM86 build of
//! demos/shaders/particles_update.comp over the particle buffer in guest memory, then one instanced draw of
//! particles.vert and particles.frag reads the same bytes as its vertex stream and adds a glow quad per particle.
use super::engine::Gpu;
use super::{VkState, device};
use crate::tests::gpu::engine::{
    BIND_GROUP_CONSTANT_BUFFER, CLEAR_SURFACE, DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_FIRST, LOAD_CONSTANT_BUFFER, SET_COLOR_CLEAR_VALUE,
    SET_COLOR_TARGET_A, SET_CONSTANT_BUFFER_SELECTOR_A, SET_CT_WRITE, SET_OBJECT, SET_PIPELINE_SHADER, SET_REPORT_SEMAPHORE_A,
    SET_SCISSOR_ENABLE, SET_VERTEX_ATTRIBUTE_A, SET_VERTEX_STREAM_A_FORMAT, SET_VERTEX_STREAM_SIZE_A, SET_VIEWPORT_CLIP_HORIZONTAL,
    SET_VIEWPORT_SCALE_X, bytes, entry, header, inc, non_inc,
};
use crate::tests::gpu_test::inst::bra;
use std::sync::Mutex;

pub const WIDTH: u32 = 1920;
pub const HEIGHT: u32 = 1080;
pub const COUNT: u32 = 8192;
const PARTICLE_BYTES: u32 = COUNT * 48;

const BASE: u64 = 0x1000_0000;
const PUSHBUFFER: u64 = BASE;
const UPDATE_SEGMENT: u64 = BASE + 0x1000;
const DRAW_SEGMENT: u64 = BASE + 0x2000;
const VERTEX_PROGRAM: u64 = BASE + 0x1_0000;
const PIXEL_PROGRAM: u64 = BASE + 0x1_4000;
const UPDATE_PROGRAM: u64 = BASE + 0x1_8000;
const VS_POINTERS: u64 = BASE + 0x2_0000;
const VS_UNIFORMS: u64 = BASE + 0x2_1000;
const CS_C0: u64 = BASE + 0x2_3000;
const CS_UNIFORMS: u64 = BASE + 0x2_4000;
const QMD: u64 = BASE + 0x3_0000;
pub const PARTICLES: u64 = BASE + 0x10_0000;
pub const TARGET: u64 = BASE + 0x20_0000;
pub const RELEASE: u64 = BASE + 0xa0_0000;
const MAPPED: u64 = 0xa1_0000;

// particles.vert loads its uniform block handle with `ULDC.64 UR4, c[0x1d][0x0]`.
const POINTER_BANK: u32 = 0x1d;
const WAIT_FOR_IDLE: u32 = 0x0110;
const SEND_PCAS_A: u32 = 0x02b4;
const SEND_SIGNALING_PCAS2_B: u32 = 0x02c0;
const INVALIDATE_COPY_SCHEDULE: u32 = 3;
const DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_SUBSEQUENT: u32 = 0x1218;
const SET_BLEND_SEPARATE_FOR_ALPHA: u32 = 0x133c;
const SET_BLEND: u32 = 0x1360;
const SET_VERTEX_STREAM_INSTANCE_A: u32 = 0x1880;
const STRIP_OF_FOUR: u32 = 5 << 28 | 4 << 16;

fn named(e: impl std::fmt::Debug) -> String {
    format!("{e:?}")
}

fn floats(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn compute(method: u32, values: &[u32], words: &mut Vec<u32>) {
    words.push(header(1, method, values.len() as u32) | 1 << 13);
    words.extend(values);
}

// A bindless cbuf handle, which cx[URa] operands read: the base address >> 4 in bits 0..45, the size >> 4 above
// (NVK nvk_bindless_cbuf).
fn handle(va: u64, size: u32) -> [u32; 2] {
    [(va >> 4) as u32, (va >> 36) as u32 | size >> 4 << 13]
}

/// attractor_a, attractor_b, step (dt, gravity, damping, time) of particles_update.comp at `seconds`.
pub fn update_uniforms(seconds: f32) -> [f32; 12] {
    let (sin, cos) = seconds.sin_cos();
    let [x, y, z] = [1.2 * cos, 0.4 * (2.0 * seconds).sin(), 1.2 * sin];
    [x, y, z, 6.0, -x, -y, -z, 4.0, 1.0 / 60.0, 0.5, 0.995, seconds]
}

/// view_proj column-major, camera right and camera up: fov 60 degrees from (0, 0.5, 4) at the origin, y down, z 0..1.
pub fn camera() -> ([f32; 16], [f32; 3], [f32; 3]) {
    let eye = [0.0, 0.5, 4.0];
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let unit = |a: [f32; 3]| a.map(|c| c / dot(a, a).sqrt());
    let forward = unit(eye.map(|c: f32| -c));
    let right = unit(cross(forward, [0.0, 1.0, 0.0]));
    let up = cross(right, forward);
    let [r, u, back] = [right, up, forward.map(|c| -c)].map(|axis| [axis[0], axis[1], axis[2], -dot(axis, eye)]);
    let (focal, near, far) = (1.0 / 30f32.to_radians().tan(), 0.1, 20.0);
    let depth = far / (near - far);
    let rows = [
        r.map(|c| c * focal * HEIGHT as f32 / WIDTH as f32),
        u.map(|c| -c * focal),
        [back[0] * depth, back[1] * depth, back[2] * depth, back[3] * depth + near * depth],
        back.map(|c| -c),
    ];
    (std::array::from_fn(|i| rows[i % 4][i / 4]), right, up)
}

// Positions in a 0.5 ball around attractor A at time 0, ages 0..1, lifetimes 2..4 s, colour 0 (particles.md).
fn initial_particles() -> Vec<u8> {
    let mut seed = 1u32;
    let mut random = || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        seed as f32 / 4_294_967_296.0
    };
    let attractor = update_uniforms(0.0);
    let mut particles = Vec::new();
    for _ in 0..COUNT {
        let offset = loop {
            let p = [random(), random(), random()].map(|c| c * 2.0 - 1.0);
            if p.iter().map(|c| c * c).sum::<f32>() <= 1.0 {
                break p;
            }
        };
        let [x, y, z] = [0, 1, 2].map(|c| attractor[c] + 0.5 * offset[c]);
        particles.extend([x, y, z, random(), 0.0, 0.0, 0.0, 1.0 / (2.0 + 2.0 * random()), 0.0, 0.0, 0.0, 0.0]);
    }
    floats(&particles)
}

// QMDV03_00 per particles.md: 32 CTAs of 256 threads, cbufs 0 and 1, release 0 of the frame number. 24 registers is
// the highest the SASS names (R23) plus one.
fn qmd(frame: u32) -> [u32; 64] {
    let mut qmd = [0; 64];
    [qmd[4], qmd[5]] = [0x40, if frame == 1 { 0xfc00_0000 } else { 0xa000_0000 }];
    [qmd[12], qmd[13], qmd[14]] = [COUNT / 256, 1, 1];
    [qmd[18], qmd[19], qmd[20]] = [256 << 16 | 3 << 4, 1 << 16 | 1, 24 << 8 | 3];
    [qmd[24], qmd[25], qmd[26]] = [RELEASE as u32, 0x4088_0000, frame];
    [qmd[32], qmd[33], qmd[34], qmd[35]] = [CS_C0 as u32, 0x1000 >> 4 << 19, CS_UNIFORMS as u32, 0x100 >> 4 << 19];
    qmd[48] = UPDATE_PROGRAM as u32;
    qmd
}

fn init_words() -> Vec<u32> {
    const A8B8G8R8: u32 = 0xd5;
    const PITCH: u32 = 0x1000;
    const R32_G32_B32_A32_FLOAT: u32 = 0x01 << 21 | 7 << 27;
    const OGL_FUNC_ADD: u32 = 0x8006;
    const OGL_ONE: u32 = 0x4001;
    let (x, y) = (WIDTH as f32 / 2.0, HEIGHT as f32 / 2.0);
    let mut words = Vec::new();
    inc(SET_OBJECT, &[0xc797], &mut words);
    compute(SET_OBJECT, &[0xc7c0], &mut words);
    inc(SET_COLOR_TARGET_A, &[0, TARGET as u32, WIDTH * 4, HEIGHT, A8B8G8R8, PITCH], &mut words);
    inc(SET_VIEWPORT_SCALE_X, &[x, y, 1.0, x, y, 0.0].map(f32::to_bits), &mut words);
    inc(SET_VIEWPORT_CLIP_HORIZONTAL, &[WIDTH << 16, HEIGHT << 16, 0, 1.0f32.to_bits()], &mut words);
    inc(SET_SCISSOR_ENABLE, &[1, WIDTH << 16, HEIGHT << 16], &mut words);
    inc(SET_COLOR_CLEAR_VALUE, &[0, 0, 0, 1.0f32.to_bits()], &mut words);
    inc(SET_CT_WRITE, &[0x1111], &mut words);
    inc(SET_BLEND_SEPARATE_FOR_ALPHA, &[0, OGL_FUNC_ADD, OGL_ONE, OGL_ONE], &mut words);
    inc(SET_BLEND, &[1], &mut words);
    inc(SET_VERTEX_STREAM_SIZE_A, &[0, PARTICLE_BYTES], &mut words);
    inc(SET_VERTEX_STREAM_A_FORMAT, &[0x1000 | 48, 0, PARTICLES as u32, 1], &mut words);
    inc(SET_VERTEX_STREAM_INSTANCE_A, &[1], &mut words);
    inc(SET_VERTEX_ATTRIBUTE_A, &[R32_G32_B32_A32_FLOAT, 32 << 7 | R32_G32_B32_A32_FLOAT], &mut words);
    // Register counts are the highest register each SASS program names plus one: R9 and R11.
    for (pipeline, enable, registers, group, program) in [(1, 0x11, 10, 0, VERTEX_PROGRAM), (5, 0x51, 12, 4, PIXEL_PROGRAM)] {
        inc(SET_PIPELINE_SHADER + pipeline * 64, &[enable], &mut words);
        inc(SET_PIPELINE_SHADER + pipeline * 64 + 0xc, &[registers, group, 0, program as u32], &mut words);
    }
    inc(SET_CONSTANT_BUFFER_SELECTOR_A, &[0x100, 0, VS_POINTERS as u32], &mut words);
    inc(BIND_GROUP_CONSTANT_BUFFER, &[POINTER_BANK << 4 | 1], &mut words);
    words
}

pub struct Particles<'a> {
    pub gpu: Gpu<'a>,
    frame: u32,
}

impl<'a> Particles<'a> {
    pub fn new(state: &'a VkState) -> Result<Self, String> {
        let mut gpu = Gpu::new(state, MAPPED as usize);
        gpu.space.map(BASE, 0, MAPPED);
        let program = |code: &[u8]| [code, &bra(-16).to_le_bytes()].concat();
        let (view_proj, right, up) = camera();
        let vs_uniforms = [&view_proj[..], &right, &[0.03], &up, &[0.0]].concat();
        let mut cs_c0 = [0; 19];
        [cs_c0[12], cs_c0[13]] = handle(CS_UNIFORMS, 0x100);
        [cs_c0[16], cs_c0[17], cs_c0[18]] = [PARTICLES as u32, 0, PARTICLE_BYTES];
        let init = init_words();
        for (va, data) in [
            (VERTEX_PROGRAM, program(include_bytes!("../../tests/fixtures/driver-shaders/particles.vert.bin"))),
            (PIXEL_PROGRAM, program(include_bytes!("../../tests/fixtures/driver-shaders/particles.frag.bin"))),
            (UPDATE_PROGRAM, program(include_bytes!("../../tests/fixtures/driver-shaders/particles_update.comp.bin"))),
            (VS_POINTERS, bytes(&handle(VS_UNIFORMS, 0x100))),
            (VS_UNIFORMS, floats(&vs_uniforms)),
            (CS_C0, bytes(&cs_c0)),
            (CS_UNIFORMS + 48, bytes(&[COUNT])),
            (PARTICLES, initial_particles()),
            (PUSHBUFFER, bytes(&init)),
        ] {
            gpu.write(va, &data).map_err(named)?;
        }
        gpu.submit(&[entry(PUSHBUFFER, init.len())])?;
        Ok(Self { gpu, frame: 0 })
    }

    /// Integrates every particle one step with the attractors at `seconds` and releases the frame number.
    pub fn update(&mut self, seconds: f32) -> Result<(), String> {
        self.frame += 1;
        self.gpu.write(QMD, &bytes(&qmd(self.frame))).map_err(named)?;
        let mut words = Vec::new();
        inc(SET_CONSTANT_BUFFER_SELECTOR_A, &[0x100, 0, CS_UNIFORMS as u32, 0], &mut words);
        non_inc(LOAD_CONSTANT_BUFFER, &update_uniforms(seconds).map(f32::to_bits), &mut words);
        inc(WAIT_FOR_IDLE, &[0], &mut words);
        compute(SEND_PCAS_A, &[(QMD >> 8) as u32], &mut words);
        compute(SEND_SIGNALING_PCAS2_B, &[INVALIDATE_COPY_SCHEDULE], &mut words);
        self.run(UPDATE_SEGMENT, &words)
    }

    /// Clears the target to black and adds one glow quad per particle.
    pub fn draw(&mut self) -> Result<(), String> {
        const ONE_WORD_RELEASE: u32 = 0x1000_f010;
        let mut words = Vec::new();
        inc(WAIT_FOR_IDLE, &[0], &mut words);
        inc(CLEAR_SURFACE, &[0x3c], &mut words);
        inc(DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_FIRST, &[STRIP_OF_FOUR], &mut words);
        non_inc(DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_SUBSEQUENT, &[STRIP_OF_FOUR; COUNT as usize - 1], &mut words);
        inc(SET_REPORT_SEMAPHORE_A, &[0, RELEASE as u32 + 0x10, self.frame, ONE_WORD_RELEASE], &mut words);
        self.run(DRAW_SEGMENT, &words)
    }

    fn run(&mut self, va: u64, words: &[u32]) -> Result<(), String> {
        self.gpu.write(va, &bytes(words)).map_err(named)?;
        self.gpu.submit(&[entry(va, words.len())])
    }
}

pub fn render_particles_frame(seconds: f32) -> Result<(), String> {
    static SCENE: Mutex<Option<Particles<'static>>> = Mutex::new(None);
    let mut slot = SCENE.lock().unwrap();
    let scene = match &mut *slot {
        Some(scene) => scene,
        empty => empty.insert(Particles::new(device().map_err(named)?)?),
    };
    scene.update(seconds)?;
    scene.draw()?;
    scene.gpu.present(TARGET, WIDTH, HEIGHT).map_err(named)
}
