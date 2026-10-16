#![allow(unused_parens)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use std::collections::HashMap;
use crate::gpu::VkState;
use crate::gpu::spirv::{self, storage_class};

const MAX_REG_COUNT: usize = 254;
const MAX_UNIFORM_REG_COUNT: usize = 63;
pub const STATE_WORDS: usize = MAX_REG_COUNT + 1 + MAX_UNIFORM_REG_COUNT + 1;
pub const SUPPORT_BINDING: u32 = 0;
pub const CBUF_BINDING: u32 = 1;
pub const GLOBAL_BINDING: u32 = 9;
const RZ: u32 = 255;
const URZ: u32 = 63;
const SRZ: u32 = 255;
const PT: u32 = 7;
const CALL_ABS_R: u32 = 0x343;
const CALL_REL_R: u32 = 0x344;
const CALL_ABS: u32 = 0x943;
const CALL_REL: u32 = 0x944;
const BRA: u32 = 0x947;
const BRA_UPRED: u32 = 0x1547;
const BRA_UREG: u32 = 0x1947;
const BRX: u32 = 0x949;
const JMX: u32 = 0x94c;
const EXIT: u32 = 0x94d;
const RET: u32 = 0x950;
const KILL: u32 = 0x95b;
const VOTEU: u32 = 0x086;
const S2UR: u32 = 0x1c3;
const ROUNDING: [u32; 4] = [spirv::glsl::ROUND_EVEN, spirv::glsl::FLOOR, spirv::glsl::CEIL, spirv::glsl::TRUNC];
static MAX_CONST_BANK: usize = 17;
static ALLOW_F16_PARTIAL_WRITES: usize = 1;

#[repr(u32)]
pub enum TextureType {
    ONE_D = 0,
    TWO_D = 1,
    THREE_D = 2,
    CUBEMAP = 3,
    ONE_D_ARRAY = 4,
    TWO_D_ARRAY = 5,
    ONE_D_BUFFER = 6,
    TWO_D_NO_MIPMAP = 7,
    CUBE_ARRAY = 8,
}

#[repr(u32)]
pub enum TicSource {
    ZERO = 0,
    R = 2,
    G = 3,
    B = 4,
    A = 5,
    ONE_INT = 6,
    ONE_FLOAT = 7,
}

#[repr(u32)]
pub enum TicType {
    SNORM = 1,
    UNORM = 2,
    SINT = 3,
    UINT = 4,
    SNORM_FORCE_FP16 = 5,
    UNORM_FORCE_FP16 = 6,
    FLOAT = 7,
}

#[repr(u32)]
pub enum TicComponentSize {
    R32_G32_B32_A32 = 0x01,
    R32_G32_B32 = 0x02,
    R16_G16_B16_A16 = 0x03,
    R32_G32 = 0x04,
    R32_B24G8 = 0x05,
    X8B8G8R8 = 0x07,
    A8B8G8R8 = 0x08,
    A2B10G10R10 = 0x09,
    R16_G16 = 0x0c,
    G8R24 = 0x0d,
    G24R8 = 0x0e,
    R32 = 0x0f,
    A4B4G4R4 = 0x12,
    A5B5G5R1 = 0x13,
    A1B5G5R5 = 0x14,
    B5G6R5 = 0x15,
    B6G5R5 = 0x16,
    G8R8 = 0x18,
    R16 = 0x1b,
    Y8_VIDEO = 0x1c,
    R8 = 0x1d,
    G4R4 = 0x1e,
    R1 = 0x1f,
    E5B9G9R9_SHAREDEXP = 0x20,
    BF10GF11RF11 = 0x21,
    G8B8G8R8 = 0x22,
    B8G8R8G8 = 0x23,
    DXT1 = 0x24,
    DXT23 = 0x25,
    DXT45 = 0x26,
    DXN1 = 0x27,
    DXN2 = 0x28,
    BC6H_SF16 = 0x10,
    BC6H_UF16 = 0x11,
    BC7U = 0x17,
    Z24S8 = 0x29,
    X8Z24 = 0x2a,
    S8Z24 = 0x2b,
    X4V4Z24__COV4R4V = 0x2c,
    X4V4Z24__COV8R8V = 0x2d,
    V8Z24__COV4R12V = 0x2e,
    ZF32 = 0x2f,
    ZF32_X24S8 = 0x30,
    X8Z24_X20V4S8__COV4R4V = 0x31,
    X8Z24_X20V4S8__COV8R8V = 0x32,
    ZF32_X20V4X8__COV4R4V = 0x33,
    ZF32_X20V4X8__COV8R8V = 0x34,
    ZF32_X20V4S8__COV4R4V = 0x35,
    ZF32_X20V4S8__COV8R8V = 0x36,
    X8Z24_X16V8S8__COV4R12V = 0x37,
    ZF32_X16V8X8__COV4R12V = 0x38,
    ZF32_X16V8S8__COV4R12V = 0x39,
    Z16 = 0x3a,
    V8Z24__COV8R24V = 0x3b,
    X8Z24_X16V8S8__COV8R24V = 0x3c,
    ZF32_X16V8X8__COV8R24V = 0x3d,
    ZF32_X16V8S8__COV8R24V = 0x3e,
    CS_BITFIELD_SIZE = 0x3f
}

pub struct TicHeaderV1 {}

#[repr(u32)]
pub enum MaxAnisotropy {
    _1_TO_1 = 0x0,
    _2_TO_1 = 0x1,
    _4_TO_1 = 0x2,
    _6_TO_1 = 0x3,
    _8_TO_1 = 0x4,
    _10_TO_1 = 0x5,
    _12_TO_1 = 0x6,
    _16_TO_1 = 0x7,
}

#[repr(u32)]
pub enum SurfaceFormat {
    BITMAP = 0x001c,
    UNK1D = 0x001d,
    RGBA32_FLOAT = 0x00c0,
    RGBA32_SINT = 0x00c1,
    RGBA32_UINT = 0x00c2,
    RGBX32_FLOAT = 0x00c3,
    RGBX32_SINT = 0x00c4,
    RGBX32_UINT = 0x00c5,
    RGBA16_UNORM = 0x00c6,
    RGBA16_SNORM = 0x00c7,
    RGBA16_SINT = 0x00c8,
    RGBA16_UINT = 0x00c9,
    RGBA16_FLOAT = 0x00ca,
    RG32_FLOAT = 0x00cb,
    RG32_SINT = 0x00cc,
    RG32_UINT = 0x00cd,
    RGBX16_FLOAT = 0x00ce,
    BGRA8_UNORM = 0x00cf,
    BGRA8_SRGB = 0x00d0,
    RGB10_A2_UNORM = 0x00d1,
    RGB10_A2_UINT = 0x00d2,
    RGBA8_UNORM = 0x00d5,
    RGBA8_SRGB = 0x00d6,
    RGBA8_SNORM = 0x00d7,
    RGBA8_SINT = 0x00d8,
    RGBA8_UINT = 0x00d9,
    RG16_UNORM = 0x00da,
    RG16_SNORM = 0x00db,
    RG16_SINT = 0x00dc,
    RG16_UINT = 0x00dd,
    RG16_FLOAT = 0x00de,
    BGR10_A2_UNORM = 0x00df,
    R11G11B10_FLOAT = 0x00e0,
    R32_SINT = 0x00e3,
    R32_UINT = 0x00e4,
    R32_FLOAT = 0x00e5,
    BGRX8_UNORM = 0x00e6,
    BGRX8_SRGB = 0x00e7,
    B5G6R5_UNORM = 0x00e8,
    BGR5_A1_UNORM = 0x00e9,
    RG8_UNORM = 0x00ea,
    RG8_SNORM = 0x00eb,
    RG8_SINT = 0x00ec,
    RG8_UINT = 0x00ed,
    R16_UNORM = 0x00ee,
    R16_SNORM = 0x00ef,
    R16_SINT = 0x00f0,
    R16_UINT = 0x00f1,
    R16_FLOAT = 0x00f2,
    R8_UNORM = 0x00f3,
    R8_SNORM = 0x00f4,
    R8_SINT = 0x00f5,
    R8_UINT = 0x00f6,
    A8_UNORM = 0x00f7,
    BGR5_X1_UNORM = 0x00f8,
    RGBX8_UNORM = 0x00f9,
    RGBX8_SRGB = 0x00fa,
    BGR5_X1_UNORM_UNKFB = 0x00fb,
    BGR5_X1_UNORM_UNKFC = 0x00fc,
    BGRX8_UNORM_UNKFD = 0x00fd,
    BGRX8_UNORM_UNKFE = 0x00fe,
    Y32_UINT_UNKFF = 0x00ff,
}

pub enum ZetaFormat {
    Z32_FLOAT = 0x000a,
    Z16_UNORM = 0x0013,
    S8_Z24_UNORM = 0x0014,
    Z24_X8_UNORM = 0x0015,
    Z24_S8_UNORM = 0x0016,
    Z24_C8_UNORM = 0x0018,
    Z32_S8_X24_FLOAT = 0x0019,
    Z24_X8_S8_C8_X16_UNORM = 0x001d,
    Z32_X8_C8_X16_FLOAT = 0x001e,
    Z32_S8_C8_X16_FLOAT = 0x001f,
}

// SUTP
pub enum ImageFormat {
    RGBA32_FLOAT = 0x02,
    RGBA32_SINT = 0x03,
    RGBA32_UINT = 0x04,
    RGBA16_UNORM = 0x08,
    RGBA16_SNORM = 0x09,
    RGBA16_SINT = 0x0a,
    RGBA16_UINT = 0x0b,
    RGBA16_FLOAT = 0x0c,
    RG32_FLOAT = 0x0d,
    RG32_SINT = 0x0e,
    RG32_UINT = 0x0f,
    BGRA8_UNORM = 0x11,
    RGB10_A2_UNORM = 0x13,
    RGB10_A2_UINT = 0x15,
    RGBA8_UNORM = 0x18,
    RGBA8_SNORM = 0x1a,
    RGBA8_SINT = 0x1b,
    RGBA8_UINT = 0x1c,
    RG16_UNORM = 0x1d,
    RG16_SNORM = 0x1e,
    RG16_SINT = 0x1f,
    RG16_UINT = 0x20,
    RG16_FLOAT = 0x21,
    R11G11B10_FLOAT = 0x24,
    R32_SINT = 0x27,
    R32_UINT = 0x28,
    R32_FLOAT = 0x29,
    RG8_UNORM = 0x2e,
    RG8_SNORM = 0x2f,
    RG8_SINT = 0x30,
    RG8_UINT = 0x31,
    R16_UNORM = 0x32,
    R16_SNORM = 0x33,
    R16_SINT = 0x34,
    R16_UINT = 0x35,
    R16_FLOAT = 0x36,
    R8_UNORM = 0x37,
    R8_SNORM = 0x38,
    R8_SINT = 0x39,
    R8_UINT = 0x3a,
}

pub enum BitSize {
    B32 = 0b00,
    B64 = 0b01,
    B96 = 0b10,
    B128 = 0b11,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResourceKind {
    BuiltIn(u32),
    Cbuf(u32),
    Ssbo(u32),
    Support,
    Global,
}

#[derive(Debug, Clone, Copy)]
pub struct Resource {
    pub kind: ResourceKind,
    pub var: u32,
}

pub struct Decoder<'a> {
    pub ir: &'a mut spirv::Emitter,
    type_void: u32,
    // Pointers
    type_ptr_u32: [u32; 5],
    // Declared types for headers
    type_u32: [u32; 5],
    type_s32: [u32; 5],
    type_f32: [u32; 5],
    type_bool: [u32; 5],
    // abstract state
    type_abstract_state: u32,
    type_ptr_abstract_state_regs: u32,
    var_abstract_state: u32,
    var_pc: u32,
    pc: u32,
    const_u32_0: u32,
    const_u32_1: u32,
    type_ptr_ssbo: u32,
    type_ptr_ssbo_u32: u32,
    type_ptr_cbuf: u32,
    type_ptr_cbuf_u32: u32,
    type_u64: u32,
    type_s64: u32,
    type_f64: u32,
    glsl_ext: u32,
    pub resources: Vec<Resource>,
    // Cache for emitted u32 constants to avoid duplicate OpConstant
    const_cache: HashMap<u32, u32>,
}

fn opcode(inst: u128) -> u32 {
    ((inst >> 91 & 1) << 12 | inst & 0xfff) as u32
}

// In instructions: bits 81:34 hold a signed byte offset divided by four.
fn branch_offset(inst: u128) -> i64 {
    ((inst >> 34) as i64) << 16 >> 18
}

fn branch_target(pc: u32, inst: u128) -> Option<u32> {
    let offset = branch_offset(inst);
    match opcode(inst) {
        BRA | BRA_UREG | BRA_UPRED | CALL_REL => Some((pc as i64 + 1 + offset) as u32),
        CALL_ABS => Some(offset as u32),
        _ => None,
    }
}

fn nth_reg(reg: u32, zero: u32, i: u32) -> u32 {
    if reg == zero { reg } else { reg + i }
}

fn access_words(sz: u32) -> u32 {
    match sz {
        4..=6 => 1 << (sz - 4),
        _ => todo!("access size {sz}"),
    }
}

// Uniform datapath ops set bit 91 with forms 1 (URb) and 2, 4 (imm32); vector ops with bit 91 read a UR through forms 5 to 7.
fn is_uniform(inst: u128) -> bool {
    inst >> 91 & 1 != 0 && matches!(inst >> 9 & 7, 1 | 2 | 4)
}

impl<'a> Decoder<'a> {
    pub fn new(ir: &'a mut spirv::Emitter) -> Self {
        Self {
            ir,
            type_void: 0,
            type_ptr_u32: [0; 5],
            type_u32: [0; 5],
            type_s32: [0; 5],
            type_f32: [0; 5],
            type_bool: [0; 5],
            type_abstract_state: 0,
            type_ptr_abstract_state_regs: 0,
            var_abstract_state: 0,
            var_pc: 0,
            pc: 0,
            const_u32_0: 0,
            const_u32_1: 0,
            type_ptr_ssbo: 0,
            type_ptr_ssbo_u32: 0,
            type_ptr_cbuf: 0,
            type_ptr_cbuf_u32: 0,
            type_u64: 0,
            type_s64: 0,
            type_f64: 0,
            glsl_ext: 0,
            resources: Vec::new(),
            const_cache: HashMap::new(),
        }
    }

    pub fn get_type_void(&self) -> u32 {
        self.type_void
    }

    pub fn emit_entry_point(&mut self, func: u32, device: Option<&VkState>) {
        // SPIR-V 1.4+ entry points must list every global variable they use, not just Input/Output.
        let resources = self.resources.iter().map(|r| r.var);
        let interface: Vec<u32> = [self.var_abstract_state, self.var_pc].into_iter().chain(resources).collect();
        self.ir
            .emit_entry_point(spirv::execution_model::GLCOMPUTE, func, "main", &interface);
        self.ir
            .emit_execution_mode(func, spirv::execution_mode::LOCAL_SIZE, &[1, 1, 1]);
        use spirv::{capability as cap, execution_mode as mode};
        if device.is_some_and(|d| d.signed_zero_inf_nan_preserve) {
            self.ir.emit_capability(cap::SIGNED_ZERO_INF_NAN_PRESERVE);
            self.ir.emit_execution_mode(func, mode::SIGNED_ZERO_INF_NAN_PRESERVE, &[32]);
        }
        if device.is_some_and(|d| d.denorm_preserve) {
            self.ir.emit_capability(cap::DENORM_PRESERVE);
            self.ir.emit_execution_mode(func, mode::DENORM_PRESERVE, &[32]);
        }
    }

    pub fn init(&mut self) {
        self.type_void = self.ir.emit_type_void();
        self.type_u32[1] = self.ir.emit_type_int(32, 0);
        self.type_s32[1] = self.ir.emit_type_int(32, 1);
        self.type_f32[1] = self.ir.emit_type_float(32);
        self.type_bool[1] = self.ir.emit_type_bool();
        self.const_cache = HashMap::new();
        self.const_u32_0 = self.ir.emit_constant_typed(self.type_u32[1], 0u32);
        self.const_u32_1 = self.ir.emit_constant_typed(self.type_u32[1], 1u32);
        self.const_cache.insert(0, self.const_u32_0);
        self.const_cache.insert(1, self.const_u32_1);
        for i in 2..=4 {
            for type_sxx in [
                &mut self.type_u32, &mut self.type_s32, &mut self.type_f32, &mut self.type_bool
            ] {
                type_sxx[i] = self.ir.emit_type_vector(type_sxx[1], i as u32);
            }
        }

        {
            let length_const = self.ir.emit_constant_typed(self.type_u32[1], MAX_REG_COUNT as u32);
            let type_regs_array = self.ir.emit_type_array(self.type_u32[1], length_const);
            let ureg_count = self.cached_const_u32(MAX_UNIFORM_REG_COUNT as u32);
            let type_uregs_array = self.ir.emit_type_array(self.type_u32[1], ureg_count);
            let members = &[
                ("regs", type_regs_array),
                ("pred", self.type_u32[1]),
                ("uregs", type_uregs_array),
                ("upred", self.type_u32[1]),
            ];
            self.type_abstract_state = self.ir.emit_type_struct_typed(members);
            self.type_ptr_abstract_state_regs = self.ir.emit_type_pointer(storage_class::PRIVATE, type_regs_array);
        }

        // Define generic pointers
        for i in 1..=4 {
            self.type_ptr_u32[i] = self.ir.emit_type_pointer(storage_class::PRIVATE, self.type_u32[i]);
        }
        let type_ptr_abstract_state = self.ir.emit_type_pointer(storage_class::PRIVATE, self.type_abstract_state);
        self.var_abstract_state = self.ir.emit_variable(type_ptr_abstract_state, storage_class::PRIVATE);
        self.var_pc = self.ir.emit_variable(self.type_ptr_u32[1], storage_class::PRIVATE);


    }
    

    pub fn emit_register_io(&mut self, in_binding: u32, out_binding: u32) {
        let input = self.declare_ssbo(ResourceKind::Ssbo(0), in_binding);
        self.declare_ssbo(ResourceKind::Ssbo(1), out_binding);
        // Every register is seeded because predicated stores read the old value.
        for slot in 0..STATE_WORDS as u32 {
            let word = self.ssbo_word(input, slot);
            let value = self.ir.emit_load(self.type_u32[1], word);
            let ptr = self.state_slot(slot);
            self.ir.emit_store(ptr, value);
        }
    }

    pub fn emit_register_dump(&mut self) {
        let output = self.resource(ResourceKind::Ssbo(1)).unwrap();
        for slot in 0..STATE_WORDS as u32 {
            let ptr = self.state_slot(slot);
            let value = self.ir.emit_load(self.type_u32[1], ptr);
            let word = self.ssbo_word(output, slot);
            self.ir.emit_store(word, value);
        }
    }

    fn declare_ssbo(&mut self, kind: ResourceKind, binding: u32) -> u32 {
        use spirv::{decoration as deco, storage_class::STORAGE_BUFFER};
        if self.type_ptr_ssbo == 0 {
            let array = self.ir.emit_type_runtime_array(self.type_u32[1]);
            let block = self.ir.emit_type_struct(&[array]);
            self.type_ptr_ssbo = self.ir.emit_type_pointer(STORAGE_BUFFER, block);
            self.type_ptr_ssbo_u32 = self.ir.emit_type_pointer(STORAGE_BUFFER, self.type_u32[1]);
            self.ir.emit_decorate(array, deco::ARRAY_STRIDE, &[4]);
            self.ir.emit_decorate(block, deco::BLOCK, &[]);
            self.ir.emit_member_decorate(block, 0, deco::OFFSET, &[0]);
        }
        let var = self.ir.emit_variable(self.type_ptr_ssbo, STORAGE_BUFFER);
        self.ir.emit_decorate(var, deco::DESCRIPTOR_SET, &[0]);
        self.ir.emit_decorate(var, deco::BINDING, &[binding]);
        self.resources.push(Resource { kind, var });
        var
    }

    // Vulkan only takes structured control flow, so guest branches store a pc that one loop around a switch dispatches on.
    pub fn translate_program(&mut self, words: &[u128]) {
        let len = words.len() as u32;
        // BRX, JMX and CALL Ra only know their target at run time, so such a program dispatches on every pc; RET lands after a CALL.
        let indirect = words.iter().any(|inst| matches!(opcode(*inst), CALL_ABS_R | CALL_REL_R | BRX | JMX));
        let mut leaders: Vec<u32> = if indirect { (0..=len).collect() } else { vec![0, len] };
        for (pc, &inst) in (0..).zip(words) {
            leaders.extend(branch_target(pc, inst));
            if matches!(
                opcode(inst),
                CALL_ABS_R | CALL_REL_R | CALL_ABS | CALL_REL | BRA | BRA_UPRED | BRA_UREG | BRX | JMX | EXIT | RET | KILL
            ) {
                leaders.push(pc + 1);
            }
        }
        leaders.sort_unstable();
        leaders.dedup();
        assert_eq!(leaders.last(), Some(&len), "branch target outside the program");
        let [header, body, dispatched, cont, merge] = [(); 5].map(|_| self.ir.alloc_id());
        let blocks: Vec<(u32, u32)> = leaders[..leaders.len() - 1].iter().map(|&pc| (pc, self.ir.alloc_id())).collect();
        self.ir.emit_store(self.var_pc, self.const_u32_0);
        self.ir.emit_branch(header);
        self.ir.emit_label_id(header);
        self.ir.emit_loop_merge(merge, cont, 0);
        self.ir.emit_branch(body);
        self.ir.emit_label_id(body);
        let pc = self.ir.emit_load(self.type_u32[1], self.var_pc);
        self.ir.emit_selection_merge(dispatched, 0);
        self.ir.emit_switch(pc, dispatched, &blocks);
        for (&(start, label), &end) in blocks.iter().zip(&leaders[1..]) {
            self.ir.emit_label_id(label);
            let next = self.cached_const_u32(end);
            self.ir.emit_store(self.var_pc, next);
            for pc in start..end {
                self.pc = pc;
                self.translate_guarded(words[pc as usize]);
            }
            self.ir.emit_branch(dispatched);
        }
        self.ir.emit_label_id(dispatched);
        self.ir.emit_branch(cont);
        self.ir.emit_label_id(cont);
        let pc = self.ir.emit_load(self.type_u32[1], self.var_pc);
        let len = self.cached_const_u32(len);
        let done = self.ir.emit_ugreater_than_equal(self.type_bool[1], pc, len);
        self.ir.emit_branch_conditional(done, merge, header);
        self.ir.emit_label_id(merge);
    }

    fn translate_guarded(&mut self, inst: u128) {
        let (pg, pg_not) = ((inst >> 12 & 7) as u32, inst >> 15 & 1 != 0);
        if pg == PT {
            if !pg_not {
                self.translate(inst);
            }
            return;
        }
        // Uniform datapath opcodes, except VOTEU, take their guard from UP0..UP6 instead of P0..P6.
        let op = opcode(inst) & 0x1ff;
        let uniform = matches!(op, 0x80..=0xbf | S2UR) && op != VOTEU;
        let predicates = if uniform { self.load_upredicate() } else { self.load_predicate() };
        let bit = self.predicate_bit(predicates, pg);
        let cond = if pg_not {
            self.ir.emit_iequal(self.type_bool[1], bit, self.const_u32_0)
        } else {
            self.ir.emit_inot_equal(self.type_bool[1], bit, self.const_u32_0)
        };
        let [then, after] = [(); 2].map(|_| self.ir.alloc_id());
        self.ir.emit_selection_merge(after, 0);
        self.ir.emit_branch_conditional(cond, then, after);
        self.ir.emit_label_id(then);
        self.translate(inst);
        self.ir.emit_branch(after);
        self.ir.emit_label_id(after);
    }

    // Ra holds a byte address and the pc counts 16-byte instructions.
    fn jump_to_register(&mut self, inst: u128, relative: bool) {
        let ra = self.load_register((inst >> 24 & 0xff) as u32);
        let four = self.cached_const_u32(4);
        let ra = self.ir.emit_shift_right_logical(self.type_u32[1], ra, four);
        let offset = branch_offset(inst);
        let base = self.cached_const_u32((offset + if relative { self.pc as i64 + 1 } else { 0 }) as u32);
        let target = self.ir.emit_iadd(self.type_u32[1], base, ra);
        self.ir.emit_store(self.var_pc, target);
    }

    fn resource(&self, kind: ResourceKind) -> Option<u32> {
        self.resources.iter().find(|r| r.kind == kind).map(|r| r.var)
    }

    fn state_ptr(&mut self, path: &[u32]) -> u32 {
        let ids: Vec<u32> = path.iter().map(|&i| self.cached_const_u32(i)).collect();
        self.ir.emit_access_chain(self.type_ptr_u32[1], self.var_abstract_state, &ids)
    }

    fn state_slot(&mut self, slot: u32) -> u32 {
        let (regs, uregs) = (MAX_REG_COUNT as u32, MAX_UNIFORM_REG_COUNT as u32);
        match slot {
            s if s < regs => self.state_ptr(&[0, s]),
            s if s == regs => self.state_ptr(&[1]),
            s if s <= regs + uregs => self.state_ptr(&[2, s - regs - 1]),
            _ => self.state_ptr(&[3]),
        }
    }

    fn load_ureg(&mut self, ur: u32) -> u32 {
        if ur == URZ {
            return self.const_u32_0;
        }
        let ptr = self.state_ptr(&[2, ur]);
        self.ir.emit_load(self.type_u32[1], ptr)
    }

    fn store_ureg(&mut self, ur: u32, val: u32) {
        if ur != URZ {
            let ptr = self.state_ptr(&[2, ur]);
            self.ir.emit_store(ptr, val);
        }
    }

    fn load_upredicate(&mut self) -> u32 {
        let ptr = self.state_ptr(&[3]);
        self.ir.emit_load(self.type_u32[1], ptr)
    }

    fn store_upredicate(&mut self, val: u32) {
        let ptr = self.state_ptr(&[3]);
        self.ir.emit_store(ptr, val);
    }

    // Bits 11:9 say what bits 32..64 hold: Rb (1), imm32 (2, 4), c[bank][off] (3, 5), URb with bit 91 set (6, 7).
    // Forms 2, 3 and 7 feed that operand to C and move B to Rc at 64..72.
    fn alu_src(&mut self, inst: u128) -> u32 {
        let field = (inst >> 32) as u32;
        match ((inst >> 9) & 7, (inst >> 91) & 1) {
            (1, 0) => self.load_register(field & 0xff),
            (2 | 4, 0) => self.cached_const_u32(field),
            (3 | 5, 0) => {
                let offset = self.cached_const_u32((field >> 8 & 0x3fff) << 2);
                self.load_cbuf(ResourceKind::Cbuf(field >> 22 & 0x1f), offset)
            }
            (6 | 7, 1) => self.load_ureg(field & 0x3f),
            form => todo!("ALU source form {form:?}"),
        }
    }

    fn alu_reg(&mut self, inst: u128, bit: u32) -> u32 {
        let index = (inst >> bit) as u32;
        if is_uniform(inst) { self.load_ureg(index & 0x3f) } else { self.load_register(index & 0xff) }
    }

    fn store_alu(&mut self, inst: u128, val: u32) {
        let rd = (inst >> 16) as u32;
        if is_uniform(inst) { self.store_ureg(rd & 0x3f, val) } else { self.store_register(rd & 0xff, val) }
    }

    fn src_b(&mut self, inst: u128) -> u32 {
        match (is_uniform(inst), inst >> 9 & 7) {
            (true, 1) => self.alu_reg(inst, 32),
            (true, _) => self.cached_const_u32((inst >> 32) as u32),
            _ => self.alu_src(inst),
        }
    }

    // Bit 63 is the ~ on a register or cbuf B and part of the immediate otherwise.
    fn src_b_maybe_inverted(&mut self, inst: u128) -> u32 {
        let b = self.src_b(inst);
        if inst >> 63 & 1 != 0 && !matches!(inst >> 9 & 7, 2 | 4) { self.ir.emit_not(self.type_u32[1], b) } else { b }
    }

    fn srcs_bc(&mut self, inst: u128) -> (u32, u32) {
        let (src, rc) = (self.src_b(inst), self.alu_reg(inst, 64));
        if matches!(inst >> 9 & 7, 2 | 3 | 7) { (rc, src) } else { (src, rc) }
    }

    // Bit 75 picks .W (amount modulo bits) over the default clamp to bits.
    fn shift_amount(&mut self, inst: u128, amount: u32, bits: u32) -> u32 {
        if inst >> 75 & 1 != 0 {
            let mask = self.cached_const_u32(bits - 1);
            self.ir.emit_bitwise_and(self.type_u32[1], amount, mask)
        } else {
            let bits = self.cached_const_u32(bits);
            self.glsl(spirv::glsl::U_MIN, self.type_u32[1], &[amount, bits])
        }
    }

    fn alu_src_pair(&mut self, inst: u128) -> u32 {
        let field = (inst >> 32) as u32;
        let words = match ((inst >> 9) & 7, (inst >> 91) & 1) {
            (1, 0) => return self.load_pair(field & 0xff),
            (3 | 5, 0) => {
                let offset = (field >> 8 & 0x3fff) << 2;
                let bank = ResourceKind::Cbuf(field >> 22 & 0x1f);
                let lo_offset = self.cached_const_u32(offset);
                let hi_offset = self.cached_const_u32(offset + 4);
                [self.load_cbuf(bank, lo_offset), self.load_cbuf(bank, hi_offset)]
            }
            (6 | 7, 1) => {
                let ur = field & 0x3f;
                [self.load_ureg(ur), self.load_ureg((ur + 1).min(63))]
            }
            form => todo!("64-bit ALU source form {form:?}"),
        };
        let pair = self.ir.emit_composite_construct(self.type_u32[2], &words);
        let ty = self.type_u64();
        self.ir.emit_bitcast(ty, pair)
    }

    // Guest cbuf slot N is a std140 UBO at set 0, binding 1 + N, holding uvec4 data[4096]; the host support UBO is binding 0.
    fn load_cbuf(&mut self, kind: ResourceKind, offset: u32) -> u32 {
        let var = match self.resource(kind) {
            Some(var) => var,
            None => self.declare_cbuf(kind),
        };
        let [two, three, four] = [2, 3, 4].map(|i| self.cached_const_u32(i));
        let vec = self.ir.emit_shift_right_logical(self.type_u32[1], offset, four);
        let word = self.ir.emit_shift_right_logical(self.type_u32[1], offset, two);
        let elem = self.ir.emit_bitwise_and(self.type_u32[1], word, three);
        let ptr = self.ir.emit_access_chain(self.type_ptr_cbuf_u32, var, &[self.const_u32_0, vec, elem]);
        self.ir.emit_load(self.type_u32[1], ptr)
    }

    fn declare_cbuf(&mut self, kind: ResourceKind) -> u32 {
        use spirv::{decoration as deco, storage_class::UNIFORM};
        let binding = match kind {
            ResourceKind::Cbuf(slot) if slot < 8 => CBUF_BINDING + slot,
            ResourceKind::Support => SUPPORT_BINDING,
            _ => panic!("{kind:?} has no compute uniform binding"),
        };
        if self.type_ptr_cbuf == 0 {
            let len = self.cached_const_u32(4096);
            let array = self.ir.emit_type_array(self.type_u32[4], len);
            let block = self.ir.emit_type_struct(&[array]);
            // std140 pads every array element to 16 bytes, so uvec4 is the only dense u32 array a UBO can hold.
            self.ir.emit_decorate(array, deco::ARRAY_STRIDE, &[16]);
            self.ir.emit_decorate(block, deco::BLOCK, &[]);
            self.ir.emit_member_decorate(block, 0, deco::OFFSET, &[0]);
            self.type_ptr_cbuf = self.ir.emit_type_pointer(UNIFORM, block);
            self.type_ptr_cbuf_u32 = self.ir.emit_type_pointer(UNIFORM, self.type_u32[1]);
        }
        let var = self.ir.emit_variable(self.type_ptr_cbuf, UNIFORM);
        self.ir.emit_decorate(var, deco::DESCRIPTOR_SET, &[0]);
        self.ir.emit_decorate(var, deco::BINDING, &[binding]);
        self.resources.push(Resource { kind, var });
        var
    }

    // Global memory is a 4 GiB window above the base in support word 0, so the low address words suffice.
    fn global_word(&mut self, inst: u128, word: u32) -> u32 {
        assert!(inst >> 91 & 1 == 0 || inst >> 76 & 1 == 1, "uniform register in a global address");
        let var = match self.resource(ResourceKind::Global) {
            Some(var) => var,
            None => self.declare_ssbo(ResourceKind::Global, GLOBAL_BINDING),
        };
        let base = self.load_cbuf(ResourceKind::Support, self.const_u32_0);
        let offset = (((inst >> 40) as i32) << 8 >> 8) as u32;
        let offset = self.cached_const_u32(offset.wrapping_add(4 * word));
        let ra = self.load_register((inst >> 24) as u32 & 0xff);
        let addr = self.ir.emit_iadd(self.type_u32[1], ra, offset);
        let rel = self.ir.emit_isub(self.type_u32[1], addr, base);
        let two = self.cached_const_u32(2);
        let index = self.ir.emit_shift_right_logical(self.type_u32[1], rel, two);
        self.ir.emit_access_chain(self.type_ptr_ssbo_u32, var, &[self.const_u32_0, index])
    }

    // Int64 is declared on first use so 32-bit shaders still run on devices without shaderInt64.
    fn type_u64(&mut self) -> u32 {
        if self.type_u64 == 0 {
            self.ir.emit_capability(spirv::capability::INT64);
            self.type_u64 = self.ir.emit_type_int(64, 0);
        }
        self.type_u64
    }

    fn type_s64(&mut self) -> u32 {
        if self.type_s64 == 0 {
            self.ir.emit_capability(spirv::capability::INT64);
            self.type_s64 = self.ir.emit_type_int(64, 1);
        }
        self.type_s64
    }

    fn type_f64(&mut self) -> u32 {
        if self.type_f64 == 0 {
            self.ir.emit_capability(spirv::capability::FLOAT64);
            self.type_f64 = self.ir.emit_type_float(64);
        }
        self.type_f64
    }

    // OpBitcast puts vector component 0 in the low bits, so (Rn, Rn+1) is (lo, hi).
    fn load_pair(&mut self, reg: u32) -> u32 {
        let words = [self.load_register(reg), self.load_register(nth_reg(reg, RZ, 1))];
        let pair = self.ir.emit_composite_construct(self.type_u32[2], &words);
        let ty = self.type_u64();
        self.ir.emit_bitcast(ty, pair)
    }

    fn store_pair(&mut self, reg: u32, val: u32) {
        if reg == RZ {
            return;
        }
        let pair = self.ir.emit_bitcast(self.type_u32[2], val);
        for i in 0..2 {
            let word = self.ir.emit_composite_extract(self.type_u32[1], pair, &[i]);
            self.store_register(reg + i, word);
        }
    }

    fn special_register(&mut self, index: u32) -> u32 {
        use spirv::{BuiltIn, decoration, storage_class::INPUT};
        let builtin = |this: &mut Self, builtin: BuiltIn| {
            let ty = match builtin {
                BuiltIn::LocalInvocationIndex => this.type_u32[1],
                _ => this.type_u32[3],
            };
            let builtin = builtin as u32;
            let var = match this.resource(ResourceKind::BuiltIn(builtin)) {
                Some(var) => var,
                None => {
                    let ptr = this.ir.emit_type_pointer(INPUT, ty);
                    let var = this.ir.emit_variable(ptr, INPUT);
                    this.ir.emit_decorate(var, decoration::BUILTIN, &[builtin]);
                    this.resources.push(Resource { kind: ResourceKind::BuiltIn(builtin), var });
                    var
                }
            };
            this.ir.emit_load(ty, var)
        };
        // SR_VIRTID holds the warp id at bit 8, SR_TID packs x, y and z at bits 0, 16 and 26.
        match index {
            0 => {
                let invocation = builtin(self, BuiltIn::LocalInvocationIndex);
                let mask = self.cached_const_u32(31);
                self.ir.emit_bitwise_and(self.type_u32[1], invocation, mask)
            }
            3 => {
                let invocation = builtin(self, BuiltIn::LocalInvocationIndex);
                let [five, eight] = [5, 8].map(|i| self.cached_const_u32(i));
                let warp = self.ir.emit_shift_right_logical(self.type_u32[1], invocation, five);
                self.ir.emit_shift_left_logical(self.type_u32[1], warp, eight)
            }
            32 => {
                let tid = builtin(self, BuiltIn::LocalInvocationId);
                let x = self.ir.emit_composite_extract(self.type_u32[1], tid, &[0]);
                let y = self.ir.emit_composite_extract(self.type_u32[1], tid, &[1]);
                let z = self.ir.emit_composite_extract(self.type_u32[1], tid, &[2]);
                let [o16, c10, o26, c6] = [16, 10, 26, 6].map(|i| self.cached_const_u32(i));
                let xy = self.ir.emit_bit_field_insert(self.type_u32[1], x, y, o16, c10);
                self.ir.emit_bit_field_insert(self.type_u32[1], xy, z, o26, c6)
            }
            33..=35 => {
                let tid = builtin(self, BuiltIn::LocalInvocationId);
                self.ir.emit_composite_extract(self.type_u32[1], tid, &[index - 33])
            }
            37..=39 => {
                let ctaid = builtin(self, BuiltIn::WorkgroupId);
                self.ir.emit_composite_extract(self.type_u32[1], ctaid, &[index - 37])
            }
            1 | 2 | 4..=31 | 36 | 40..=133 | SRZ => self.const_u32_0,
            _ => panic!("unknown special register {index}"),
        }
    }

    fn bitcast_f32(&mut self, val: u32) -> u32 {
        self.ir.emit_bitcast(self.type_f32[1], val)
    }

    fn bitcast_u32(&mut self, val: u32) -> u32 {
        self.ir.emit_bitcast(self.type_u32[1], val)
    }

    fn glsl(&mut self, op: u32, ty: u32, args: &[u32]) -> u32 {
        if self.glsl_ext == 0 {
            self.glsl_ext = self.ir.emit_ext_inst_import("GLSL.std.450");
        }
        self.ir.emit_ext_inst(ty, self.glsl_ext, op, args)
    }

    fn fp_src(&mut self, ty: u32, val: u32, abs: bool, neg: bool) -> u32 {
        let val = if abs { self.glsl(spirv::glsl::F_ABS, ty, &[val]) } else { val };
        if neg { self.ir.emit_fnegate(ty, val) } else { val }
    }

    fn fp64_alu_src(&mut self, inst: u128) -> u32 {
        let ty = self.type_f64();
        let bits = self.alu_src_pair(inst);
        let val = self.ir.emit_bitcast(ty, bits);
        self.fp_src(ty, val, inst >> 62 & 1 != 0, inst >> 63 & 1 != 0)
    }

    fn int_src(&mut self, val: u32, neg: bool) -> u32 {
        if neg { self.neg(val) } else { val }
    }

    fn store_f2i(&mut self, rd: u32, fmt: u32, val: u32, rounded: u32, min: u32, max: u32) {
        let signed = fmt & 1 != 0;
        let below = self.ir.emit_ford_less_than(self.type_bool[1], rounded, min);
        let above = self.ir.emit_ford_greater_than_equal(self.type_bool[1], rounded, max);
        let nan = self.ir.emit_f_is_nan(self.type_bool[1], val);
        if fmt < 6 {
            let converted = if signed {
                let converted = self.ir.emit_convert_f_to_s(self.type_s32[1], rounded);
                self.ir.emit_bitcast(self.type_u32[1], converted)
            } else {
                self.ir.emit_convert_f_to_u(self.type_u32[1], rounded)
            };
            let lo = if signed { match fmt { 1 => (-128i32) as u32, 3 => (-32768i32) as u32, _ => i32::MIN as u32 } } else { 0 };
            let hi = match fmt { 0 => u8::MAX as u32, 1 => i8::MAX as u32, 2 => u16::MAX as u32, 3 => i16::MAX as u32, 4 => u32::MAX, _ => i32::MAX as u32 };
            let lo = self.cached_const_u32(lo);
            let hi = self.cached_const_u32(hi);
            let result = self.ir.emit_select(self.type_u32[1], below, lo, converted);
            let result = self.ir.emit_select(self.type_u32[1], above, hi, result);
            let result = self.ir.emit_select(self.type_u32[1], nan, self.const_u32_0, result);
            self.store_register(rd, result);
        } else {
            let u64_ty = self.type_u64();
            let converted = if signed {
                let s64_ty = self.type_s64();
                let converted = self.ir.emit_convert_f_to_s(s64_ty, rounded);
                self.ir.emit_bitcast(u64_ty, converted)
            } else {
                self.ir.emit_convert_f_to_u(u64_ty, rounded)
            };
            let lo = self.ir.emit_constant_typed(u64_ty, if signed { i64::MIN as u64 } else { 0 });
            let hi = self.ir.emit_constant_typed(u64_ty, if signed { i64::MAX as u64 } else { u64::MAX });
            let zero = self.ir.emit_constant_typed(u64_ty, 0u64);
            let result = self.ir.emit_select(u64_ty, below, lo, converted);
            let result = self.ir.emit_select(u64_ty, above, hi, result);
            let result = self.ir.emit_select(u64_ty, nan, zero, result);
            self.store_pair(rd, result);
        }
    }

    // FTZ keeps the sign: a subnormal becomes the zero of the same sign.
    fn flush_subnormal(&mut self, bits: u32) -> u32 {
        let [exponent, sign] = [0x7f80_0000, 0x8000_0000].map(|c| self.cached_const_u32(c));
        let exp = self.ir.emit_bitwise_and(self.type_u32[1], bits, exponent);
        let subnormal = self.ir.emit_iequal(self.type_bool[1], exp, self.const_u32_0);
        let zero = self.ir.emit_bitwise_and(self.type_u32[1], bits, sign);
        self.ir.emit_select(self.type_u32[1], subnormal, zero, bits)
    }

    fn fp_operand(&mut self, bits: u32, abs: bool, neg: bool, ftz: bool) -> u32 {
        let bits = if ftz { self.flush_subnormal(bits) } else { bits };
        let val = self.bitcast_f32(bits);
        self.fp_src(self.type_f32[1], val, abs, neg)
    }

    fn fp_reg(&mut self, inst: u128, reg_lsb: u32, abs_bit: u32, neg_bit: u32, ftz: bool) -> u32 {
        let bits = self.load_register((inst >> reg_lsb & 0xff) as u32);
        self.fp_operand(bits, inst >> abs_bit & 1 != 0, inst >> neg_bit & 1 != 0, ftz)
    }

    // An immediate fills all of bits 32..64, so it has no |x| (62) or -x (63) bit.
    fn fp_alu_src(&mut self, inst: u128, ftz: bool) -> u32 {
        let bits = self.alu_src(inst);
        let imm = matches!(inst >> 9 & 7, 2 | 4);
        self.fp_operand(bits, !imm && inst >> 62 & 1 != 0, !imm && inst >> 63 & 1 != 0, ftz)
    }

    // FMZ makes 0 * x a +0 even for x = inf or NaN, and implies FTZ.
    fn fmz_operands(&mut self, a: u32, b: u32) -> [u32; 2] {
        let zero = self.ir.emit_constant_typed(self.type_f32[1], 0.0f32);
        let [a_zero, b_zero] = [a, b].map(|x| self.ir.emit_ford_equal(self.type_bool[1], x, zero));
        let either = self.ir.emit_logical_or(self.type_bool[1], a_zero, b_zero);
        [a, b].map(|x| self.ir.emit_select(self.type_f32[1], either, zero, x))
    }

    // .SAT maps NaN to +0, which NClamp does and FClamp leaves undefined.
    fn fp_arith_result(&mut self, inst: u128, val: u32, ftz: bool) -> u32 {
        let rnd = (inst >> 78 & 3) as usize;
        assert!(rnd == 0, ".{} needs directed rounding, only RN is implemented", ["RN", "RM", "RP", "RZ"][rnd]);
        let val = if inst >> 77 & 1 != 0 {
            let [zero, one] = [0.0f32, 1.0].map(|c| self.ir.emit_constant_typed(self.type_f32[1], c));
            self.glsl(spirv::glsl::N_CLAMP, self.type_f32[1], &[val, zero, one])
        } else {
            val
        };
        let bits = self.bitcast_u32(val);
        if ftz { self.flush_subnormal(bits) } else { bits }
    }

    fn ssbo_word(&mut self, var: u32, index: u32) -> u32 {
        let index = self.cached_const_u32(index);
        self.ir
            .emit_access_chain(self.type_ptr_ssbo_u32, var, &[self.const_u32_0, index])
    }

    /// Return the SPIR-V id for an OpConstant(u32, value).
    /// Reuses a previously emitted constant if one exists.
    fn cached_const_u32(&mut self, value: u32) -> u32 {
        if let Some(&id) = self.const_cache.get(&value) {
            return id;
        }
        let id = self.ir.emit_constant_typed(self.type_u32[1], value);
        self.const_cache.insert(value, id);
        id
    }

    fn neg(&mut self, x: u32) -> u32 {
        self.ir.emit_isub(self.type_u32[1], self.const_u32_0, x)
    }

    fn load_register(&mut self, reg: u32) -> u32 {
        if reg == RZ {
            return self.const_u32_0;
        }
        assert!(reg < MAX_REG_COUNT as u32, "Register index out of bounds");
        let array_ptr = self.ir.emit_access_chain(self.type_ptr_abstract_state_regs, self.var_abstract_state, &[self.const_u32_0]);
        let idx = self.cached_const_u32(reg);
        let reg_ptr = self.ir.emit_access_chain(self.type_ptr_u32[1], array_ptr, &[idx]);
        self.ir.emit_load(self.type_u32[1], reg_ptr)
    }

    fn store_register(&mut self, reg: u32, val: u32) {
        if reg == RZ { return; }
        assert!(reg < MAX_REG_COUNT as u32, "Register index out of bounds");
        let array_ptr = self.ir.emit_access_chain(self.type_ptr_abstract_state_regs, self.var_abstract_state, &[self.const_u32_0]);
        let idx = self.cached_const_u32(reg);
        let reg_ptr = self.ir.emit_access_chain(self.type_ptr_u32[1], array_ptr, &[idx]);
        self.ir.emit_store(reg_ptr, val);
    }

    /// there are 8 predicates per invocation, the plan to modify them is as follows:
    /// %pred = load
    /// %mask = 0 - ((%pred >> N) & 1)    -> 0x00000000 or 0xFFFFFFFF
    /// <do some things>
    fn load_predicate(&mut self) -> u32 {
        let ptr = self.ir.emit_access_chain(self.type_ptr_u32[1], self.var_abstract_state, &[self.const_u32_1]);
        self.ir.emit_load(self.type_u32[1], ptr)
    }

    /// produces a full bitmask from a single predicate bit:
    ///   bit = (%pred >> N) & 1    -> 0 or 1
    ///   mask = 0 - bit            -> 0x00000000 or 0xFFFFFFFF
    /// used to avoid non-uniform control flow (branchless predication).
    fn load_predicate_mask(&mut self, pred: u32, invert: bool) -> u32 {
        if pred == PT {
            let all_ones = self.cached_const_u32(0xffff_ffff);
            if invert {
                self.ir.emit_not(self.type_u32[1], all_ones)
            } else {
                all_ones
            }
        } else {
            let base = self.load_predicate();
            let and_res = self.predicate_bit(base, pred);
            // broadcast to full mask: 0 - bit → 0x00000000 or 0xFFFFFFFF
            let res = self.ir.emit_isub(self.type_u32[1], self.const_u32_0, and_res);
            if invert {
                self.ir.emit_not(self.type_u32[1], res)
            } else {
                res
            }
        }
    }

    fn predicate_bit(&mut self, predicates: u32, pred: u32) -> u32 {
        let shift = self.cached_const_u32(pred);
        let shifted = self.ir.emit_shift_right_logical(self.type_u32[1], predicates, shift);
        self.ir.emit_bitwise_and(self.type_u32[1], shifted, self.const_u32_1)
    }

    fn store_predicate(&mut self, val: u32) {
        let ptr = self.ir.emit_access_chain(self.type_ptr_u32[1], self.var_abstract_state, &[self.const_u32_1]);
        self.ir.emit_store(ptr, val)
    }

    fn select_masked(&mut self, mask: u32, if_true: u32, if_false: u32) -> u32 {
        let not_mask = self.ir.emit_not(self.type_u32[1], mask);
        let v_true = self.ir.emit_bitwise_and(self.type_u32[1], if_true, mask);
        let v_false = self.ir.emit_bitwise_and(self.type_u32[1], if_false, not_mask);
        self.ir.emit_bitwise_or(self.type_u32[1], v_true, v_false)
    }

    fn store_register_predicated(&mut self, reg: u32, pred: u32, invert: bool, value: u32) {
        if reg == RZ { return; }
        let mask = self.load_predicate_mask(pred, invert);
        let v_orig = self.load_register(reg);
        let v_store = self.select_masked(mask, value, v_orig);
        self.store_register(reg, v_store);
    }

    fn emit_cmp_against_zero(&mut self, cop: u32, value: u32) -> u32 {
        // returns a SPIR-V u32 id containing 0 or 1
        // cop encoding: 0=F, 1=LT, 2=EQ, 3=LE, 4=GT, 5=NE, 6=GE, 7=T
        match cop {
            0 => self.const_u32_0,
            7 => self.const_u32_1,
            _ => {
                let shift31 = self.cached_const_u32(31);
                // sign bit: 1 if value is negative (signed), 0 otherwise
                let sign_bit = self.ir.emit_shift_right_logical(self.type_u32[1], value, shift31);

                match cop {
                    1 => sign_bit, // LT
                    6 => self.ir.emit_isub(self.type_u32[1], self.const_u32_1, sign_bit), // GE
                    _ => {
                        // need is_nonzero for EQ, LE, GT, NE
                        // is_nonzero = (value | (0 - value)) >>u 31
                        let neg_val = self.ir.emit_isub(self.type_u32[1], self.const_u32_0, value);
                        let val_or_neg = self.ir.emit_bitwise_or(self.type_u32[1], value, neg_val);
                        let is_nonzero = self.ir.emit_shift_right_logical(self.type_u32[1], val_or_neg, shift31);

                        match cop {
                            2 => self.ir.emit_isub(self.type_u32[1], self.const_u32_1, is_nonzero), // EQ
                            5 => is_nonzero, // NE
                            3 => { // LE = LT | EQ
                                let is_zero = self.ir.emit_isub(self.type_u32[1], self.const_u32_1, is_nonzero);
                                self.ir.emit_bitwise_or(self.type_u32[1], sign_bit, is_zero)
                            }
                            4 => { // GT = !LE
                                let is_zero = self.ir.emit_isub(self.type_u32[1], self.const_u32_1, is_nonzero);
                                let le = self.ir.emit_bitwise_or(self.type_u32[1], sign_bit, is_zero);
                                self.ir.emit_isub(self.type_u32[1], self.const_u32_1, le)
                            }
                            _ => panic!("emit_cmp_against_zero: unknown cop {}", cop),
                        }
                    }
                }
            }
        }
    }

    fn combine_and_set_predicate_bit(&mut self, pred_idx: u32, pp: u32, val: u32) {
        // val is a SPIR-V u32 id containing 0 or 1 (the comparison result)
        // pp: 0=SET, 1=AND, 2=OR, 3=XOR with existing predicate bit
        if pred_idx == PT {
            return;
        }

        let combined = if pp == 0 {
            val
        } else {
            // read existing predicate bit
            let base = self.load_predicate();
            let bit_pos = self.cached_const_u32(pred_idx);
            let shifted = self.ir.emit_shift_right_logical(self.type_u32[1], base, bit_pos);
            let old_bit = self.ir.emit_bitwise_and(self.type_u32[1], shifted, self.const_u32_1);
            match pp {
                1 => self.ir.emit_bitwise_and(self.type_u32[1], old_bit, val),
                2 => self.ir.emit_bitwise_or(self.type_u32[1], old_bit, val),
                3 => self.ir.emit_bitwise_xor(self.type_u32[1], old_bit, val),
                _ => panic!("combine_and_set_predicate_bit: unknown pp {}", pp),
            }
        };

        // write combined bit into predicate
        let base = self.load_predicate();
        let bit_pos = self.cached_const_u32(pred_idx);
        let mask = self.ir.emit_shift_left_logical(self.type_u32[1], self.const_u32_1, bit_pos);
        let not_mask = self.ir.emit_not(self.type_u32[1], mask);
        let cleared = self.ir.emit_bitwise_and(self.type_u32[1], base, not_mask);
        let shifted_val = self.ir.emit_shift_left_logical(self.type_u32[1], combined, bit_pos);
        let new_pred = self.ir.emit_bitwise_or(self.type_u32[1], cleared, shifted_val);
        self.store_predicate(new_pred);
    }

    pub fn finish(&mut self) {

    }

    // %rd := %ra + $ra_offset
    pub fn al2p(&mut self, inst: u128) {
        let pg = (((inst >> 12) & 0x7) << 0) as u32;
        let pg_not = (((inst >> 15) & 0x1) << 0) as u32;
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let ra = (((inst >> 24) & 0xff) << 0) as u32;
        let ra_offset = (((inst >> 40) & 0x7ff) << 0) as u32;
        let bop = (((inst >> 74) & 0x3) << 0) as u32;
        let _op = (((inst >> 79) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        assert!(bop == BitSize::B32 as u32);
        let base = self.load_register(ra);
        let offset = self.ir.emit_constant_typed(self.type_u32[1], ra_offset as u32);
        let dst_val = self.ir.emit_iadd(self.type_u32[1], base, offset);
        self.store_register_predicated(rd, pg, pg_not != 0, dst_val);
    }
    pub fn ald(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _op = (((inst >> 79) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn arrives(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _ra_urc = (((inst >> 64) & 0x3f) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ast(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn atom(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0xf) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _op = (((inst >> 87) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn atomg(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0xf) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _op = (((inst >> 87) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn atoms(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _sz = (((inst >> 73) & 0x3) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _op = (((inst >> 87) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn b2r(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _barname = (((inst >> 54) & 0xf) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn bar(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _sc = (((inst >> 42) & 0xfff) << 0);
        let _barname = (((inst >> 54) & 0xf) << 0);
        let _pq = (((inst >> 77) & 0x7) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn bitextract(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _num = (((inst >> 53) & 0x3) << 0);
        let _srchalf = (((inst >> 56) & 0x1) << 0);
        let _dstbyte = (((inst >> 57) & 0x3) << 0);
        let _datasize = (((inst >> 61) & 0x3) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _size = (((inst >> 75) & 0x3) << 0);
        let _mdidx = (((inst >> 78) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn bmma(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _size = (((inst >> 75) & 0x3) << 0);
        let _op = (((inst >> 77) & 0x3) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _op = (((inst >> 87) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn bmov(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _clear = (((inst >> 84) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn bmsk(&mut self, inst: u128) {
        let ty = self.type_u32[1];
        let pos = self.alu_reg(inst, 24);
        let pos = self.shift_amount(inst, pos, 32);
        let width = self.src_b(inst);
        let width = self.shift_amount(inst, width, 32);
        let [c32, ones] = [32, u32::MAX].map(|v| self.cached_const_u32(v));
        let room = self.ir.emit_isub(ty, c32, pos);
        let count = self.glsl(spirv::glsl::U_MIN, ty, &[width, room]);
        let res = self.ir.emit_bit_field_insert(ty, self.const_u32_0, ones, pos, count);
        self.store_alu(inst, res);
    }
    pub fn bpt(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _sb = (((inst >> 34) & 0x7) << 0);
        let cop = (((inst >> 84) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        panic!("BPT mode {cop} breaks into a debugger, which a compute shader has none of");
    }
    pub fn bra(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let cond = (((inst >> 32) & 0x3) << 0);
        let upq = (((inst >> 24) & 0x7) << 0) as u32;
        let upq_not = (((inst >> 27) & 0x1) << 0) as u32;
        let _simm = (((inst >> 34) & 0xffffffffffff) << 0);
        let _depth = (((inst >> 85) & 0x3) << 0);
        let pp = (((inst >> 87) & 0x7) << 0) as u32;
        let pp_not = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        assert!(pp == PT && pp_not == 0, "BRA Pp not implemented");
        // .DIV and .CONV test the warp for divergence, and one invocation per register file is always converged.
        if cond == 2 {
            return;
        }
        let mut target = self.cached_const_u32(branch_target(self.pc, inst).unwrap());
        if opcode(inst) == BRA_UPRED {
            let bit = if upq == PT {
                self.const_u32_1
            } else {
                let upred = self.load_upredicate();
                self.predicate_bit(upred, upq)
            };
            let invert = self.cached_const_u32(upq_not);
            let taken = self.ir.emit_inot_equal(self.type_bool[1], bit, invert);
            let next = self.ir.emit_load(self.type_u32[1], self.var_pc);
            target = self.ir.emit_select(self.type_u32[1], taken, target, next);
        }
        self.ir.emit_store(self.var_pc, target);
    }
    pub fn break_(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _barreg = (((inst >> 16) & 0xf) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn brev(&mut self, inst: u128) {
        let b = self.src_b(inst);
        let res = self.ir.emit_bit_reverse(self.type_u32[1], b);
        self.store_alu(inst, res);
    }
    pub fn brx(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _simm = (((inst >> 34) & 0xffffffffffff) << 0);
        let _depth = (((inst >> 85) & 0x3) << 0);
        let pp = (((inst >> 87) & 0x7) << 0) as u32;
        let pp_not = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        assert!(pp == PT && pp_not == 0, "BRX Pp not implemented");
        self.jump_to_register(inst, true);
    }
    pub fn brxu(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _cond = (((inst >> 32) & 0x3) << 0);
        let _simm = (((inst >> 34) & 0xffffffffffff) << 0);
        let _depth = (((inst >> 85) & 0x3) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn bssy(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _barreg = (((inst >> 16) & 0xf) << 0);
        let _sa = (((inst >> 34) & 0x3fffffff) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        // BSSY and BSYNC reconverge a diverged warp, and one invocation per register file never diverges.
    }
    pub fn bsync(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _barreg = (((inst >> 16) & 0xf) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
    }
    pub fn call(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _depth = (((inst >> 86) & 0x1) << 0);
        let pp = (((inst >> 87) & 0x7) << 0) as u32;
        let pp_not = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        assert!(pp == PT && pp_not == 0, "CALL Pp not implemented");
        match opcode(inst) {
            CALL_ABS_R | CALL_REL_R => self.jump_to_register(inst, opcode(inst) == CALL_REL_R),
            _ => {
                let target = branch_target(self.pc, inst).expect("CALL through a constant or uniform register not implemented");
                let target = self.cached_const_u32(target);
                self.ir.emit_store(self.var_pc, target);
            }
        }
    }
    pub fn cctl(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _mode = (((inst >> 78) & 0x7) << 0);
        let _op = (((inst >> 87) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn cctll(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _op = (((inst >> 87) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn cctlt(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _depth = (((inst >> 85) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn clmad(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn cs2r(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let sr = (((inst >> 72) & 0xff) << 0) as u32;
        let wide = (((inst >> 80) & 0x1) << 0) as u32;
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        for i in 0..=wide {
            let val = self.special_register(nth_reg(sr, SRZ, i));
            self.store_register(nth_reg(rd, RZ, i), val);
        }
    }
    pub fn csmtest(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _sa = (((inst >> 32) & 0xfffffff) << 0);
        let _vtgmode = (((inst >> 62) & 0x3) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _dstfmt = (((inst >> 76) & 0x7) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn dadd(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn depbar(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _cnt = (((inst >> 38) & 0x3f) << 0);
        let _sbidx = (((inst >> 44) & 0x7) << 0);
        let _le = (((inst >> 47) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn dfma(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn dmma(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _op = (((inst >> 87) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn dmul(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn dsetp(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _test = (((inst >> 76) & 0xf) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn errbar(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn exit(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _mode = (((inst >> 84) & 0x3) << 0);
        let _depth = (((inst >> 86) & 0x1) << 0);
        let pp = (((inst >> 87) & 0x7) << 0) as u32;
        let pp_not = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        assert!(pp == PT && pp_not == 0, "EXIT Pp not implemented");
        let exit = self.cached_const_u32(u32::MAX);
        self.ir.emit_store(self.var_pc, exit);
    }
    pub fn f2f(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let rd = (((inst >> 16) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let rnd = ((inst >> 78) & 3) as u32;
        let ftz = ((inst >> 80) & 1) != 0;
        let fmt = (((inst >> 84) & 0x7) << 3) | (((inst >> 75) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        let f64_ty = self.type_f64();
        let val = match fmt {
            19 => {
                let val = self.fp_alu_src(inst, ftz);
                self.ir.emit_f_convert(f64_ty, val)
            }
            26 => {
                let val = self.fp64_alu_src(inst);
                let nearest = self.ir.emit_f_convert(self.type_f32[1], val);
                self.ir.emit_decorate(nearest, spirv::decoration::NO_CONTRACTION, &[]);
                let mut bits = self.bitcast_u32(nearest);
                if rnd != 0 {
                    let back = self.ir.emit_f_convert(f64_ty, nearest);
                    self.ir.emit_decorate(back, spirv::decoration::NO_CONTRACTION, &[]);
                    let less = self.ir.emit_ford_less_than(self.type_bool[1], back, val);
                    let greater = self.ir.emit_ford_greater_than(self.type_bool[1], back, val);
                    let sign = self.cached_const_u32(0x8000_0000);
                    let sign = self.ir.emit_bitwise_and(self.type_u32[1], bits, sign);
                    let negative = self.ir.emit_inot_equal(self.type_bool[1], sign, self.const_u32_0);
                    let plus = self.ir.emit_iadd(self.type_u32[1], bits, self.const_u32_1);
                    let minus = self.ir.emit_isub(self.type_u32[1], bits, self.const_u32_1);
                    let up = self.ir.emit_select(self.type_u32[1], negative, minus, plus);
                    let down = self.ir.emit_select(self.type_u32[1], negative, plus, minus);
                    let (use_up, use_down) = match rnd {
                        1 => (None, Some(greater)),
                        2 => (Some(less), None),
                        _ => {
                            let positive = self.ir.emit_logical_not(self.type_bool[1], negative);
                            let use_up = self.ir.emit_logical_and(self.type_bool[1], negative, less);
                            let use_down = self.ir.emit_logical_and(self.type_bool[1], positive, greater);
                            (Some(use_up), Some(use_down))
                        }
                    };
                    if let Some(cond) = use_up {
                        bits = self.ir.emit_select(self.type_u32[1], cond, up, bits);
                    }
                    if let Some(cond) = use_down {
                        bits = self.ir.emit_select(self.type_u32[1], cond, down, bits);
                    }
                }
                let bits = if ftz { self.flush_subnormal(bits) } else { bits };
                self.store_register(rd as u32, bits);
                return;
            }
            _ => todo!("F2F format {fmt} requires f16 support"),
        };
        let u64_ty = self.type_u64();
        let bits = self.ir.emit_bitcast(u64_ty, val);
        self.store_pair(rd as u32, bits);
    }
    pub fn f2fp(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _merge = (((inst >> 78) & 0x1) << 0);
        let _rndmode = (((inst >> 79) & 0x7) << 0);
        let _dstfmt = (((inst >> 86) & 0x1) << 1) | (((inst >> 76) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!("F2FP requires f16 support");
    }
    pub fn f2i(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _extract = (((inst >> 60) & 0x3) << 0);
        let dstfmt = ((((inst >> 75) & 0x3) << 1) | (((inst >> 72) & 0x1) << 0)) as u32;
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let rnd = ((inst >> 78) & 3) as usize;
        let ftz = ((inst >> 80) & 1) != 0;
        let srcfmt = ((inst >> 84) & 3) as u32;
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        let width = 8 << (dstfmt >> 1);
        let (min, max) = match dstfmt & 1 {
            0 => (0.0, 2f64.powi(width)),
            _ => (-2f64.powi(width - 1), 2f64.powi(width - 1)),
        };
        let (val, [min, max]) = match srcfmt {
            2 => (self.fp_alu_src(inst, ftz), [min, max].map(|c| self.ir.emit_constant_typed(self.type_f32[1], c as f32))),
            3 => {
                let ty = self.type_f64();
                (self.fp64_alu_src(inst), [min, max].map(|c| self.ir.emit_constant_typed(ty, c)))
            }
            _ => todo!("F2I source format {srcfmt} is not implemented"),
        };
        let ty = if srcfmt == 2 { self.type_f32[1] } else { self.type_f64() };
        let rounded = self.glsl(ROUNDING[rnd], ty, &[val]);
        self.store_f2i(rd, dstfmt, val, rounded, min, max);
    }
    pub fn f2ip(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!("F2IP requires f16 support");
    }
    pub fn fadd(&mut self, inst: u128) {
        let ftz = inst >> 80 & 1 != 0;
        let a = self.fp_reg(inst, 24, 73, 72, ftz);
        let b = self.fp_alu_src(inst, ftz);
        let sum = self.ir.emit_fadd(self.type_f32[1], a, b);
        let bits = self.fp_arith_result(inst, sum, ftz);
        self.store_register((inst >> 16 & 0xff) as u32, bits);
    }
    pub fn fchk(&mut self, _inst: u128) {
        panic!("FCHK is not implemented: NAK, Ryujinx and Eden have no lowering for it, so it waits for silicon fixtures");
    }
    pub fn ffma(&mut self, inst: u128) {
        let fmz = inst >> 76 & 1 != 0;
        let ftz = fmz || inst >> 80 & 1 != 0;
        let a = self.fp_reg(inst, 24, 73, 72, ftz);
        let src = self.fp_alu_src(inst, ftz);
        let rc = self.fp_reg(inst, 64, 74, 75, ftz);
        let [b, c] = if matches!(inst >> 9 & 7, 2 | 3 | 7) { [rc, src] } else { [src, rc] };
        let [a, b] = if fmz { self.fmz_operands(a, b) } else { [a, b] };
        let val = self.glsl(spirv::glsl::FMA, self.type_f32[1], &[a, b, c]);
        let bits = self.fp_arith_result(inst, val, ftz);
        self.store_register((inst >> 16 & 0xff) as u32, bits);
    }
    pub fn flo(&mut self, inst: u128) {
        assert!(inst >> 81 & 7 == 7, "FLO predicate output not implemented");
        let ty = self.type_u32[1];
        let b = self.src_b_maybe_inverted(inst);
        let msb = self.glsl(if inst >> 73 & 1 != 0 { spirv::glsl::FIND_S_MSB } else { spirv::glsl::FIND_U_MSB }, ty, &[b]);
        let res = if inst >> 74 & 1 != 0 {
            let none = self.cached_const_u32(u32::MAX);
            let found = self.ir.emit_inot_equal(self.type_bool[1], msb, none);
            let c31 = self.cached_const_u32(31);
            let amount = self.ir.emit_isub(ty, c31, msb);
            self.ir.emit_select(ty, found, amount, msb)
        } else {
            msb
        };
        self.store_alu(inst, res);
    }
    pub fn fmnmx(&mut self, inst: u128) {
        assert!(inst >> 81 & 3 == 0, "FMNMX.NAN and .XORSIGN are not implemented");
        let ftz = inst >> 80 & 1 != 0;
        let a = self.fp_reg(inst, 24, 73, 72, ftz);
        let b = self.fp_alu_src(inst, ftz);
        let mask = self.load_predicate_mask((inst >> 87 & 7) as u32, inst >> 90 & 1 != 0);
        let take_min = self.ir.emit_inot_equal(self.type_bool[1], mask, self.const_u32_0);
        // NMin and NMax return the other operand when one is NaN, like the hardware.
        let [min, max] = [spirv::glsl::N_MIN, spirv::glsl::N_MAX].map(|op| self.glsl(op, self.type_f32[1], &[a, b]));
        let val = self.ir.emit_select(self.type_f32[1], take_min, min, max);
        let bits = self.bitcast_u32(val);
        self.store_register((inst >> 16 & 0xff) as u32, bits);
    }
    pub fn fmul(&mut self, inst: u128) {
        let fmz = inst >> 76 & 1 != 0;
        let ftz = fmz || inst >> 80 & 1 != 0;
        let a = self.fp_reg(inst, 24, 73, 72, ftz);
        let b = self.fp_alu_src(inst, ftz);
        let [a, b] = if fmz { self.fmz_operands(a, b) } else { [a, b] };
        let mut product = self.ir.emit_fmul(self.type_f32[1], a, b);
        let scale = (inst >> 84 & 7) as i32 - 4;
        if scale != 0 {
            let factor = self.ir.emit_constant_typed(self.type_f32[1], 2f32.powi(scale));
            product = self.ir.emit_fmul(self.type_f32[1], product, factor);
        }
        let bits = self.fp_arith_result(inst, product, ftz);
        self.store_register((inst >> 16 & 0xff) as u32, bits);
    }
    pub fn footprint(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _b = (((inst >> 59) & 0x1) << 0);
        let _scr = (((inst >> 60) & 0x1) << 0);
        let _destidx = (((inst >> 61) & 0x7) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _merge = (((inst >> 78) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn frnd(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _extract = (((inst >> 60) & 0x3) << 0);
        let rnd = ((inst >> 78) & 3) as usize;
        let ftz = ((inst >> 80) & 1) != 0;
        let fmt = (((inst >> 84) & 0x3) << 2) | (((inst >> 75) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        match fmt {
            10 => {
                let val = self.fp_alu_src(inst, ftz);
                let val = self.glsl(ROUNDING[rnd], self.type_f32[1], &[val]);
                let val = self.bitcast_u32(val);
                self.store_register(rd, val);
            }
            15 => {
                let val = self.fp64_alu_src(inst);
                let ty = self.type_f64();
                let val = self.glsl(ROUNDING[rnd], ty, &[val]);
                let u64_ty = self.type_u64();
                let val = self.ir.emit_bitcast(u64_ty, val);
                self.store_pair(rd, val);
            }
            _ => todo!("FRND format {fmt} requires f16 support"),
        }
    }
    pub fn fsel(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn fset(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _test = (((inst >> 76) & 0xf) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn fsetp(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _test = (((inst >> 76) & 0xf) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn fswzadd(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn gather(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _num = (((inst >> 53) & 0x3) << 0);
        let _srchalf = (((inst >> 56) & 0x1) << 0);
        let _dstbyte = (((inst >> 57) & 0x3) << 0);
        let _datasize = (((inst >> 61) & 0x3) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _size = (((inst >> 75) & 0x3) << 0);
        let _mdidx = (((inst >> 78) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn genmetadata(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _num = (((inst >> 53) & 0x3) << 0);
        let _seq = (((inst >> 55) & 0x1) << 0);
        let _fmt = (((inst >> 60) & 0x7) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _num = (((inst >> 72) & 0x3) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _size = (((inst >> 75) & 0x3) << 0);
        let _mdidx = (((inst >> 78) & 0xf) << 0);
        let _vecidx6 = (((inst >> 84) & 0x3f) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn getlmembase(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn hadd2(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _extract = (((inst >> 60) & 0x3) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _merge = (((inst >> 78) & 0x1) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn hfma2(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _satrelu = (((inst >> 79) & 0x1) << 1) | (((inst >> 77) & 0x1) << 0);
        let _fmz = (((inst >> 80) & 0x1) << 1) | (((inst >> 76) & 0x1) << 0);
        let _mode = (((inst >> 81) & 0x3) << 0);
        let _clear = (((inst >> 83) & 0x1) << 0);
        let _clear = (((inst >> 84) & 0x1) << 0);
        let _ofmt = (((inst >> 85) & 0x1) << 1) | (((inst >> 78) & 0x1) << 0);
        let _iswzb = (((inst >> 86) & 0x1) << 2) | (((inst >> 60) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn hfma2_mma(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _satrelu = (((inst >> 79) & 0x1) << 1) | (((inst >> 77) & 0x1) << 0);
        let _fmz = (((inst >> 80) & 0x1) << 1) | (((inst >> 76) & 0x1) << 0);
        let _clear = (((inst >> 83) & 0x1) << 0);
        let _clear = (((inst >> 84) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn hmma(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _re = (((inst >> 40) & 0xff) << 0);
        let _id = (((inst >> 48) & 0x3) << 0);
        let _re_reuse_src_e = (((inst >> 50) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _size = (((inst >> 78) & 0x1) << 1) | (((inst >> 75) & 0x1) << 0);
        let _nan = (((inst >> 81) & 0x1) << 0);
        let _srcfmt = (((inst >> 82) & 0x3) << 0);
        let _op = (((inst >> 87) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn hmnmx2(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _extract = (((inst >> 60) & 0x3) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _nan = (((inst >> 81) & 0x1) << 0);
        let _rndsrc = (((inst >> 82) & 0x1) << 0);
        let _ofmt = (((inst >> 85) & 0x1) << 1) | (((inst >> 78) & 0x1) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn hmul2(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _extract = (((inst >> 60) & 0x3) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _merge = (((inst >> 78) & 0x1) << 0);
        let _fmz = (((inst >> 80) & 0x1) << 1) | (((inst >> 76) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn hset2(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _extract = (((inst >> 60) & 0x3) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _bop = (((inst >> 69) & 0x3) << 0);
        let _memdesc = (((inst >> 71) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _test = (((inst >> 76) & 0xf) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn hsetp2(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _extract = (((inst >> 60) & 0x3) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _bop = (((inst >> 69) & 0x3) << 0);
        let _memdesc = (((inst >> 71) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _test = (((inst >> 76) & 0xf) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn i2f(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let extract = ((inst >> 60) & 3) as u32;
        let size = ((inst >> 75) & 3) as u32;
        let rnd = ((inst >> 78) & 3) as u32;
        let srcfmt = ((((inst >> 84) & 0x3) << 1) | (((inst >> 74) & 0x1) << 0)) as u32;
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        let signed = srcfmt & 1 != 0;
        let mut val = if srcfmt >= 6 {
            self.alu_src_pair(inst)
        } else {
            let val = self.alu_src(inst);
            if srcfmt < 4 {
                let offset = self.cached_const_u32(extract * 8);
                let width = self.cached_const_u32(if srcfmt < 2 { 8 } else { 16 });
                if signed {
                    self.ir.emit_bit_field_s_extract(self.type_u32[1], val, offset, width)
                } else {
                    self.ir.emit_bit_field_u_extract(self.type_u32[1], val, offset, width)
                }
            } else { val }
        };
        if signed {
            let ty = if srcfmt >= 6 {
                self.type_s64()
            } else { self.type_s32[1] };
            val = self.ir.emit_bitcast(ty, val);
        }
        let result = match size {
            2 => if signed {
                self.ir.emit_convert_s_to_f(self.type_f32[1], val)
            } else {
                self.ir.emit_convert_u_to_f(self.type_f32[1], val)
            },
            3 => {
                let ty = self.type_f64();
                if signed { self.ir.emit_convert_s_to_f(ty, val) } else { self.ir.emit_convert_u_to_f(ty, val) }
            }
            _ => todo!("I2F destination format {size} requires f16 support"),
        };
        assert!(rnd == 0, "directed I2F rounding is not implemented");
        if size == 3 {
            let u64_ty = self.type_u64();
            let result = self.ir.emit_bitcast(u64_ty, result);
            self.store_pair(rd, result);
        } else {
            let result = self.bitcast_u32(result);
            self.store_register(rd, result);
        }
    }
    pub fn i2fp(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sz = (((inst >> 75) & 0x7) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _srcfmt = (((inst >> 84) & 0x3) << 1) | (((inst >> 74) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!("I2FP requires f16 support");
    }
    pub fn i2i(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let dstfmt = ((inst >> 76) & 3) as u32;
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        let val = self.alu_src(inst);
        let (lo, hi) = match dstfmt {
            0 => (0, u8::MAX as u32),
            1 => (i8::MIN as i32 as u32, i8::MAX as u32),
            2 => (0, u16::MAX as u32),
            3 => (i16::MIN as i32 as u32, i16::MAX as u32),
            _ => unreachable!(),
        };
        let zero = self.const_u32_0;
        let lo_const = self.cached_const_u32(lo);
        let hi_const = self.cached_const_u32(hi);
        let negative = self.ir.emit_sless_than(self.type_bool[1], val, zero);
        let below = self.ir.emit_sless_than(self.type_bool[1], val, lo_const);
        let above = self.ir.emit_sgreater_than(self.type_bool[1], val, hi_const);
        let low = if dstfmt & 1 != 0 { below } else { negative };
        let result = self.ir.emit_select(self.type_u32[1], low, lo_const, val);
        let result = self.ir.emit_select(self.type_u32[1], above, hi_const, result);
        self.store_register(rd, result);
    }
    pub fn i2ip(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _num = (((inst >> 72) & 0x3) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _dstfmt = (((inst >> 76) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!("I2IP requires f16 support");
    }
    pub fn iabs(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn iadd3(&mut self, inst: u128) {
        let pg = (((inst >> 12) & 0x7) << 0) as u32;
        let pg_not = (((inst >> 15) & 0x1) << 0) as u32;
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let ra = (((inst >> 24) & 0xff) << 0) as u32;
        let rc = (((inst >> 64) & 0xff) << 0) as u32;
        let negate_a = (((inst >> 72) & 0x1) << 0);
        let x = (((inst >> 74) & 0x1) << 0);
        let negate_c = (((inst >> 75) & 0x1) << 0);
        let _pq = (((inst >> 77) & 0x7) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let pu = (((inst >> 81) & 0x7) << 0) as u32;
        let cop = (((inst >> 84) & 0x7) << 0) as u32;
        let pp = (((inst >> 87) & 0x7) << 0) as u32;
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0) as u32;
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);

        // guards for unimplemented features
        assert!(x == 0, "IADD3.X not implemented");
        // ftz is a no-op for integer operations

        let mut a = self.load_register(ra);
        if negate_a != 0 {
            a = self.neg(a);
        }

        // Determine source encoding from opcode bits 9-11:
        //   4 (100) = immediate (ra_offset, bits 32-63)
        //   1 (001) = register  (rb, bits 32-39)
        //   5 (101) = constant bank (sc_addr + sc_bank)
        //   6 (110) = uniform register bank
        let src_type = ((inst >> 9) & 0x7) as u32;
        let b = match src_type {
            4 => {
                let ra_offset = (((inst >> 32) & 0xffffffff) << 0) as u32;
                self.cached_const_u32(ra_offset)
            }
            1 => {
                let rb = (((inst >> 32) & 0xff) << 0) as u32;
                let negate_b = (((inst >> 63) & 0x1) << 0);
                assert!(negate_a == 0 || negate_b == 0, "IADD3: -Ra with -Rb");
                let b = self.load_register(rb);
                if negate_b != 0 {
                    self.neg(b)
                } else {
                    b
                }
            }
            5 => {
                todo!("IADD3: constant bank source not yet implemented")
            }
            6 => {
                todo!("IADD3: uniform register bank source not yet implemented")
            }
            _ => panic!("IADD3: unknown source encoding type {}", src_type),
        };

        let mut c = self.load_register(rc);
        if negate_c != 0 {
            c = self.neg(c);
        }

        let ab = self.ir.emit_iadd(self.type_u32[1], a, b);
        let res = self.ir.emit_iadd(self.type_u32[1], ab, c);

        // handle comparison op
        if cop != 0 {
            let cmp_result = self.emit_cmp_against_zero(cop, res);
            self.combine_and_set_predicate_bit(pu, pp, cmp_result);
        }

        self.store_register_predicated(rd, pg, pg_not != 0, res);
    }
    pub fn ide(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _sb = (((inst >> 32) & 0xffff) << 0);
        let _clear = (((inst >> 84) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn idp(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn imad(&mut self, inst: u128) {
        let rd = ((inst >> 16) & 0xff) as u32;
        let ra = ((inst >> 24) & 0xff) as u32;
        let rc = ((inst >> 64) & 0xff) as u32;
        let signed = inst >> 73 & 1 != 0;
        let x = inst >> 74 & 1 != 0;
        let pp = ((inst >> 87) & 7) as u32;
        let pp_not = inst >> 90 & 1 != 0;
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        let form = (inst >> 9) & 7;
        let swapped = matches!(form, 2 | 3 | 7);
        let a = self.load_register(ra);
        let b = if swapped { self.load_register(rc) } else { self.alu_src(inst) };
        let c32 = if swapped { self.alu_src(inst) } else { self.load_register(rc) };
        let modify_c = match form {
            2 => false,
            3 | 7 => inst >> 63 & 1 != 0,
            _ => inst >> 75 & 1 != 0,
        };
        let c32 = if x && modify_c {
            self.ir.emit_not(self.type_u32[1], c32)
        } else {
            self.int_src(c32, modify_c)
        };
        let carry = if x {
            let mask = self.load_predicate_mask(pp, pp_not);
            self.ir.emit_bitwise_and(self.type_u32[1], mask, self.const_u32_1)
        } else {
            self.const_u32_0
        };
        let mode = inst as u32 & 7;
        if mode == 5 {
            let ty = self.type_u64();
            let a = if signed { self.ir.emit_s_convert(ty, a) } else { self.ir.emit_u_convert(ty, a) };
            let b = if signed { self.ir.emit_s_convert(ty, b) } else { self.ir.emit_u_convert(ty, b) };
            let c = if swapped {
                match form {
                    2 => self.ir.emit_s_convert(ty, c32),
                    3 => {
                        let bank = (inst >> 54 & 0x1f) as u32;
                        let offset = ((inst >> 40 & 0x3fff) << 2) as u32;
                        let offsets = [offset, offset + 4].map(|v| self.cached_const_u32(v));
                        let words = offsets.map(|v| self.load_cbuf(ResourceKind::Cbuf(bank), v));
                        let pair = self.ir.emit_composite_construct(self.type_u32[2], &words);
                        self.ir.emit_bitcast(ty, pair)
                    }
                    7 => {
                        let ur = (inst >> 32 & 0x3f) as u32;
                        let words = [self.load_ureg(ur), self.load_ureg((ur + 1).min(63))];
                        let pair = self.ir.emit_composite_construct(self.type_u32[2], &words);
                        self.ir.emit_bitcast(ty, pair)
                    }
                    _ => unreachable!(),
                }
            } else {
                self.load_pair(rc)
            };
            let c = if x && modify_c {
                self.ir.emit_not(ty, c)
            } else if modify_c {
                let zero = self.ir.emit_constant_typed(ty, 0u64);
                self.ir.emit_isub(ty, zero, c)
            } else {
                c
            };
            let carry = self.ir.emit_u_convert(ty, carry);
            let product = self.ir.emit_imul(ty, a, b);
            let result = self.ir.emit_iadd(ty, product, c);
            let result = self.ir.emit_iadd(ty, result, carry);
            self.store_pair(rd, result);
        } else if mode == 7 {
            let ty = self.type_u64();
            let a = if signed { self.ir.emit_s_convert(ty, a) } else { self.ir.emit_u_convert(ty, a) };
            let b = if signed { self.ir.emit_s_convert(ty, b) } else { self.ir.emit_u_convert(ty, b) };
            let product = self.ir.emit_imul(ty, a, b);
            let shift = self.ir.emit_constant_typed(ty, 32u64);
            let hi = self.ir.emit_shift_right_logical(ty, product, shift);
            let hi = self.ir.emit_u_convert(self.type_u32[1], hi);
            let result = self.ir.emit_iadd(self.type_u32[1], hi, c32);
            let result = self.ir.emit_iadd(self.type_u32[1], result, carry);
            self.store_register(rd, result);
        } else {
            let product = self.ir.emit_imul(self.type_u32[1], a, b);
            let result = self.ir.emit_iadd(self.type_u32[1], product, c32);
            let result = self.ir.emit_iadd(self.type_u32[1], result, carry);
            self.store_register(rd, result);
        }
    }
    pub fn imma(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _re_reuse_src_e = (((inst >> 50) & 0x1) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _nan = (((inst >> 81) & 0x1) << 0);
        let _rndsrc = (((inst >> 82) & 0x1) << 0);
        let _srcfmta = (((inst >> 83) & 0x1) << 2) | (((inst >> 76) & 0x3) << 0);
        let _srcfmtb = (((inst >> 84) & 0x1) << 2) | (((inst >> 78) & 0x3) << 0);
        let _size = (((inst >> 85) & 0x3) << 1) | (((inst >> 75) & 0x1) << 0);
        let _op = (((inst >> 87) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn imnmx(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ipa(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn isberd(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffff) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _merge = (((inst >> 78) & 0x1) << 0);
        let _op = (((inst >> 79) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn isbewr(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffff) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _merge = (((inst >> 78) & 0x1) << 0);
        let _op = (((inst >> 79) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn isetp(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let ra = (((inst >> 24) & 0xff) << 0) as u32;
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let ex = inst >> 72 & 1 != 0;
        let signed = inst >> 73 & 1 != 0;
        let bop = ((inst >> 74) & 3) as u32;
        let cmp = ((inst >> 76) & 7) as u32;
        let pu = ((inst >> 81) & 7) as u32;
        let pv = ((inst >> 84) & 7) as u32;
        let pp = ((inst >> 87) & 7) as u32;
        let pp_not = inst >> 90 & 1 != 0;
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        let (a, b) = (self.load_register(ra), self.alu_src(inst));
        let mut result = match (cmp, signed) {
            (0, _) => self.const_u32_0,
            (7, _) => self.const_u32_1,
            (2, _) => {
                let v = self.ir.emit_iequal(self.type_bool[1], a, b);
                self.ir.emit_select(self.type_u32[1], v, self.const_u32_1, self.const_u32_0)
            }
            (5, _) => {
                let v = self.ir.emit_inot_equal(self.type_bool[1], a, b);
                self.ir.emit_select(self.type_u32[1], v, self.const_u32_1, self.const_u32_0)
            }
            (1, false) => { let v = self.ir.emit_uless_than(self.type_bool[1], a, b); self.ir.emit_select(self.type_u32[1], v, self.const_u32_1, self.const_u32_0) }
            (1, true) => { let v = self.ir.emit_sless_than(self.type_bool[1], a, b); self.ir.emit_select(self.type_u32[1], v, self.const_u32_1, self.const_u32_0) }
            (3, false) => { let v = self.ir.emit_uless_than_equal(self.type_bool[1], a, b); self.ir.emit_select(self.type_u32[1], v, self.const_u32_1, self.const_u32_0) }
            (3, true) => { let v = self.ir.emit_sless_than_equal(self.type_bool[1], a, b); self.ir.emit_select(self.type_u32[1], v, self.const_u32_1, self.const_u32_0) }
            (4, false) => { let v = self.ir.emit_ugreater_than(self.type_bool[1], a, b); self.ir.emit_select(self.type_u32[1], v, self.const_u32_1, self.const_u32_0) }
            (4, true) => { let v = self.ir.emit_sgreater_than(self.type_bool[1], a, b); self.ir.emit_select(self.type_u32[1], v, self.const_u32_1, self.const_u32_0) }
            (6, false) => { let v = self.ir.emit_ugreater_than_equal(self.type_bool[1], a, b); self.ir.emit_select(self.type_u32[1], v, self.const_u32_1, self.const_u32_0) }
            (6, true) => { let v = self.ir.emit_sgreater_than_equal(self.type_bool[1], a, b); self.ir.emit_select(self.type_u32[1], v, self.const_u32_1, self.const_u32_0) }
            _ => unreachable!(),
        };
        if ex && !matches!(cmp, 0 | 7) {
            let pr = ((inst >> 68) & 7) as u32;
            let pr_not = inst >> 71 & 1 != 0;
            let pr = self.load_predicate_mask(pr, pr_not);
            let pr = self.ir.emit_bitwise_and(self.type_u32[1], pr, self.const_u32_1);
            let equal = self.ir.emit_iequal(self.type_bool[1], a, b);
            let equal = self.ir.emit_select(self.type_u32[1], equal, self.const_u32_1, self.const_u32_0);
            result = match cmp {
                2 => self.ir.emit_bitwise_and(self.type_u32[1], equal, pr),
                5 => self.ir.emit_bitwise_or(self.type_u32[1], result, pr),
                _ => {
                    let strict = match (cmp, signed) {
                        (1 | 3, false) => self.ir.emit_uless_than(self.type_bool[1], a, b),
                        (1 | 3, true) => self.ir.emit_sless_than(self.type_bool[1], a, b),
                        (4 | 6, false) => self.ir.emit_ugreater_than(self.type_bool[1], a, b),
                        (4 | 6, true) => self.ir.emit_sgreater_than(self.type_bool[1], a, b),
                        _ => unreachable!(),
                    };
                    let strict = self.ir.emit_select(self.type_u32[1], strict, self.const_u32_1, self.const_u32_0);
                    let tied = self.ir.emit_bitwise_and(self.type_u32[1], equal, pr);
                    self.ir.emit_bitwise_or(self.type_u32[1], strict, tied)
                }
            };
        }
        let pp_mask = self.load_predicate_mask(pp, pp_not);
        let p = self.ir.emit_bitwise_and(self.type_u32[1], pp_mask, self.const_u32_1);
        let not_result = self.ir.emit_isub(self.type_u32[1], self.const_u32_1, result);
        let combine = |this: &mut Self, value| match bop {
            0 => this.ir.emit_bitwise_and(this.type_u32[1], value, p),
            1 => this.ir.emit_bitwise_or(this.type_u32[1], value, p),
            2 => this.ir.emit_bitwise_xor(this.type_u32[1], value, p),
            _ => unreachable!(),
        };
        let u = combine(self, result);
        let v = combine(self, not_result);
        self.combine_and_set_predicate_bit(pu, 0, u);
        self.combine_and_set_predicate_bit(pv, 0, v);
    }
    pub fn jmp(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _cond = (((inst >> 32) & 0x3) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _depth = (((inst >> 85) & 0x3) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn jmx(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _simm = (((inst >> 34) & 0xffffffffffff) << 0);
        let _depth = (((inst >> 85) & 0x3) << 0);
        let pp = (((inst >> 87) & 0x7) << 0) as u32;
        let pp_not = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        assert!(pp == PT && pp_not == 0, "JMX Pp not implemented");
        self.jump_to_register(inst, false);
    }
    pub fn jmxu(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _cond = (((inst >> 32) & 0x3) << 0);
        let _simm = (((inst >> 34) & 0xffffffffffff) << 0);
        let _depth = (((inst >> 85) & 0x3) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn kill(&mut self, inst: u128) {
        // KILL is pixel-only on hardware, so a compute invocation that reaches one just ends like at EXIT.
        self.exit(inst);
    }
    pub fn ld(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _pnz = (((inst >> 64) & 0xf) << 0);
        let _sp2 = (((inst >> 68) & 0x3) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ldc(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let ra = (((inst >> 24) & 0xff) << 0) as u32;
        let ra_offset = (((inst >> 38) & 0xffff) << 0) as u16 as i16 as u32;
        let sc_bank = (((inst >> 54) & 0x1f) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let stride = (((inst >> 78) & 0x3) << 0) as u32;
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        assert!(inst >> 91 & 1 == 0 && stride == 0, "LDC with a uniform bank or a segmented address");
        let base = self.load_register(ra);
        for i in 0..access_words(sz) {
            let offset = self.cached_const_u32(ra_offset.wrapping_add(4 * i));
            let addr = self.ir.emit_iadd(self.type_u32[1], base, offset);
            let value = self.load_cbuf(ResourceKind::Cbuf(sc_bank), addr);
            self.store_register(nth_reg(rd, RZ, i), value);
        }
    }
    pub fn ldg(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _pnz = (((inst >> 64) & 0xf) << 0);
        let _sp2 = (((inst >> 68) & 0x3) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        for i in 0..access_words(sz) {
            let ptr = self.global_word(inst, i);
            let value = self.ir.emit_load(self.type_u32[1], ptr);
            self.store_register(nth_reg(rd, RZ, i), value);
        }
    }
    pub fn ldgdepbar(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ldgsts(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 32) & 0xfff) << 0);
        let _rb_offset = (((inst >> 44) & 0xfffff) << 0);
        let _ra_urc = (((inst >> 64) & 0x3f) << 0);
        let _input_reg_sz_32_dist = (((inst >> 70) & 0x1) << 0);
        let _sp2 = (((inst >> 71) & 0x3) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _nan = (((inst >> 81) & 0x1) << 0);
        let _rndsrc = (((inst >> 82) & 0x1) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ldl(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn lds(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ldsm(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _num = (((inst >> 72) & 0x3) << 0);
        let _sz = (((inst >> 75) & 0x7) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ldtram(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn lea(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let ra = (((inst >> 24) & 0xff) << 0) as u32;
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let rc = (((inst >> 64) & 0xff) << 0) as u32;
        let negate = inst >> 72 & 1 != 0;
        let sx32 = inst >> 73 & 1 != 0;
        let x = inst >> 74 & 1 != 0;
        let scale = (((inst >> 75) & 0x1f) << 0) as u32;
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        assert!(rc == 255 && !sx32 && !x, "LEA.HI/.X/.SX32 not implemented");
        let shift = self.cached_const_u32(scale);
        let a = self.load_register(ra);
        let a = self.ir.emit_shift_left_logical(self.type_u32[1], a, shift);
        let a = self.int_src(a, negate);
        let b = self.alu_src(inst);
        let result = self.ir.emit_iadd(self.type_u32[1], a, b);
        self.store_register(rd, result);
    }
    pub fn lepc(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn lop3(&mut self, inst: u128) {
        let ty = self.type_u32[1];
        let a = self.alu_reg(inst, 24);
        let (b, c) = self.srcs_bc(inst);
        let lut = (inst >> 72) as u8;
        let srcs = [a, b, c];
        let nots = srcs.map(|x| self.ir.emit_not(ty, x));
        let mut res = self.const_u32_0;
        for minterm in (0..8).filter(|m| lut >> m & 1 != 0) {
            let [x, y, z] = [0, 1, 2].map(|i| if minterm >> (2 - i) & 1 != 0 { srcs[i] } else { nots[i] });
            let xy = self.ir.emit_bitwise_and(ty, x, y);
            let xyz = self.ir.emit_bitwise_and(ty, xy, z);
            res = self.ir.emit_bitwise_or(ty, res, xyz);
        }
        self.store_alu(inst, res);
        let (pu, pp) = ((inst >> 81 & 7) as u32, (inst >> 87 & 7) as u32);
        if pu == 7 {
            return;
        }
        let uniform = is_uniform(inst);
        let preds = if uniform { self.load_upredicate() } else { self.load_predicate() };
        let pp_bit = if pp == 7 {
            self.const_u32_1
        } else {
            let offset = self.cached_const_u32(pp);
            self.ir.emit_bit_field_u_extract(ty, preds, offset, self.const_u32_1)
        };
        let pp_bit = if inst >> 90 & 1 != 0 { self.ir.emit_bitwise_xor(ty, pp_bit, self.const_u32_1) } else { pp_bit };
        let nonzero = self.emit_cmp_against_zero(5, res);
        let bit = if inst >> 80 & 1 != 0 {
            self.ir.emit_bitwise_and(ty, nonzero, pp_bit)
        } else {
            self.ir.emit_bitwise_or(ty, nonzero, pp_bit)
        };
        let offset = self.cached_const_u32(pu);
        let preds = self.ir.emit_bit_field_insert(ty, preds, bit, offset, self.const_u32_1);
        if uniform { self.store_upredicate(preds) } else { self.store_predicate(preds) }
    }
    pub fn match_(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _op = (((inst >> 79) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn membar(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _dstfmt = (((inst >> 76) & 0x7) << 0);
        let _sem = (((inst >> 79) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn mov(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        const MOV_RR: u32 = 0x202;
        const MOV_RI: u32 = 0x802;
        const MOV_RC: u32 = 0xa02;
        const MOV_RU: u32 = 0x1c02;

        // MOV shares one primary opcode across several addressing-mode
        // variants (register, constant-bank, indexed constant-bank, immediate)
        // that real hardware disambiguates using bits inside the operand
        // field itself, not the opcode. Re-derive the same 13-bit selector
        // the dispatcher used to route here; every form but cx[UR] is the
        // ALU source selector.
        let variant_key = ((((inst >> 91) & 0x1) << 12) | (inst & 0xfff)) as u32;
        if matches!(variant_key, MOV_RR | MOV_RI | MOV_RC | MOV_RU) {
            let val = self.alu_src(inst);
            self.store_register(rd, val);
            return;
        }

        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _pixmasku04 = (((inst >> 72) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn movm(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sz = (((inst >> 75) & 0x7) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn mufu(&mut self, inst: u128) {
        use spirv::glsl::{COS, EXP2, INVERSE_SQRT, LOG2, SIN, SQRT, TANH};
        let ty = self.type_f32[1];
        let x = self.fp_alu_src(inst, false);
        let val = match inst >> 74 & 0xf {
            // Since SM70 SIN and COS take the angle in turns, nvcc scales by 1/(2*pi) first.
            op @ (0 | 1) => {
                let tau = self.ir.emit_constant_typed(ty, std::f32::consts::TAU);
                let radians = self.ir.emit_fmul(ty, x, tau);
                self.glsl(if op == 0 { COS } else { SIN }, ty, &[radians])
            }
            2 => self.glsl(EXP2, ty, &[x]),
            3 => self.glsl(LOG2, ty, &[x]),
            4 => {
                let one = self.ir.emit_constant_typed(ty, 1.0f32);
                self.ir.emit_fdiv(ty, one, x)
            }
            5 => self.glsl(INVERSE_SQRT, ty, &[x]),
            8 => self.glsl(SQRT, ty, &[x]),
            9 => self.glsl(TANH, ty, &[x]),
            op => panic!("MUFU op {op} (RCP64H, RSQ64H) is not implemented"),
        };
        let bits = self.bitcast_u32(val);
        self.store_register((inst >> 16 & 0xff) as u32, bits);
    }
    pub fn nanosleep(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _clear = (((inst >> 83) & 0x1) << 0);
        let _warp = (((inst >> 85) & 0x1) << 0);
        let _depth = (((inst >> 86) & 0x1) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn nanotrap(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _depth = (((inst >> 86) & 0x1) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn nop(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        self.ir.emit_nop();
    }
    pub fn out(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn p2r(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn pixld(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _mode = (((inst >> 78) & 0x7) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn plop3(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _up = (((inst >> 67) & 0x1) << 0);
        let _pr = (((inst >> 68) & 0x7) << 0);
        let _memdesc = (((inst >> 71) & 0x1) << 0);
        let _lop = (((inst >> 72) & 0x1f) << 11)
            | (((inst >> 64) & 0x7) << 8)
            | (((inst >> 16) & 0xff) << 0);
        let _pq = (((inst >> 77) & 0x7) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn pmtrig(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _sb = (((inst >> 32) & 0xffff) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn popc(&mut self, inst: u128) {
        let b = self.src_b_maybe_inverted(inst);
        let res = self.ir.emit_bit_count(self.type_u32[1], b);
        self.store_alu(inst, res);
    }
    pub fn prmt(&mut self, inst: u128) {
        let ty = self.type_u32[1];
        let a = self.alu_reg(inst, 24);
        let (sel, b) = self.srcs_bc(inst);
        let u64_ty = self.type_u64();
        let pair = self.ir.emit_composite_construct(self.type_u32[2], &[a, b]);
        let bytes = self.ir.emit_bitcast(u64_ty, pair);
        let mode = inst >> 72 & 7;
        let [c1, c3, c7, c8] = [1, 3, 7, 8].map(|v| self.cached_const_u32(v));
        let n = self.ir.emit_bitwise_and(ty, sel, c3);
        let mut res = self.const_u32_0;
        for i in 0..4 {
            let ci = self.cached_const_u32(i);
            let index = match mode {
                0 => {
                    let offset = self.cached_const_u32(4 * i);
                    self.ir.emit_bit_field_u_extract(ty, sel, offset, c3)
                }
                1 | 2 => {
                    let k = self.cached_const_u32(if mode == 1 { i } else { 8 - i });
                    let sum = self.ir.emit_iadd(ty, n, k);
                    self.ir.emit_bitwise_and(ty, sum, c7)
                }
                3 => n,
                4 => self.glsl(spirv::glsl::U_MAX, ty, &[n, ci]),
                5 => self.glsl(spirv::glsl::U_MIN, ty, &[n, ci]),
                6 => {
                    let lane = self.cached_const_u32(i & 1);
                    self.ir.emit_bit_field_insert(ty, lane, sel, c1, c1)
                }
                _ => panic!("invalid PRMT mode 7"),
            };
            let shift = self.ir.emit_shift_left_logical(ty, index, c3);
            let wide = self.ir.emit_shift_right_logical(u64_ty, bytes, shift);
            let mut byte = self.ir.emit_u_convert(ty, wide);
            if mode == 0 {
                let offset = self.cached_const_u32(4 * i + 3);
                let flag = self.ir.emit_bit_field_u_extract(ty, sel, offset, c1);
                let replicate = self.ir.emit_inot_equal(self.type_bool[1], flag, self.const_u32_0);
                let sign = self.ir.emit_bit_field_s_extract(ty, byte, c7, c1);
                byte = self.ir.emit_select(ty, replicate, sign, byte);
            }
            let offset = self.cached_const_u32(8 * i);
            res = self.ir.emit_bit_field_insert(ty, res, byte, offset, c8);
        }
        self.store_alu(inst, res);
    }
    pub fn qspc(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x3) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn r2b(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _barname = (((inst >> 54) & 0xf) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn r2p(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn r2ur(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _clear = (((inst >> 84) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn red(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _memdesc = (((inst >> 71) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0xf) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn redux(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _mode = (((inst >> 78) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ret(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _simm = (((inst >> 34) & 0xffffffffffff) << 0);
        let _warp = (((inst >> 85) & 0x1) << 0);
        let _depth = (((inst >> 86) & 0x1) << 0);
        let pp = (((inst >> 87) & 0x7) << 0) as u32;
        let pp_not = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        assert!(pp == PT && pp_not == 0, "RET Pp not implemented");
        assert!(inst >> 91 & 1 == 0, "RET through a uniform register not implemented");
        self.jump_to_register(inst, inst >> 85 & 1 == 0);
    }
    pub fn rpcmov(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn rtt(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn s2r(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let sr = (((inst >> 72) & 0xff) << 0) as u32;
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        let val = self.special_register(sr);
        self.store_register(rd, val);
    }
    pub fn s2ur(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let urd = (((inst >> 16) & 0x3f) << 0) as u32;
        let sr = (((inst >> 72) & 0xff) << 0) as u32;
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        let val = self.special_register(sr);
        self.store_ureg(urd, val);
    }
    pub fn scatter(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0xf) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _mode = (((inst >> 81) & 0x3) << 0);
        let _vecidx = (((inst >> 83) & 0x7f) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn sel(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn setctaid(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn setlmembase(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn sgxt(&mut self, inst: u128) {
        let ty = self.type_u32[1];
        let a = self.alu_reg(inst, 24);
        let width = self.src_b(inst);
        let width = self.shift_amount(inst, width, 32);
        let res = if inst >> 73 & 1 != 0 {
            self.ir.emit_bit_field_s_extract(ty, a, self.const_u32_0, width)
        } else {
            self.ir.emit_bit_field_u_extract(ty, a, self.const_u32_0, width)
        };
        self.store_alu(inst, res);
    }
    pub fn shf(&mut self, inst: u128) {
        let ty = self.type_u32[1];
        let low = self.alu_reg(inst, 24);
        let (shift, high) = self.srcs_bc(inst);
        let (fmt, right) = (inst >> 73 & 3, inst >> 76 & 1 != 0);
        let shift = self.shift_amount(inst, shift, if fmt < 2 { 64 } else { 32 });
        let u64_ty = self.type_u64();
        let pair = self.ir.emit_composite_construct(self.type_u32[2], &[low, high]);
        let mut x = self.ir.emit_bitcast(u64_ty, pair);
        // A clamped 64 is a full-width shift, which SPIR-V leaves undefined, so it goes in two halves.
        let half = self.ir.emit_shift_right_logical(ty, shift, self.const_u32_1);
        let rest = self.ir.emit_isub(ty, shift, half);
        for s in [half, rest] {
            x = match (right, fmt % 2 == 0) {
                (false, _) => self.ir.emit_shift_left_logical(u64_ty, x, s),
                (true, true) => self.ir.emit_shift_right_arithmetic(u64_ty, x, s),
                (true, false) => self.ir.emit_shift_right_logical(u64_ty, x, s),
            };
        }
        let words = self.ir.emit_bitcast(self.type_u32[2], x);
        let res = self.ir.emit_composite_extract(ty, words, &[(inst >> 80 & 1) as u32]);
        self.store_alu(inst, res);
    }
    pub fn shfl(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc = (((inst >> 40) & 0x1fff) << 0);
        let _sb = (((inst >> 53) & 0x1f) << 0);
        let _shflmd = (((inst >> 58) & 0x3) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn spmetadata(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _destidx = (((inst >> 61) & 0x7) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0xf) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _mode = (((inst >> 81) & 0x3) << 0);
        let _vecidx = (((inst >> 83) & 0x7f) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn st(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn stg(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let rb = (((inst >> 32) & 0xff) << 0) as u32;
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        for i in 0..access_words(sz) {
            let ptr = self.global_word(inst, i);
            let value = self.load_register(nth_reg(rb, RZ, i));
            self.ir.emit_store(ptr, value);
        }
    }
    pub fn stl(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn sts(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn suatom(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _clamp = (((inst >> 59) & 0x3) << 0);
        let _destidx = (((inst >> 61) & 0x7) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn suld(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _clamp = (((inst >> 59) & 0x3) << 0);
        let _destidx = (((inst >> 61) & 0x7) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn suquery(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _destidx = (((inst >> 61) & 0x7) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cas = (((inst >> 87) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn sured(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _clamp = (((inst >> 59) & 0x3) << 0);
        let _destidx = (((inst >> 61) & 0x7) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn sust(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _clamp = (((inst >> 59) & 0x3) << 0);
        let _destidx = (((inst >> 61) & 0x7) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn tex(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _b = (((inst >> 59) & 0x1) << 0);
        let _scr = (((inst >> 60) & 0x1) << 0);
        let _destidx = (((inst >> 61) & 0x7) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _pixmasku04 = (((inst >> 72) & 0xf) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _merge = (((inst >> 78) & 0x1) << 0);
        let _sem = (((inst >> 79) & 0x3) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn tld(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _b = (((inst >> 59) & 0x1) << 0);
        let _scr = (((inst >> 60) & 0x1) << 0);
        let _destidx = (((inst >> 61) & 0x7) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _pixmasku04 = (((inst >> 72) & 0xf) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _merge = (((inst >> 78) & 0x1) << 0);
        let _sem = (((inst >> 79) & 0x3) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn tld4(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _b = (((inst >> 59) & 0x1) << 0);
        let _scr = (((inst >> 60) & 0x1) << 0);
        let _destidx = (((inst >> 61) & 0x7) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _pixmasku04 = (((inst >> 72) & 0xf) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _merge = (((inst >> 78) & 0x1) << 0);
        let _sem = (((inst >> 79) & 0x3) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _comp = (((inst >> 87) & 0x3) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn tmml(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _b = (((inst >> 59) & 0x1) << 0);
        let _destidx = (((inst >> 61) & 0x7) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _pixmasku04 = (((inst >> 72) & 0xf) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ttucctl(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ttugo(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ttuld(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ttumacrofuse(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _sb = (((inst >> 40) & 0x1f) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ttuopen(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ttust(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn txd(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _b = (((inst >> 59) & 0x1) << 0);
        let _destidx = (((inst >> 61) & 0x7) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _pixmasku04 = (((inst >> 72) & 0xf) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _sem = (((inst >> 79) & 0x3) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _cas = (((inst >> 87) & 0x1) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn txq(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _b = (((inst >> 59) & 0x1) << 0);
        let _vtgmode = (((inst >> 62) & 0x3) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _pixmasku04 = (((inst >> 72) & 0xf) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ubmsk(&mut self, inst: u128) {
        self.bmsk(inst)
    }
    pub fn ubrev(&mut self, inst: u128) {
        self.brev(inst)
    }
    pub fn uclea(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _sb = (((inst >> 32) & 0xffff) << 0);
        let _sz = (((inst >> 73) & 0xf) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn uf2fp(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _ra_urc = (((inst >> 64) & 0x3f) << 0);
        let _merge = (((inst >> 78) & 0x1) << 0);
        let _rndmode = (((inst >> 79) & 0x7) << 0);
        let _fmt = (((inst >> 84) & 0x3) << 2) | (((inst >> 75) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn uflo(&mut self, inst: u128) {
        self.flo(inst)
    }
    pub fn uiadd3(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _ra_urc = (((inst >> 64) & 0x3f) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _pq = (((inst >> 77) & 0x7) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn uiadd3_64(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _ra_urc = (((inst >> 64) & 0x3f) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _pq = (((inst >> 77) & 0x7) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn uimad(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _ra_urc = (((inst >> 64) & 0x3f) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn uisetp(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _pr = (((inst >> 68) & 0x7) << 0);
        let _memdesc = (((inst >> 71) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _dstfmt = (((inst >> 76) & 0x7) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn uldc(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let urd = (((inst >> 16) & 0x3f) << 0) as u32;
        let ra_offset = (((inst >> 38) & 0xffff) << 0) as u32;
        let sc_bank = (((inst >> 54) & 0x1f) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        assert!(inst >> 91 & 1 == 0, "ULDC with a uniform address");
        for i in 0..access_words(sz) {
            let offset = self.cached_const_u32(ra_offset + 4 * i);
            let value = self.load_cbuf(ResourceKind::Cbuf(sc_bank), offset);
            self.store_ureg(nth_reg(urd, URZ, i), value);
        }
    }
    pub fn ulea(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _ra_urc = (((inst >> 64) & 0x3f) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _scaleu5 = (((inst >> 75) & 0x1f) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ulop3(&mut self, inst: u128) {
        self.lop3(inst)
    }
    pub fn umov(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn up2ur(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn uplop3(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _pr = (((inst >> 68) & 0x7) << 0);
        let _memdesc = (((inst >> 71) & 0x1) << 0);
        let _lop = (((inst >> 72) & 0x1f) << 11)
            | (((inst >> 64) & 0x7) << 8)
            | (((inst >> 16) & 0xff) << 0);
        let _pq = (((inst >> 77) & 0x7) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn upopc(&mut self, inst: u128) {
        self.popc(inst)
    }
    pub fn uprmt(&mut self, inst: u128) {
        self.prmt(inst)
    }
    pub fn ur2up(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn usel(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn usgxt(&mut self, inst: u128) {
        self.sgxt(inst)
    }
    pub fn ushf(&mut self, inst: u128) {
        self.shf(inst)
    }
    pub fn vabsdiff(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn vabsdiff4(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn vote(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _num = (((inst >> 72) & 0x3) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn voteu(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _num = (((inst >> 72) & 0x3) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn warpsync(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _depth = (((inst >> 86) & 0x1) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
    }
    pub fn yield_(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
}

include!("sm86_decoder_generated.rs");
