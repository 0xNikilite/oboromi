//! Frames drawn by translated SASS programs that write RGBA8 pixels into global memory, presented as is.
use super::compute::{ComputePipeline, HostBuffer};
use super::sm86::{CBUF_BINDING, GLOBAL_BINDING, SUPPORT_BINDING, Stage, emit_module};
use super::demo::SCENE;
use super::demo_post::POST;
use super::engine::Gpu;
use super::{VkError, device};
use crate::tests::gpu::engine::{TARGET, triangle_frame, triangle_guest};
use crate::tests::gpu_test::inst::{cvt, *};
use ash::vk::DescriptorType;
use std::path::PathBuf;
use std::sync::Mutex;

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const BASE: u64 = 0x2_0000_1000;
const PP_PT: u128 = 7 << 87;
const IMAD: u128 = 0x224 | S32 | 7 << 81 | PP_PT;
const IMAD_WIDE_U32: u128 = 0x225 | 7 << 81 | PP_PT;
const ISETP_GE_U32: u128 = 0x20c | 6 << 76 | 7 << 68 | 7 << 84;
const BOP_OR: u128 = 1 << 74;
const P0: u32 = 0;
const RZ: u32 = 255;
const GROUP: u32 = 16;
const MAX_ITERATIONS: u32 = 64;
const ESCAPE_RADIUS_SQUARED: f32 = 256.0;
const UNITS_PER_PIXEL: f32 = 2.6 / HEIGHT as f32;
const C_RADIUS: f32 = 0.7885;
const SECONDS_PER_TURN: f32 = 16.0;

// Runs `colour` with x in R0 and y in R2, then stores R1 as the pixel's RGBA8.
fn pixel_program(colour: &[u128]) -> Vec<u128> {
    let head = [
        s2r(0, 37),
        s2r(1, 33),
        s2r(2, 38),
        s2r(3, 34),
        alu3(IMAD ^ IMM_B, 0, 0, GROUP, 1),
        alu3(IMAD ^ IMM_B, 2, 2, GROUP, 3),
        alu(ISETP_GE_U32 ^ IMM_B | PP_PT, 0, 0, WIDTH),
        alu(ISETP_GE_U32 ^ IMM_B | BOP_OR, 0, 2, HEIGHT),
        guard(exit(), P0, false),
    ];
    let store = [
        alu3(IMAD ^ IMM_B, 0, 2, WIDTH, 0),
        mov_imm(4, BASE as u32),
        mov_imm(5, (BASE >> 32) as u32),
        alu3(IMAD_WIDE_U32 ^ IMM_B, 4, 0, 4, 4),
        stg(4, 0, 1),
        exit(),
    ];
    [&head, colour, &store].concat()
}

// r = x * 17 >> 7 and g = y * 495176 >> 21 are x * 255 / 1920 and y * 255 / 1080 exactly over the frame.
// The u32 is stored little-endian, so r is byte 0 and a is byte 3 of each RGBA8 pixel.
fn gradient_program() -> Vec<u128> {
    let shr = |rd, rs, n| alu3(SHF ^ IMM_B | SHF_R | SHF_U32 | SHF_HI, rd, RZ, n, rs);
    pixel_program(&[
        alu3(IMAD ^ IMM_B, 1, 0, 17, RZ),
        shr(1, 1, 7),
        alu3(IMAD ^ IMM_B, 3, 2, 495176, RZ),
        shr(3, 3, 21),
        alu3(IMAD ^ IMM_B, 1, 3, 0x100, 1),
        alu3(LOP3 ^ IMM_B | lop3_fields(0xfc, 7, 7, true), 1, 1, 0xff80_0000, RZ),
    ])
}

// Julia set z = z * z + c with c circling the origin as the time in turns at c[0x0][0x0] grows,
// coloured by i - log2(log2(|z|^2)) through a cosine palette and black where z never escapes.
pub fn julia_program() -> Vec<u128> {
    let channel = |rd, phase| [fadd_imm(rd, 14, phase), mufu(rd, MUFU_COS, rd), ffma_imm_b(rd, rd, 127.5, 15), f2i(rd, rd, cvt::S32, cvt::F32, cvt::RN)];
    let iterate = [
        fmul(12, 7, 7),
        ffma(13, 6, 6, 12),
        alu(FSETP_GT ^ IMM_B, 0, 13, ESCAPE_RADIUS_SQUARED.to_bits()),
        alu(ISETP_GE_U32 ^ IMM_B | BOP_OR, 0, 10, MAX_ITERATIONS),
        guard(bra(6 * 16), P0, false),
        fmul(14, 6, 7),
        ffma_imm_b(7, 14, 2.0, 9),
        ffma(14, 6, 6, 12) | FP_NEG_C,
        fadd(6, 14, 8),
        iadd_imm(10, 10, 1),
        bra(-11 * 16),
    ];
    let start = [
        i2f(6, 0, cvt::F32, cvt::S32),
        i2f(7, 2, cvt::F32, cvt::S32),
        fadd_imm(6, 6, -(WIDTH as f32) / 2.0),
        fmul_imm(6, 6, UNITS_PER_PIXEL),
        fadd_imm(7, 7, -(HEIGHT as f32) / 2.0),
        fmul_imm(7, 7, -UNITS_PER_PIXEL),
        mov_cbuf(11, 0, 0),
        mufu(8, MUFU_COS, 11),
        mufu(9, MUFU_SIN, 11),
        fmul_imm(8, 8, C_RADIUS),
        fmul_imm(9, 9, C_RADIUS),
        mov_imm(10, 0),
    ];
    let shade = [
        mufu(13, MUFU_LG2, 13),
        mufu(13, MUFU_LG2, 13),
        i2f(14, 10, cvt::F32, cvt::S32),
        fadd(14, 14, 13) | FP_NEG_B,
        ffma_imm_b(14, 14, 0.07, 11),
        mov_imm(15, 127.5f32.to_bits()),
    ];
    let pack = [
        alu3(IMAD ^ IMM_B, 1, 17, 0x100, 16),
        alu3(IMAD ^ IMM_B, 1, 18, 0x1_0000, 1),
        alu3(LOP3 ^ IMM_B | lop3_fields(0xfc, 7, 7, true), 1, 1, 0xff00_0000, RZ),
        alu(ISETP_GE_U32 ^ IMM_B | PP_PT, 0, 10, MAX_ITERATIONS),
        guard(mov_imm(1, 0xff00_0000), P0, false),
    ];
    pixel_program(&[&start[..], &iterate, &shade, &channel(16, 0.0), &channel(17, 0.1), &channel(18, 0.2), &pack].concat())
}

struct Frame {
    pipeline: ComputePipeline<'static>,
    pixels_buffer: HostBuffer<'static>,
    cbuf0: HostBuffer<'static>,
    _support: HostBuffer<'static>,
    pixels: Vec<u8>,
}

impl Frame {
    fn new(program: &[u128]) -> Result<Self, VkError> {
        let state = device()?;
        let size = u64::from(WIDTH * HEIGHT * 4);
        let module = emit_module(Some(state), Stage::Compute { local_size: [GROUP, GROUP, 1] }, |decoder| decoder.translate_program(program));
        let pixels_buffer = HostBuffer::new(state, size)?;
        let cbuf0 = HostBuffer::new(state, 16)?;
        let mut support = HostBuffer::new(state, 16)?;
        support.write(&BASE.to_le_bytes())?;
        let ubo = DescriptorType::UNIFORM_BUFFER;
        let bindings = [(SUPPORT_BINDING, ubo), (CBUF_BINDING, ubo), (GLOBAL_BINDING, DescriptorType::STORAGE_BUFFER)];
        let mut pipeline = ComputePipeline::new(state, &module.words(), &bindings)?;
        pipeline.descriptors.bind(&[(SUPPORT_BINDING, &support), (CBUF_BINDING, &cbuf0), (GLOBAL_BINDING, &pixels_buffer)]);
        Ok(Self { pipeline, pixels_buffer, cbuf0, _support: support, pixels: vec![0; size as usize] })
    }

    fn render(&mut self) -> Result<(), VkError> {
        // Round up so 1080 rows get a partial last group; the program's guard drops the overhang.
        device()?.dispatch(&self.pipeline, [WIDTH.div_ceil(GROUP), HEIGHT.div_ceil(GROUP), 1])?;
        self.pixels_buffer.read(&mut self.pixels)?;
        crate::display::present(WIDTH, HEIGHT, &self.pixels);
        Ok(())
    }
}

pub fn render_test_frame() -> Result<(), VkError> {
    Frame::new(&gradient_program())?.render()
}

pub fn render_demo_frame(seconds: f32) -> Result<(), VkError> {
    static DEMO: Mutex<Option<Frame>> = Mutex::new(None);
    let mut slot = DEMO.lock().unwrap();
    let demo = match &mut *slot {
        Some(demo) => demo,
        empty => empty.insert(Frame::new(&julia_program())?),
    };
    demo.cbuf0.write(&(seconds / SECONDS_PER_TURN).to_le_bytes())?;
    demo.render()
}

static TRIANGLE_DEMO: Mutex<Option<Gpu<'static>>> = Mutex::new(None);

/// `$XDG_CACHE_HOME/oboromi`, or `~/.cache/oboromi`, or the temp dir's when there is no home.
pub(super) fn cache_root() -> PathBuf {
    let home = || std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache"));
    std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from).or_else(home).unwrap_or_else(std::env::temp_dir).join("oboromi")
}

/// Draws a triangle from a pushbuffer in guest memory, turning once per `SECONDS_PER_TURN`, and presents it.
pub fn render_triangle_demo(seconds: f32) -> Result<(), String> {
    let mut slot = TRIANGLE_DEMO.lock().unwrap();
    let gpu = match &mut *slot {
        Some(gpu) => gpu,
        empty => {
            let vertices = [([0.0, -0.8], [255, 0, 0, 255]), ([0.7, 0.4], [0, 255, 0, 255]), ([-0.7, 0.4], [0, 0, 255, 255])];
            let state = device().map_err(|e| format!("{e:?}"))?;
            empty.insert(triangle_guest(state, Some((&cache_root(), "demos")), WIDTH, HEIGHT, &vertices).map_err(|e| format!("{e:?}"))?)
        }
    };
    let (sin, cos) = (seconds / SECONDS_PER_TURN * std::f32::consts::TAU).sin_cos();
    let aspect = HEIGHT as f32 / WIDTH as f32;
    triangle_frame(gpu, WIDTH, HEIGHT, [cos * aspect, -sin * aspect, sin, cos], 0)?;
    gpu.present(TARGET, WIDTH, HEIGHT).map_err(|e| format!("{e:?}"))
}

/// Saves the running demos' pipeline cache, so the next run starts warm.
pub fn flush_demo_caches() -> std::io::Result<()> {
    TRIANGLE_DEMO.lock().unwrap().as_ref().map_or(Ok(()), Gpu::flush_caches)?;
    POST.lock().unwrap().as_ref().map_or(Ok(()), |post| post.gpu.flush_caches())?;
    SCENE.lock().unwrap().as_ref().map_or(Ok(()), |(_, scene)| scene.gpu.flush_caches())
}
