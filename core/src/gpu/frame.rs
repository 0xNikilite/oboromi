//! Test frame: a translated SASS program writes a gradient into global memory that is presented as is.
use super::compute::{ComputePipeline, HostBuffer};
use super::sm86::{GLOBAL_BINDING, SUPPORT_BINDING, emit_module};
use super::{VkError, device};
use crate::tests::gpu_test::inst::*;
use ash::vk::DescriptorType;

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

// r = x * 17 >> 7 and g = y * 495176 >> 21 are x * 255 / 1920 and y * 255 / 1080 exactly over the frame.
// The u32 is stored little-endian, so r is byte 0 and a is byte 3 of each RGBA8 pixel.
fn gradient_program() -> [u128; 21] {
    let shr = |rd, rs, n| alu3(SHF ^ IMM_B | SHF_R | SHF_U32 | SHF_HI, rd, RZ, n, rs);
    [
        s2r(0, 37),
        s2r(1, 33),
        s2r(2, 38),
        s2r(3, 34),
        alu3(IMAD ^ IMM_B, 0, 0, GROUP, 1),
        alu3(IMAD ^ IMM_B, 2, 2, GROUP, 3),
        alu(ISETP_GE_U32 ^ IMM_B | PP_PT, 0, 0, WIDTH),
        alu(ISETP_GE_U32 ^ IMM_B | BOP_OR, 0, 2, HEIGHT),
        guard(exit(), P0, false),
        alu3(IMAD ^ IMM_B, 1, 0, 17, RZ),
        shr(1, 1, 7),
        alu3(IMAD ^ IMM_B, 3, 2, 495176, RZ),
        shr(3, 3, 21),
        alu3(IMAD ^ IMM_B, 1, 3, 0x100, 1),
        alu3(LOP3 ^ IMM_B | lop3_fields(0xfc, 7, 7, true), 1, 1, 0xff80_0000, RZ),
        alu3(IMAD ^ IMM_B, 0, 2, WIDTH, 0),
        mov_imm(4, BASE as u32),
        mov_imm(5, (BASE >> 32) as u32),
        alu3(IMAD_WIDE_U32 ^ IMM_B, 4, 0, 4, 4),
        stg(4, 0, 1),
        exit(),
    ]
}

pub fn render_test_frame() -> Result<(), VkError> {
    let state = device()?;
    let size = u64::from(WIDTH * HEIGHT * 4);
    let module = emit_module(Some(state), [GROUP, GROUP, 1], |decoder| decoder.translate_program(&gradient_program()));
    let pixels_buffer = HostBuffer::new(state, size)?;
    let mut support = HostBuffer::new(state, 16)?;
    support.write(&BASE.to_le_bytes())?;
    let bindings = [(SUPPORT_BINDING, DescriptorType::UNIFORM_BUFFER), (GLOBAL_BINDING, DescriptorType::STORAGE_BUFFER)];
    let mut pipeline = ComputePipeline::new(state, &module.words(), &bindings)?;
    pipeline.bind(&[(SUPPORT_BINDING, &support), (GLOBAL_BINDING, &pixels_buffer)]);
    // Round up so 1080 rows get a partial last group; the program's guard drops the overhang.
    state.dispatch(&pipeline, [WIDTH.div_ceil(GROUP), HEIGHT.div_ceil(GROUP), 1])?;
    let mut pixels = vec![0; size as usize];
    pixels_buffer.read(&mut pixels)?;
    crate::display::present(WIDTH, HEIGHT, &pixels);
    Ok(())
}
