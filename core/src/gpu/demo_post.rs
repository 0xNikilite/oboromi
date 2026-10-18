//! The post-effect scene (demos/scenes/post-effect.md): a spinning pinwheel drawn into one colour target, then that
//! target's VA sampled as TIC 4 by the NVIDIA driver's SM86 build of demos/shaders/post.vert and post.frag into a second.
use super::engine::Gpu;
use super::frame::cache_root;
use super::memory::PAGE;
use super::{VkState, device};
use crate::tests::gpu::engine::{bytes, entry, inc};
use crate::tests::gpu_test::inst::bra;
use std::path::Path;
use std::sync::Mutex;

const BASE: u64 = 0x1000_0000;
const PUSHBUFFER: u64 = BASE;
const FRAME_SEGMENT: u64 = BASE + 0x8000;
const SCENE_VERTEX: u64 = BASE + 0x1_0000;
const SCENE_PIXEL: u64 = BASE + 0x1_8000;
const UNIFORMS: u64 = BASE + 0x2_5000;
const POINTERS: u64 = BASE + 0x2_6000;
const TIC: u64 = BASE + 0x3_0000;
const TSC: u64 = BASE + 0x3_8000;
const REPORT: u64 = BASE + 0x3_f000;
const VERTICES: u64 = BASE + 0x4_0000;
const POST_VERTEX: u64 = BASE + 0x9_0000;
const POST_PIXEL: u64 = BASE + 0x9_8000;
pub const SCENE: u64 = BASE + 0xa_0000;

// post.frag reads its texture handle at c[0x1d][0x0] and its uniform block through the handle at c[0x1d][0x8]
// (`TEX ... 0x1d, 0x0`, `ULDC.64 UR4, c[0x1d][0x8]`).
const POINTER_BANK: u32 = 0x1d;
const TEXTURE_HANDLE: u32 = 4 | 1 << 20;
const BLADES: usize = 6;

const SET_OBJECT: usize = 0x0000;
const WAIT_FOR_IDLE: usize = 0x0110;
const SET_VERTEX_STREAM_SIZE_A: usize = 0x0600;
const SET_COLOR_TARGET_A: usize = 0x0800;
const SET_COLOR_TARGET_FORMAT: usize = 0x0810;
const SET_VIEWPORT_SCALE_X: usize = 0x0a00;
const SET_VIEWPORT_CLIP_HORIZONTAL: usize = 0x0c00;
const SET_COLOR_CLEAR_VALUE: usize = 0x0d80;
const SET_SCISSOR_ENABLE: usize = 0x0e00;
const SET_VERTEX_ATTRIBUTE_A: usize = 0x1160;
const DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_FIRST: usize = 0x1214;
const INVALIDATE_TEXTURE_DATA_CACHE: usize = 0x1338;
const SET_TEX_SAMPLER_POOL_A: usize = 0x155c;
const SET_TEX_HEADER_POOL_A: usize = 0x1574;
const CLEAR_SURFACE: usize = 0x19d0;
const SET_CT_WRITE: usize = 0x1a00;
const SET_REPORT_SEMAPHORE_A: usize = 0x1b00;
const SET_VERTEX_STREAM_A_FORMAT: usize = 0x1c00;
const SET_PIPELINE_SHADER: usize = 0x2000;
const SET_CONSTANT_BUFFER_SELECTOR_A: usize = 0x2380;
const BIND_GROUP_CONSTANT_BUFFER: usize = 0x2410;

fn split(va: u64) -> [u32; 2] {
    [(va >> 32) as u32, va as u32]
}

fn named(e: impl std::fmt::Debug) -> String {
    format!("{e:?}")
}

/// Colour target 0 at `va`, with the viewport and scissor covering it.
fn colour_target(va: u64, [width, height]: [u32; 2], words: &mut Vec<u32>) {
    let (x, y) = (width as f32 / 2.0, height as f32 / 2.0);
    inc(SET_COLOR_TARGET_A, &[split(va)[0], split(va)[1], width * 4, height], words);
    inc(SET_VIEWPORT_SCALE_X, &[x, y, 1.0, x, y, 0.0].map(f32::to_bits), words);
    inc(SET_VIEWPORT_CLIP_HORIZONTAL, &[width << 16, height << 16, 0, 1.0f32.to_bits()], words);
    inc(SET_SCISSOR_ENABLE, &[1, width << 16, height << 16], words);
}

fn programs(vertex: u64, pixel: u64, words: &mut Vec<u32>) {
    inc(SET_PIPELINE_SHADER + 64 + 0x14, &split(vertex), words);
    inc(SET_PIPELINE_SHADER + 5 * 64 + 0x14, &split(pixel), words);
}

pub struct Post<'a> {
    pub gpu: Gpu<'a>,
    pub target: u64,
    scene: [u32; 2],
    /// Where the frame segment holds the time it loads into `effect.x` and the payload of its report, and its length.
    time_word: u64,
    payload_word: u64,
    frame_words: usize,
    frame: u32,
}

impl<'a> Post<'a> {
    /// Draws the pinwheel at `scene` size and the post pass at `out` size, running the init segment.
    pub fn new(state: &'static VkState, cache: Option<(&Path, &str)>, scene: [u32; 2], out: [u32; 2]) -> Result<Self, String> {
        let target = (SCENE + u64::from(scene[0] * scene[1] * 4)).next_multiple_of(PAGE);
        let end = (target + u64::from(out[0] * out[1] * 4)).next_multiple_of(PAGE);
        let mut gpu = Gpu::new(state, (end - BASE) as usize, cache, *super::SPEED_HACKS.lock().unwrap()).map_err(named)?;
        gpu.space.map(BASE, 0, end - BASE);
        // The rig's programs stop at their last EXIT; the self-branch a compiled program ends with marks the end.
        for (va, program) in [
            (SCENE_VERTEX, &include_bytes!("../../tests/fixtures/driver-shaders/triangle.vert.bin")[..]),
            (SCENE_PIXEL, include_bytes!("../../tests/fixtures/driver-shaders/triangle.frag.bin")),
            (POST_VERTEX, include_bytes!("../../tests/fixtures/driver-shaders/post.vert.bin")),
            (POST_PIXEL, include_bytes!("../../tests/fixtures/driver-shaders/post.frag.bin")),
        ] {
            gpu.write(va, &[program, &bra(-16).to_le_bytes()].concat()).map_err(named)?;
        }
        let [width, height] = scene.map(|n| n as f32);
        let effect = [0.0, 6.0, 0.35, 0.55, width, height, 1.0 / width, 1.0 / height];
        gpu.write(UNIFORMS, &bytes(&effect.map(f32::to_bits))).map_err(named)?;
        // A bindless cbuf handle: the address >> 4 in bits 0..45 and the size >> 4 above (NVK nvk_bindless_cbuf).
        gpu.write(POINTERS, &bytes(&[TEXTURE_HANDLE, 0, (UNIFORMS >> 4) as u32, (UNIFORMS >> 36) as u32 | 32 >> 4 << 13])).map_err(named)?;
        const A8B8G8R8_UNORM: u32 = 8 | 0o2222 << 7 | 0o5432 << 19;
        const PITCH: u32 = 2 << 21;
        const TWO_D: u32 = 1 << 23;
        const NORMALIZED: u32 = 1 << 31;
        let tic = [A8B8G8R8_UNORM, split(SCENE)[1], split(SCENE)[0] | PITCH, scene[0] * 4 >> 5, (scene[0] - 1) | TWO_D, (scene[1] - 1) | NORMALIZED, 0, 0];
        gpu.write(TIC + 4 * 32, &bytes(&tic)).map_err(named)?;
        const CLAMP_TO_EDGE: u32 = 2 | 2 << 3 | 2 << 6;
        const LINEAR_NO_MIP: u32 = 2 | 2 << 4 | 1 << 6;
        gpu.write(TSC + 32, &bytes(&[CLAMP_TO_EDGE, LINEAR_NO_MIP])).map_err(named)?;

        const FLOAT: u32 = 7 << 27;
        const R32_G32_B32_A32: u32 = 0x01 << 21;
        let mut words = Vec::new();
        inc(SET_OBJECT, &[0xc797], &mut words);
        inc(SET_COLOR_TARGET_FORMAT, &[0xd5, 0x1000], &mut words);
        inc(SET_CT_WRITE, &[0x1111], &mut words);
        inc(SET_COLOR_CLEAR_VALUE, &[0.35f32, 0.4, 0.5, 1.0].map(f32::to_bits), &mut words);
        inc(SET_VERTEX_STREAM_A_FORMAT, &[0x1000 | 16, split(VERTICES)[0], split(VERTICES)[1]], &mut words);
        inc(SET_VERTEX_STREAM_SIZE_A, &split(16 * 3 * BLADES as u64), &mut words);
        inc(SET_VERTEX_ATTRIBUTE_A, &[R32_G32_B32_A32 | FLOAT], &mut words);
        inc(SET_PIPELINE_SHADER + 64, &[0x11], &mut words);
        inc(SET_PIPELINE_SHADER + 5 * 64, &[0x51], &mut words);
        inc(SET_PIPELINE_SHADER + 5 * 64 + 0x10, &[4], &mut words);
        inc(SET_CONSTANT_BUFFER_SELECTOR_A, &[16, split(POINTERS)[0], split(POINTERS)[1]], &mut words);
        inc(BIND_GROUP_CONSTANT_BUFFER + 4 * 32, &[POINTER_BANK << 4 | 1], &mut words);
        inc(SET_TEX_HEADER_POOL_A, &[split(TIC)[0], split(TIC)[1], 4], &mut words);
        inc(SET_TEX_SAMPLER_POOL_A, &[split(TSC)[0], split(TSC)[1], 1], &mut words);
        gpu.write(PUSHBUFFER, &bytes(&words)).map_err(named)?;
        gpu.submit(&[entry(PUSHBUFFER, words.len())])?;

        words.clear();
        colour_target(SCENE, scene, &mut words);
        programs(SCENE_VERTEX, SCENE_PIXEL, &mut words);
        inc(CLEAR_SURFACE, &[0x3c], &mut words);
        inc(DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_FIRST, &[4 << 28 | (3 * BLADES as u32) << 16], &mut words);
        inc(WAIT_FOR_IDLE, &[0], &mut words);
        inc(INVALIDATE_TEXTURE_DATA_CACHE, &[0], &mut words);
        colour_target(target, out, &mut words);
        programs(POST_VERTEX, POST_PIXEL, &mut words);
        inc(SET_CONSTANT_BUFFER_SELECTOR_A, &[32, split(UNIFORMS)[0], split(UNIFORMS)[1], 0, 0], &mut words);
        let time_word = FRAME_SEGMENT + 4 * (words.len() as u64 - 1);
        inc(DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_FIRST, &[4 << 28 | 3 << 16], &mut words);
        const ONE_WORD: u32 = 1 << 28;
        inc(SET_REPORT_SEMAPHORE_A, &[split(REPORT)[0], split(REPORT)[1], 0, ONE_WORD], &mut words);
        let payload_word = FRAME_SEGMENT + 4 * (words.len() as u64 - 2);
        gpu.write(FRAME_SEGMENT, &bytes(&words)).map_err(named)?;
        Ok(Self { gpu, target, scene, time_word, payload_word, frame_words: words.len(), frame: 0 })
    }

    /// Turns the pinwheel and the effect to `seconds`, runs the frame segment and waits for its report.
    pub fn frame(&mut self, seconds: f32) -> Result<(), String> {
        let aspect = self.scene[1] as f32 / self.scene[0] as f32;
        let corner = |angle: f32, radius: f32, [r, g]: [f32; 2]| {
            let (sin, cos) = angle.sin_cos();
            // triangle.vert colours a vertex (z, w, x, y) * 0.5 + 0.25.
            [cos * radius * aspect, sin * radius, (r - 0.25) * 2.0, (g - 0.25) * 2.0]
        };
        let vertices: Vec<u32> = (0..BLADES)
            .flat_map(|k| {
                let angle = seconds * 0.5 + k as f32 * std::f32::consts::TAU / BLADES as f32;
                let colour = if k % 2 == 0 { [1.0, 0.2] } else { [0.2, 0.9] };
                [corner(0.0, 0.0, [1.0, 1.0]), corner(angle - 0.25, 0.85, colour), corner(angle + 0.25, 0.85, colour)]
            })
            .flatten()
            .map(f32::to_bits)
            .collect();
        self.frame += 1;
        self.gpu.write(VERTICES, &bytes(&vertices)).map_err(named)?;
        self.gpu.write(self.time_word, &seconds.to_le_bytes()).map_err(named)?;
        self.gpu.write(self.payload_word, &self.frame.to_le_bytes()).map_err(named)?;
        self.gpu.submit(&[entry(FRAME_SEGMENT, self.frame_words)])?;
        let mut released = [0; 4];
        self.gpu.read(REPORT, &mut released).map_err(named)?;
        match u32::from_le_bytes(released) == self.frame {
            true => Ok(()),
            false => Err(format!("frame {} did not release its report", self.frame)),
        }
    }
}

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;

pub(super) static POST: Mutex<Option<Post<'static>>> = Mutex::new(None);

pub fn render_post_frame(seconds: f32) -> Result<(), String> {
    let mut slot = POST.lock().unwrap();
    let post = match &mut *slot {
        Some(post) => post,
        empty => empty.insert(Post::new(device().map_err(named)?, Some((&cache_root(), "demos")), [1280, 720], [WIDTH, HEIGHT])?),
    };
    post.frame(seconds)?;
    post.gpu.present(post.target, WIDTH, HEIGHT).map_err(named)
}
