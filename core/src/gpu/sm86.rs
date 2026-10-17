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
const SHARED_WORDS: u32 = 256;
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
    Local,
    Shared,
    Warp,
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
    type_ptr_shared_u32: u32,
    type_ptr_cbuf: u32,
    type_ptr_cbuf_u32: u32,
    type_u64: u32,
    type_s64: u32,
    type_f64: u32,
    type_f16: u32,
    type_f16x2: u32,
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

// Sizes 0 to 4 are U8, S8, U16, S16 and 32 bits in one word; 5 and 6 are 64 and 128 bits.
fn access_words(sz: u32) -> u32 {
    if sz <= 4 { 1 } else { 1 << (sz - 4) }
}

// Uniform datapath ops set bit 91 with forms 1 (URb) and 2, 4 (imm32); vector ops with bit 91 read a UR through forms 5 to 7.
fn is_uniform(inst: u128) -> bool {
    inst >> 91 & 1 != 0 && matches!(inst >> 9 & 7, 1 | 2 | 4)
}

pub fn emit_module(device: Option<&VkState>, local_size: [u32; 3], body: impl FnOnce(&mut Decoder)) -> spirv::Emitter {
    let mut emitter = spirv::Emitter::new();
    emitter.emit_header();
    emitter.emit_capability(spirv::capability::SHADER);
    emitter.emit_memory_model(0, 1);
    let mut decoder = Decoder::new(&mut emitter);
    decoder.init();
    let void_ty = decoder.get_type_void();
    let func_ty = decoder.ir.emit_type_function(void_ty, &[]);
    let func = decoder.ir.emit_function(void_ty, 0, func_ty);
    decoder.ir.emit_label();
    body(&mut decoder);
    decoder.emit_entry_point(func, device, local_size);
    decoder.ir.emit_return();
    decoder.ir.emit_function_end();
    decoder.ir.finalize();
    emitter
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
            type_ptr_shared_u32: 0,
            type_ptr_cbuf: 0,
            type_ptr_cbuf_u32: 0,
            type_u64: 0,
            type_s64: 0,
            type_f64: 0,
            type_f16: 0,
            type_f16x2: 0,
            glsl_ext: 0,
            resources: Vec::new(),
            const_cache: HashMap::new(),
        }
    }

    pub fn get_type_void(&self) -> u32 {
        self.type_void
    }

    pub fn emit_entry_point(&mut self, func: u32, device: Option<&VkState>, local_size: [u32; 3]) {
        // SPIR-V 1.4+ entry points must list every global variable they use, not just Input/Output.
        let resources = self.resources.iter().map(|r| r.var);
        let interface: Vec<u32> = [self.var_abstract_state, self.var_pc].into_iter().chain(resources).collect();
        self.ir
            .emit_entry_point(spirv::execution_model::GLCOMPUTE, func, "main", &interface);
        self.ir
            .emit_execution_mode(func, spirv::execution_mode::LOCAL_SIZE, &local_size);
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
        let base = self.invocation_state_base();
        // Every register is seeded because predicated stores read the old value.
        for slot in 0..STATE_WORDS as u32 {
            let word = self.ssbo_word(input, base, slot);
            let value = self.ir.emit_load(self.type_u32[1], word);
            let ptr = self.state_slot(slot);
            self.ir.emit_store(ptr, value);
        }
    }

    pub fn emit_register_dump(&mut self) {
        let output = self.resource(ResourceKind::Ssbo(1)).unwrap();
        let base = self.invocation_state_base();
        for slot in 0..STATE_WORDS as u32 {
            let ptr = self.state_slot(slot);
            let value = self.ir.emit_load(self.type_u32[1], ptr);
            let word = self.ssbo_word(output, base, slot);
            self.ir.emit_store(word, value);
        }
    }

    fn invocation_state_base(&mut self) -> u32 {
        let invocation = self.builtin(spirv::BuiltIn::LocalInvocationIndex);
        let stride = self.cached_const_u32(STATE_WORDS as u32);
        self.ir.emit_imul(self.type_u32[1], invocation, stride)
    }

    fn workgroup_word(&mut self, kind: ResourceKind, len: u32, word: u32) -> u32 {
        use spirv::storage_class::WORKGROUP;
        let var = match self.resource(kind) {
            Some(var) => var,
            None => {
                let len = self.cached_const_u32(len);
                let array = self.ir.emit_type_array(self.type_u32[1], len);
                let ptr = self.ir.emit_type_pointer(WORKGROUP, array);
                if self.type_ptr_shared_u32 == 0 {
                    self.type_ptr_shared_u32 = self.ir.emit_type_pointer(WORKGROUP, self.type_u32[1]);
                }
                let var = self.ir.emit_variable(ptr, WORKGROUP);
                self.resources.push(Resource { kind, var });
                var
            }
        };
        self.ir.emit_access_chain(self.type_ptr_shared_u32, var, &[word])
    }

    // A warp is 32 consecutive invocations that trade values through a workgroup array, so every lane must reach the barriers.
    fn warp_exchange(&mut self, val: u32, lanes: &[u32]) -> Vec<u32> {
        use spirv::memory_semantics::{ACQUIRE_RELEASE, WORKGROUP_MEMORY};
        const MAX_INVOCATIONS: u32 = 1024;
        let ty = self.type_u32[1];
        let invocation = self.builtin(spirv::BuiltIn::LocalInvocationIndex);
        let own = self.workgroup_word(ResourceKind::Warp, MAX_INVOCATIONS, invocation);
        self.ir.emit_store(own, val);
        let [scope, semantics] = [spirv::scope::WORKGROUP, ACQUIRE_RELEASE | WORKGROUP_MEMORY].map(|c| self.cached_const_u32(c));
        self.ir.emit_control_barrier(scope, scope, semantics);
        let first_lane = self.cached_const_u32(!31);
        let base = self.ir.emit_bitwise_and(ty, invocation, first_lane);
        let mut values = Vec::new();
        for &lane in lanes {
            let index = self.ir.emit_iadd(ty, base, lane);
            let slot = self.workgroup_word(ResourceKind::Warp, MAX_INVOCATIONS, index);
            values.push(self.ir.emit_load(ty, slot));
        }
        self.ir.emit_control_barrier(scope, scope, semantics);
        values
    }

    fn warp_values(&mut self, val: u32) -> Vec<u32> {
        let lanes: Vec<u32> = (0..32).map(|lane| self.cached_const_u32(lane)).collect();
        self.warp_exchange(val, &lanes)
    }

    fn pack_lanes(&mut self, bits: Vec<u32>) -> u32 {
        let mut mask = self.const_u32_0;
        for (lane, bit) in (0..).zip(bits) {
            let offset = self.cached_const_u32(lane);
            mask = self.ir.emit_bit_field_insert(self.type_u32[1], mask, bit, offset, self.const_u32_1);
        }
        mask
    }

    fn bool_bit(&mut self, cond: u32) -> u32 {
        self.ir.emit_select(self.type_u32[1], cond, self.const_u32_1, self.const_u32_0)
    }

    fn shared_address(&mut self, inst: u128, uniform: u32) -> u32 {
        assert!(inst >> 78 & 3 == 0, "shared address stride {}", inst >> 78 & 3);
        self.global_address(inst, uniform, 40)
    }

    fn shared_word_at(&mut self, address: u32) -> u32 {
        let two = self.cached_const_u32(2);
        let index = self.ir.emit_shift_right_logical(self.type_u32[1], address, two);
        self.workgroup_word(ResourceKind::Shared, SHARED_WORDS, index)
    }

    fn atomic_update(&mut self, ptr: u32, scope: u32, sem: u32, update: impl Fn(&mut Self, u32) -> u32) -> u32 {
        let relaxed = self.cached_const_u32(spirv::memory_semantics::RELAXED);
        let [header, body, merge] = [(); 3].map(|_| self.ir.alloc_id());
        self.ir.emit_branch(header);
        self.ir.emit_label_id(header);
        self.ir.emit_loop_merge(merge, body, 0);
        self.ir.emit_branch(body);
        self.ir.emit_label_id(body);
        let old = self.ir.emit_atomic_load(self.type_u32[1], ptr, scope, relaxed);
        let new = update(self, old);
        let seen = self.ir.emit_atomic_compare_exchange(self.type_u32[1], ptr, scope, sem, relaxed, new, old);
        let done = self.ir.emit_iequal(self.type_bool[1], seen, old);
        self.ir.emit_branch_conditional(done, merge, header);
        self.ir.emit_label_id(merge);
        old
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
        let bit = self.pred_bit(uniform, pg, pg_not);
        let cond = self.ir.emit_inot_equal(self.type_bool[1], bit, self.const_u32_0);
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

    fn alu_pair(&mut self, inst: u128, bit: u32) -> u32 {
        let index = (inst >> bit) as u32;
        if is_uniform(inst) { self.load_ureg_pair(index & 0x3f) } else { self.load_pair(index & 0xff) }
    }

    fn store_alu_pair(&mut self, inst: u128, val: u32) {
        let rd = (inst >> 16) as u32;
        if !is_uniform(inst) {
            return self.store_pair(rd & 0xff, val);
        }
        let pair = self.ir.emit_bitcast(self.type_u32[2], val);
        for i in 0..2 {
            let word = self.ir.emit_composite_extract(self.type_u32[1], pair, &[i]);
            self.store_ureg(nth_reg(rd & 0x3f, URZ, i), word);
        }
    }

    fn pred_bit(&mut self, uniform: bool, pred: u32, invert: bool) -> u32 {
        let bit = if pred == PT {
            self.const_u32_1
        } else {
            let preds = if uniform { self.load_upredicate() } else { self.load_predicate() };
            let offset = self.cached_const_u32(pred);
            self.ir.emit_bit_field_u_extract(self.type_u32[1], preds, offset, self.const_u32_1)
        };
        if invert { self.ir.emit_bitwise_xor(self.type_u32[1], bit, self.const_u32_1) } else { bit }
    }

    fn pp_bit(&mut self, inst: u128) -> u32 {
        self.pred_bit(is_uniform(inst), (inst >> 87 & 7) as u32, inst >> 90 & 1 != 0)
    }

    fn pp_set(&mut self, inst: u128) -> u32 {
        let bit = self.pp_bit(inst);
        self.ir.emit_inot_equal(self.type_bool[1], bit, self.const_u32_0)
    }

    fn store_pred_bit(&mut self, uniform: bool, pred: u32, bit: u32) {
        if pred == PT {
            return;
        }
        let preds = if uniform { self.load_upredicate() } else { self.load_predicate() };
        let offset = self.cached_const_u32(pred);
        let preds = self.ir.emit_bit_field_insert(self.type_u32[1], preds, bit, offset, self.const_u32_1);
        if uniform { self.store_upredicate(preds) } else { self.store_predicate(preds) }
    }

    // UP2UR and UR2UP move UP0..UP6 only, masked by imm32 (form 4) or URb.
    fn up_mask(&mut self, inst: u128) -> u32 {
        let mask = if inst >> 9 & 7 == 4 { self.cached_const_u32((inst >> 32) as u32) } else { self.load_ureg((inst >> 32 & 0x3f) as u32) };
        let predicates = self.cached_const_u32(0x7f);
        self.ir.emit_bitwise_and(self.type_u32[1], mask, predicates)
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
            (2 | 4, 0) => [self.const_u32_0, self.cached_const_u32(field)],
            (3 | 5, 0) => {
                let offset = (field >> 8 & 0x3fff) << 2;
                let bank = ResourceKind::Cbuf(field >> 22 & 0x1f);
                let lo_offset = self.cached_const_u32(offset);
                let hi_offset = self.cached_const_u32(offset + 4);
                [self.load_cbuf(bank, lo_offset), self.load_cbuf(bank, hi_offset)]
            }
            (6 | 7, 1) => return self.load_ureg_pair(field & 0x3f),
            form => todo!("64-bit ALU source form {form:?}"),
        };
        self.join_u64(words[0], words[1])
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

    fn global_word_at(&mut self, address: u32) -> u32 {
        let var = match self.resource(ResourceKind::Global) {
            Some(var) => var,
            None => self.declare_ssbo(ResourceKind::Global, GLOBAL_BINDING),
        };
        let base = self.load_cbuf(ResourceKind::Support, self.const_u32_0);
        let rel = self.ir.emit_isub(self.type_u32[1], address, base);
        let two = self.cached_const_u32(2);
        let index = self.ir.emit_shift_right_logical(self.type_u32[1], rel, two);
        self.ir.emit_access_chain(self.type_ptr_ssbo_u32, var, &[self.const_u32_0, index])
    }

    fn global_address(&mut self, inst: u128, uniform: u32, offset_shift: u32) -> u32 {
        let mut address = self.load_register((inst >> 24) as u32 & 0xff);
        if inst >> 91 & 1 != 0 {
            let ur = (inst >> uniform & 0x3f) as u32;
            let value = self.load_ureg(ur);
            address = self.ir.emit_iadd(self.type_u32[1], address, value);
        }
        let offset = if inst >> 91 & 1 != 0 || offset_shift == 40 {
            (((inst >> 40) as i32) << 8 >> 8) as u32
        } else {
            (inst >> 32) as u32
        };
        let offset = self.cached_const_u32(offset);
        self.ir.emit_iadd(self.type_u32[1], address, offset)
    }

    fn global_atomic(&mut self, inst: u128, op: u32, sz: u32, rb: u32) -> u32 {
        use spirv::memory_semantics::{ACQUIRE_RELEASE, UNIFORM_MEMORY};
        if sz >= 2 {
            todo!("64-bit and float global atomics");
        }
        let address = self.global_address(inst, 64, 40);
        let ptr = self.global_word_at(address);
        let value = self.load_register(rb);
        let scope = self.cached_const_u32(spirv::scope::DEVICE);
        let sem = self.cached_const_u32(ACQUIRE_RELEASE | UNIFORM_MEMORY);
        if inst & 1 != 0 {
            let swap = self.load_register((inst >> 64 & 0xff) as u32);
            return self.ir.emit_atomic_compare_exchange(self.type_u32[1], ptr, scope, sem, self.const_u32_0, swap, value);
        }
        self.atomic_op(ptr, scope, sem, op, sz == 1, value)
    }

    fn atomic_op(&mut self, ptr: u32, scope: u32, sem: u32, op: u32, signed: bool, b: u32) -> u32 {
        let ty = self.type_u32[1];
        match op {
            0 => self.ir.emit_atomic_iadd(ty, ptr, scope, sem, b),
            1 if signed => self.ir.emit_atomic_smin(ty, ptr, scope, sem, b),
            1 => self.ir.emit_atomic_umin(ty, ptr, scope, sem, b),
            2 if signed => self.ir.emit_atomic_smax(ty, ptr, scope, sem, b),
            2 => self.ir.emit_atomic_umax(ty, ptr, scope, sem, b),
            3 => self.atomic_update(ptr, scope, sem, |d, old| {
                let wraps = d.ir.emit_ugreater_than_equal(d.type_bool[1], old, b);
                let next = d.ir.emit_iadd(ty, old, d.const_u32_1);
                d.ir.emit_select(ty, wraps, d.const_u32_0, next)
            }),
            4 => self.atomic_update(ptr, scope, sem, |d, old| {
                let zero = d.ir.emit_iequal(d.type_bool[1], old, d.const_u32_0);
                let above = d.ir.emit_ugreater_than(d.type_bool[1], old, b);
                let wraps = d.ir.emit_logical_or(d.type_bool[1], zero, above);
                let next = d.ir.emit_isub(ty, old, d.const_u32_1);
                d.ir.emit_select(ty, wraps, b, next)
            }),
            5 => self.ir.emit_atomic_and(ty, ptr, scope, sem, b),
            6 => self.ir.emit_atomic_or(ty, ptr, scope, sem, b),
            7 => self.ir.emit_atomic_xor(ty, ptr, scope, sem, b),
            8 => self.ir.emit_atomic_exchange(ty, ptr, scope, sem, b),
            _ => todo!("atomic op {op}"),
        }
    }

    // Local memory is a lazy 4 KiB Private array per invocation.
    fn local_word_at(&mut self, address: u32) -> u32 {
        let var = match self.resource(ResourceKind::Local) {
            Some(var) => var,
            None => {
                let len = self.cached_const_u32(1024);
                let array = self.ir.emit_type_array(self.type_u32[1], len);
                let ptr = self.ir.emit_type_pointer(storage_class::PRIVATE, array);
                let var = self.ir.emit_variable(ptr, storage_class::PRIVATE);
                self.resources.push(Resource { kind: ResourceKind::Local, var });
                var
            }
        };
        let two = self.cached_const_u32(2);
        let index = self.ir.emit_shift_right_logical(self.type_u32[1], address, two);
        self.ir.emit_access_chain(self.type_ptr_u32[1], var, &[index])
    }

    fn load_narrow(&mut self, word: u32, address: u32, size: u32) -> u32 {
        if size == 4 {
            return word;
        }
        let three = self.cached_const_u32(3);
        let byte = self.ir.emit_bitwise_and(self.type_u32[1], address, three);
        let eight = self.cached_const_u32(8);
        let offset = self.ir.emit_imul(self.type_u32[1], byte, eight);
        let width = self.cached_const_u32(if size < 2 { 8 } else { 16 });
        if size & 1 != 0 {
            self.ir.emit_bit_field_s_extract(self.type_u32[1], word, offset, width)
        } else {
            self.ir.emit_bit_field_u_extract(self.type_u32[1], word, offset, width)
        }
    }

    fn store_narrow(&mut self, kind: ResourceKind, ptr: u32, address: u32, size: u32, value: u32) {
        if size >= 4 {
            return self.ir.emit_store(ptr, value);
        }
        let ty = self.type_u32[1];
        let three = self.cached_const_u32(3);
        let byte = self.ir.emit_bitwise_and(ty, address, three);
        let offset = self.ir.emit_shift_left_logical(ty, byte, three);
        let low = self.cached_const_u32(if size < 2 { 0xff } else { 0xffff });
        let mask = self.ir.emit_shift_left_logical(ty, low, offset);
        let keep = self.ir.emit_not(ty, mask);
        let value = self.ir.emit_shift_left_logical(ty, value, offset);
        let value = self.ir.emit_bitwise_and(ty, value, mask);
        let scope = match kind {
            ResourceKind::Local => {
                let word = self.ir.emit_load(ty, ptr);
                let word = self.ir.emit_bitwise_and(ty, word, keep);
                let word = self.ir.emit_bitwise_or(ty, word, value);
                return self.ir.emit_store(ptr, word);
            }
            ResourceKind::Shared => spirv::scope::WORKGROUP,
            _ => spirv::scope::DEVICE,
        };
        // Lanes storing other bytes of the same word race, so the bytes are cleared and set atomically.
        let scope = self.cached_const_u32(scope);
        let relaxed = self.cached_const_u32(spirv::memory_semantics::RELAXED);
        self.ir.emit_atomic_and(ty, ptr, scope, relaxed, keep);
        self.ir.emit_atomic_or(ty, ptr, scope, relaxed, value);
    }

    fn memory_word_at(&mut self, kind: ResourceKind, address: u32) -> u32 {
        match kind {
            ResourceKind::Global => self.global_word_at(address),
            ResourceKind::Local => self.local_word_at(address),
            ResourceKind::Shared => self.shared_word_at(address),
            _ => unreachable!(),
        }
    }

    fn load_memory(&mut self, kind: ResourceKind, address: u32, size: u32, dst: u32) {
        for i in 0..access_words(size) {
            let offset = self.cached_const_u32(4 * i);
            let word_address = self.ir.emit_iadd(self.type_u32[1], address, offset);
            let ptr = self.memory_word_at(kind, word_address);
            let word = self.ir.emit_load(self.type_u32[1], ptr);
            let value = if size <= 4 { self.load_narrow(word, address, size) } else { word };
            self.store_register(nth_reg(dst, RZ, i), value);
        }
    }

    fn store_memory(&mut self, kind: ResourceKind, address: u32, size: u32, src: u32) {
        for i in 0..access_words(size) {
            let offset = self.cached_const_u32(4 * i);
            let word_address = self.ir.emit_iadd(self.type_u32[1], address, offset);
            let ptr = self.memory_word_at(kind, word_address);
            let value = self.load_register(nth_reg(src, RZ, i));
            self.store_narrow(kind, ptr, address, size, value);
        }
    }

    fn load_dynamic_cbuf(&mut self, bank: u32, offset: u32) -> u32 {
        let mut value = self.load_cbuf(ResourceKind::Cbuf(0), offset);
        for slot in 1..8 {
            let candidate = self.load_cbuf(ResourceKind::Cbuf(slot), offset);
            let slot = self.cached_const_u32(slot);
            let selected = self.ir.emit_iequal(self.type_bool[1], bank, slot);
            value = self.ir.emit_select(self.type_u32[1], selected, candidate, value);
        }
        value
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

    // Float16 is declared on first use so fp32 shaders still run on devices without shaderFloat16.
    fn type_f16(&mut self) -> u32 {
        if self.type_f16 == 0 {
            self.ir.emit_capability(spirv::capability::FLOAT16);
            self.type_f16 = self.ir.emit_type_float(16);
        }
        self.type_f16
    }

    fn type_f16x2(&mut self) -> u32 {
        if self.type_f16x2 == 0 {
            let half = self.type_f16();
            self.type_f16x2 = self.ir.emit_type_vector(half, 2);
        }
        self.type_f16x2
    }

    // Component 0 is bits 15:0 (H0), so bitcast_u32 is the inverse.
    fn unpack_half2(&mut self, word: u32) -> u32 {
        let ty = self.type_f16x2();
        self.ir.emit_bitcast(ty, word)
    }

    // ISWZ: 0 = H1_H0, 2 = H0_H0, 3 = H1_H1; 1 (F32) reads a whole f32 and is not a swizzle.
    fn half2_swizzle(&mut self, half2: u32, iswz: u32) -> u32 {
        let lanes = match iswz {
            0 => return half2,
            2 => [0, 0],
            3 => [1, 1],
            _ => panic!("ISWZ {iswz} is not a half2 swizzle"),
        };
        let ty = self.type_f16x2();
        self.ir.emit_vector_shuffle(ty, half2, half2, &lanes)
    }

    // Each half with a zero exponent keeps only its sign.
    fn flush_half2_subnormal(&mut self, word: u32) -> u32 {
        let ty = self.type_u32[1];
        let mut keep = self.const_u32_0;
        for half in [0xffff, 0xffff_0000] {
            let [exponent, half, sign] = [half & 0x7c00_7c00, half, half & 0x8000_8000].map(|c| self.cached_const_u32(c));
            let exp = self.ir.emit_bitwise_and(ty, word, exponent);
            let subnormal = self.ir.emit_iequal(self.type_bool[1], exp, self.const_u32_0);
            let lane = self.ir.emit_select(ty, subnormal, sign, half);
            keep = self.ir.emit_bitwise_or(ty, keep, lane);
        }
        self.ir.emit_bitwise_and(ty, word, keep)
    }

    // Lanes come out as exact f32 so the fp32 helpers apply; ISWZ 1 (F32) rounds the whole register to f16 for both lanes.
    fn half2_lanes(&mut self, word: u32, iswz: u32, abs: bool, neg: bool, ftz: bool) -> [u32; 2] {
        let (half, half2_ty) = (self.type_f16(), self.type_f16x2());
        let (word, iswz) = if iswz == 1 {
            let val = self.bitcast_f32(word);
            let val = self.ir.emit_f_convert(half, val);
            let half2 = self.ir.emit_composite_construct(half2_ty, &[val, val]);
            (self.bitcast_u32(half2), 0)
        } else {
            (word, iswz)
        };
        let word = if ftz { self.flush_half2_subnormal(word) } else { word };
        let half2 = self.unpack_half2(word);
        let half2 = self.half2_swizzle(half2, iswz);
        [0, 1].map(|i| {
            let lane = self.ir.emit_composite_extract(half, half2, &[i]);
            let lane = self.ir.emit_f_convert(self.type_f32[1], lane);
            self.fp_src(self.type_f32[1], lane, abs, neg)
        })
    }

    fn half2_reg(&mut self, inst: u128, reg_lsb: u32, iswz_lsb: u32, abs_bit: u32, neg_bit: u32, ftz: bool) -> [u32; 2] {
        let word = self.load_register((inst >> reg_lsb & 0xff) as u32);
        self.half2_lanes(word, (inst >> iswz_lsb & 3) as u32, inst >> abs_bit & 1 != 0, inst >> neg_bit & 1 != 0, ftz)
    }

    // An immediate (form 2 or 4) is two raw halves filling bits 32..64, so it has no swizzle (61:60), |x| (62) or -x (63).
    fn half2_alu_src(&mut self, inst: u128, ftz: bool) -> [u32; 2] {
        let word = self.alu_src(inst);
        if matches!(inst >> 9 & 7, 2 | 4) {
            return self.half2_lanes(word, 0, false, false, ftz);
        }
        self.half2_lanes(word, (inst >> 60 & 3) as u32, inst >> 62 & 1 != 0, inst >> 63 & 1 != 0, ftz)
    }

    fn pack_half2(&mut self, lanes: [u32; 2]) -> u32 {
        let half = self.type_f16();
        let halves = lanes.map(|lane| self.ir.emit_f_convert(half, lane));
        let ty = self.type_f16x2();
        let half2 = self.ir.emit_composite_construct(ty, &halves);
        self.bitcast_u32(half2)
    }

    // .F32 writes lane 0 as one f32 instead of packing both lanes; .SAT maps NaN to +0 as in fp_arith_result.
    fn store_half2(&mut self, inst: u128, lanes: [u32; 2], f32_out: bool, sat: bool, ftz: bool) {
        let lanes = if sat {
            let [zero, one] = [0.0f32, 1.0].map(|c| self.ir.emit_constant_typed(self.type_f32[1], c));
            lanes.map(|lane| self.glsl(81, self.type_f32[1], &[lane, zero, one]))
        } else {
            lanes
        };
        let bits = if f32_out { self.bitcast_u32(lanes[0]) } else { self.pack_half2(lanes) };
        let bits = match (ftz, f32_out) {
            (false, _) => bits,
            (true, true) => self.flush_subnormal(bits),
            (true, false) => self.flush_half2_subnormal(bits),
        };
        self.store_register((inst >> 16 & 0xff) as u32, bits);
    }

    // Bits 70:69 combine each lane's comparison with Pp (89:87, inverted by 90); .H_AND first turns the lanes into both and !both.
    fn half2_compare(&mut self, inst: u128, h_and: bool) -> [u32; 2] {
        let ftz = inst >> 80 & 1 != 0;
        let a = self.half2_reg(inst, 24, 74, 73, 72, ftz);
        let b = self.half2_alu_src(inst, ftz);
        let op = (inst >> 76 & 0xf) as u32;
        let mut lanes = [0, 1].map(|i| self.fp_compare(op, a[i], b[i]));
        if h_and {
            let both = self.ir.emit_bitwise_and(self.type_u32[1], lanes[0], lanes[1]);
            lanes = [both, self.ir.emit_bitwise_xor(self.type_u32[1], both, self.const_u32_1)];
        }
        let p = self.pp_bit(inst);
        lanes.map(|lane| self.bool_op(inst >> 69 & 3, lane, p))
    }

    // Pu (83:81) takes the result and Pv (86:84) its inverse, each combined with Pp by the op at 75:74.
    fn store_setp(&mut self, inst: u128, result: u32) {
        let p = self.pp_bit(inst);
        let inverse = self.ir.emit_bitwise_xor(self.type_u32[1], result, self.const_u32_1);
        for (value, lsb) in [(result, 81), (inverse, 84)] {
            let value = self.bool_op(inst >> 74 & 3, value, p);
            self.store_pred_bit(is_uniform(inst), (inst >> lsb & 7) as u32, value);
        }
    }

    fn bool_op(&mut self, op: u128, a: u32, b: u32) -> u32 {
        let ty = self.type_u32[1];
        match op {
            0 => self.ir.emit_bitwise_and(ty, a, b),
            1 => self.ir.emit_bitwise_or(ty, a, b),
            2 => self.ir.emit_bitwise_xor(ty, a, b),
            _ => panic!("boolean op {op} is invalid"),
        }
    }

    // B fills the low `bits`, A the next `bits` and the low bits of C the rest, as PTX cvt.pack.
    fn pack_narrow(&mut self, a: u32, b: u32, c: u32, bits: u32) -> u32 {
        let ty = self.type_u32[1];
        let base = if bits < 16 {
            let shift = self.cached_const_u32(2 * bits);
            self.ir.emit_shift_left_logical(ty, c, shift)
        } else {
            self.const_u32_0
        };
        let width = self.cached_const_u32(bits);
        let low = self.ir.emit_bit_field_insert(ty, base, b, self.const_u32_0, width);
        self.ir.emit_bit_field_insert(ty, low, a, width, width)
    }

    fn high_half_if(&mut self, word: u32, high: bool) -> u32 {
        if !high {
            return word;
        }
        let shift = self.cached_const_u32(16);
        self.ir.emit_shift_right_logical(self.type_u32[1], word, shift)
    }

    // OpBitcast puts vector component 0 in the low bits, so (Rn, Rn+1) is (lo, hi).
    fn join_u64(&mut self, lo: u32, hi: u32) -> u32 {
        let pair = self.ir.emit_composite_construct(self.type_u32[2], &[lo, hi]);
        let ty = self.type_u64();
        self.ir.emit_bitcast(ty, pair)
    }

    fn load_pair(&mut self, reg: u32) -> u32 {
        let [lo, hi] = [0, 1].map(|i| self.load_register(nth_reg(reg, RZ, i)));
        self.join_u64(lo, hi)
    }

    fn load_ureg_pair(&mut self, ur: u32) -> u32 {
        let [lo, hi] = [0, 1].map(|i| self.load_ureg(nth_reg(ur, URZ, i)));
        self.join_u64(lo, hi)
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

    fn builtin(&mut self, builtin: spirv::BuiltIn) -> u32 {
        use spirv::{BuiltIn, decoration, storage_class::INPUT};
        let ty = match builtin {
            BuiltIn::LocalInvocationIndex => self.type_u32[1],
            _ => self.type_u32[3],
        };
        let builtin = builtin as u32;
        let var = match self.resource(ResourceKind::BuiltIn(builtin)) {
            Some(var) => var,
            None => {
                let ptr = self.ir.emit_type_pointer(INPUT, ty);
                let var = self.ir.emit_variable(ptr, INPUT);
                self.ir.emit_decorate(var, decoration::BUILTIN, &[builtin]);
                self.resources.push(Resource { kind: ResourceKind::BuiltIn(builtin), var });
                var
            }
        };
        self.ir.emit_load(ty, var)
    }

    fn special_register(&mut self, index: u32) -> u32 {
        use spirv::BuiltIn;
        // SR_VIRTID holds the warp id at bit 8, SR_TID packs x, y and z at bits 0, 16 and 26.
        match index {
            0 => {
                let invocation = self.builtin(BuiltIn::LocalInvocationIndex);
                let mask = self.cached_const_u32(31);
                self.ir.emit_bitwise_and(self.type_u32[1], invocation, mask)
            }
            3 => {
                let invocation = self.builtin(BuiltIn::LocalInvocationIndex);
                let [five, eight] = [5, 8].map(|i| self.cached_const_u32(i));
                let warp = self.ir.emit_shift_right_logical(self.type_u32[1], invocation, five);
                self.ir.emit_shift_left_logical(self.type_u32[1], warp, eight)
            }
            32 => {
                let tid = self.builtin(BuiltIn::LocalInvocationId);
                let x = self.ir.emit_composite_extract(self.type_u32[1], tid, &[0]);
                let y = self.ir.emit_composite_extract(self.type_u32[1], tid, &[1]);
                let z = self.ir.emit_composite_extract(self.type_u32[1], tid, &[2]);
                let [o16, c10, o26, c6] = [16, 10, 26, 6].map(|i| self.cached_const_u32(i));
                let xy = self.ir.emit_bit_field_insert(self.type_u32[1], x, y, o16, c10);
                self.ir.emit_bit_field_insert(self.type_u32[1], xy, z, o26, c6)
            }
            33..=35 => {
                let tid = self.builtin(BuiltIn::LocalInvocationId);
                self.ir.emit_composite_extract(self.type_u32[1], tid, &[index - 33])
            }
            37..=39 => {
                let ctaid = self.builtin(BuiltIn::WorkgroupId);
                self.ir.emit_composite_extract(self.type_u32[1], ctaid, &[index - 37])
            }
            56..=60 => {
                let ty = self.type_u32[1];
                let lane = self.special_register(0);
                let eq = self.ir.emit_shift_left_logical(ty, self.const_u32_1, lane);
                let lt = self.ir.emit_isub(ty, eq, self.const_u32_1);
                let le = self.ir.emit_bitwise_or(ty, lt, eq);
                match index {
                    56 => eq,
                    57 => lt,
                    58 => le,
                    59 => self.ir.emit_not(ty, le),
                    _ => self.ir.emit_not(ty, lt),
                }
            }
            1 | 2 | 4..=31 | 36 | 40..=55 | 61..=133 | SRZ => self.const_u32_0,
            _ => panic!("unknown special register {index}"),
        }
    }

    fn bitcast_f32(&mut self, val: u32) -> u32 {
        self.ir.emit_bitcast(self.type_f32[1], val)
    }

    fn bitcast_u32(&mut self, val: u32) -> u32 {
        self.ir.emit_bitcast(self.type_u32[1], val)
    }

    fn bitcast_f64(&mut self, val: u32) -> u32 {
        let ty = self.type_f64();
        self.ir.emit_bitcast(ty, val)
    }

    fn bitcast_u64(&mut self, val: u32) -> u32 {
        let ty = self.type_u64();
        self.ir.emit_bitcast(ty, val)
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
        let imm = matches!(inst >> 9 & 7, 2 | 4);
        self.fp64_src(val, !imm && inst >> 62 & 1 != 0, !imm && inst >> 63 & 1 != 0)
    }

    fn fp64_src(&mut self, val: u32, abs: bool, neg: bool) -> u32 {
        let ty = self.type_f64();
        self.fp_src(ty, val, abs, neg)
    }

    fn fp64_rounding(inst: u128) {
        let mode = (inst >> 78 & 3) as usize;
        assert!(mode == 0, "{} rounding mode is not implemented", ["RN", "RM", "RP", "RZ"][mode]);
    }

    fn fp_compare(&mut self, op: u32, a: u32, b: u32) -> u32 {
        let bool_ty = self.type_bool[1];
        let cmp = match op {
            0 => self.ir.emit_iequal(bool_ty, self.const_u32_0, self.const_u32_1),
            1 => self.ir.emit_ford_less_than(bool_ty, a, b),
            2 => self.ir.emit_ford_equal(bool_ty, a, b),
            3 => self.ir.emit_ford_less_than_equal(bool_ty, a, b),
            4 => self.ir.emit_ford_greater_than(bool_ty, a, b),
            5 => self.ir.emit_ford_not_equal(bool_ty, a, b),
            6 => self.ir.emit_ford_greater_than_equal(bool_ty, a, b),
            7 | 8 => {
                let a_nan = self.ir.emit_f_is_nan(bool_ty, a);
                let b_nan = self.ir.emit_f_is_nan(bool_ty, b);
                let unordered = self.ir.emit_logical_or(bool_ty, a_nan, b_nan);
                if op == 7 { self.ir.emit_logical_not(bool_ty, unordered) } else { unordered }
            }
            9 => self.ir.emit_funord_less_than(bool_ty, a, b),
            10 => self.ir.emit_funord_equal(bool_ty, a, b),
            11 => self.ir.emit_funord_less_than_equal(bool_ty, a, b),
            12 => self.ir.emit_funord_greater_than(bool_ty, a, b),
            13 => self.ir.emit_funord_not_equal(bool_ty, a, b),
            14 => self.ir.emit_funord_greater_than_equal(bool_ty, a, b),
            15 => self.ir.emit_iequal(bool_ty, self.const_u32_0, self.const_u32_0),
            _ => unreachable!(),
        };
        self.bool_bit(cmp)
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

    fn ssbo_word(&mut self, var: u32, base: u32, offset: u32) -> u32 {
        let offset = self.cached_const_u32(offset);
        let index = self.ir.emit_iadd(self.type_u32[1], base, offset);
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

    fn load_predicate(&mut self) -> u32 {
        let ptr = self.ir.emit_access_chain(self.type_ptr_u32[1], self.var_abstract_state, &[self.const_u32_1]);
        self.ir.emit_load(self.type_u32[1], ptr)
    }


    fn store_predicate(&mut self, val: u32) {
        let ptr = self.ir.emit_access_chain(self.type_ptr_u32[1], self.var_abstract_state, &[self.const_u32_1]);
        self.ir.emit_store(ptr, val)
    }


    pub fn finish(&mut self) {

    }

    // %rd := %ra + $ra_offset
    pub fn al2p(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let ra = (((inst >> 24) & 0xff) << 0) as u32;
        let ra_offset = (((inst >> 40) & 0x7ff) << 0) as u32;
        let bop = (((inst >> 74) & 0x3) << 0) as u32;
        assert!(bop == BitSize::B32 as u32);
        let base = self.load_register(ra);
        let offset = self.ir.emit_constant_typed(self.type_u32[1], ra_offset as u32);
        let dst_val = self.ir.emit_iadd(self.type_u32[1], base, offset);
        self.store_register(rd, dst_val);
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
        match opcode(inst) {
            0x1f8a => todo!("ATOM.ARRIVE and ATOM.POPC.INC"),
            0x38b if inst >> 87 & 1 == 1 => todo!("ATOM.CAST"),
            _ => self.atomg(inst),
        }
    }
    pub fn atomg(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let rb = (((inst >> 32) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0xf) << 0) as u32;
        let op = (((inst >> 87) & 0xf) << 0) as u32;
        let old = self.global_atomic(inst, op, sz, rb);
        self.store_register(rd, old);
    }
    pub fn atoms(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let rb = (((inst >> 32) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0x3) << 0) as u32;
        let op = (((inst >> 87) & 0xf) << 0) as u32;
        let cas = opcode(inst) == 0x38d;
        match opcode(inst) {
            0x1f8c => todo!("ATOMS.ARRIVE and ATOMS.POPC.INC"),
            0x38d if op & 1 == 1 => todo!("ATOMS.CAST"),
            _ if sz >= 2 => todo!("64-bit ATOMS"),
            _ => {}
        }
        let address = self.shared_address(inst, 64);
        let ptr = self.shared_word_at(address);
        let b = self.load_register(rb);
        let (ty, signed) = (self.type_u32[1], sz == 1);
        let scope = self.cached_const_u32(spirv::scope::WORKGROUP);
        let relaxed = self.cached_const_u32(spirv::memory_semantics::RELAXED);
        let old = if cas {
            let c = self.load_register((inst >> 64) as u32 & 0xff);
            self.ir.emit_atomic_compare_exchange(ty, ptr, scope, relaxed, relaxed, c, b)
        } else {
            self.atomic_op(ptr, scope, relaxed, op, signed, b)
        };
        self.store_register(rd, old);
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
        let pq = (((inst >> 77) & 0x7) << 0) as u32;
        if !matches!(pq, 0 | 4) {
            todo!("BAR mode {pq}");
        }
        use spirv::memory_semantics::{ACQUIRE_RELEASE, WORKGROUP_MEMORY};
        let workgroup = self.cached_const_u32(spirv::scope::WORKGROUP);
        let semantics = self.cached_const_u32(ACQUIRE_RELEASE | WORKGROUP_MEMORY);
        self.ir.emit_control_barrier(workgroup, workgroup, semantics);
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
        let cop = (((inst >> 84) & 0x7) << 0);
        panic!("BPT mode {cop} breaks into a debugger, which a compute shader has none of");
    }
    pub fn bra(&mut self, inst: u128) {
        let cond = (((inst >> 32) & 0x3) << 0);
        let upq = (((inst >> 24) & 0x7) << 0) as u32;
        let upq_not = (((inst >> 27) & 0x1) << 0) as u32;
        let pp = (((inst >> 87) & 0x7) << 0) as u32;
        let pp_not = (((inst >> 90) & 0x1) << 0);
        assert!(pp == PT && pp_not == 0, "BRA Pp not implemented");
        // .DIV and .CONV test the warp for divergence, and one invocation per register file is always converged.
        if cond == 2 {
            return;
        }
        let mut target = self.cached_const_u32(branch_target(self.pc, inst).unwrap());
        if opcode(inst) == BRA_UPRED {
            let bit = self.pred_bit(true, upq, upq_not != 0);
            let taken = self.ir.emit_inot_equal(self.type_bool[1], bit, self.const_u32_0);
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
        let pp = (((inst >> 87) & 0x7) << 0) as u32;
        let pp_not = (((inst >> 90) & 0x1) << 0);
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
    pub fn bssy(&mut self, _inst: u128) {
        // BSSY and BSYNC reconverge a diverged warp, and one invocation per register file never diverges.
    }
    pub fn bsync(&mut self, _inst: u128) {
    }
    pub fn call(&mut self, inst: u128) {
        let pp = (((inst >> 87) & 0x7) << 0) as u32;
        let pp_not = (((inst >> 90) & 0x1) << 0);
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
    pub fn cctl(&mut self, _inst: u128) {
        // Cache control has no observable effect in the single-invocation model.
    }
    pub fn cctll(&mut self, _inst: u128) {
        // Cache control has no observable effect in the single-invocation model.
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
        let ty = self.type_u64();
        let form = inst >> 9 & 7;
        let swapped = matches!(form, 2 | 3 | 7);
        let a = self.load_pair((inst >> 24 & 0xff) as u32);
        let b = if swapped {
            self.load_pair((inst >> 64 & 0xff) as u32)
        } else {
            self.alu_src_pair(inst)
        };
        let mut result = if swapped {
            self.alu_src_pair(inst)
        } else {
            self.load_pair((inst >> 64 & 0xff) as u32)
        };
        let zero = self.ir.emit_constant_typed(ty, 0u64);
        let one = self.ir.emit_constant_typed(ty, 1u64);
        for i in 0..64 {
            let shift = self.ir.emit_constant_typed(ty, i as u64);
            let bit = self.ir.emit_shift_right_logical(ty, a, shift);
            let bit = self.ir.emit_bitwise_and(ty, bit, one);
            let mask = self.ir.emit_isub(ty, zero, bit);
            let product = if inst >> 77 & 1 == 0 {
                self.ir.emit_shift_left_logical(ty, b, shift)
            } else if i == 0 {
                zero
            } else {
                let shift = self.ir.emit_constant_typed(ty, (64 - i) as u64);
                self.ir.emit_shift_right_logical(ty, b, shift)
            };
            let product = self.ir.emit_bitwise_and(ty, product, mask);
            result = self.ir.emit_bitwise_xor(ty, result, product);
        }
        self.store_pair((inst >> 16 & 0xff) as u32, result);
    }
    pub fn cs2r(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let sr = (((inst >> 72) & 0xff) << 0) as u32;
        let wide = (((inst >> 80) & 0x1) << 0) as u32;
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
        Self::fp64_rounding(inst);
        let ty = self.type_f64();
        let a = self.load_pair((inst >> 24 & 0xff) as u32);
        let a = self.bitcast_f64(a);
        let a = self.fp64_src(a, inst >> 73 & 1 != 0, inst >> 72 & 1 != 0);
        let form = inst >> 9 & 7;
        let b = if form == 1 { self.load_pair((inst >> 64 & 0xff) as u32) } else { self.alu_src_pair(inst) };
        let b = self.bitcast_f64(b);
        let imm = matches!(form, 2 | 4);
        let (abs, neg) = if form == 1 { (inst >> 74 & 1 != 0, inst >> 75 & 1 != 0) } else { (inst >> 62 & 1 != 0, inst >> 63 & 1 != 0) };
        let b = self.fp64_src(b, !imm && abs, !imm && neg);
        let result = self.ir.emit_fadd(ty, a, b);
        let result = self.bitcast_u64(result);
        self.store_pair((inst >> 16 & 0xff) as u32, result);
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
        Self::fp64_rounding(inst);
        let ty = self.type_f64();
        let a = self.load_pair((inst >> 24 & 0xff) as u32);
        let a = self.bitcast_f64(a);
        let a = self.fp64_src(a, inst >> 73 & 1 != 0, inst >> 72 & 1 != 0);
        let src = self.fp64_alu_src(inst);
        let c = self.load_pair((inst >> 64 & 0xff) as u32);
        let c = self.bitcast_f64(c);
        let c = self.fp64_src(c, inst >> 74 & 1 != 0, inst >> 75 & 1 != 0);
        let [b, c] = if matches!(inst >> 9 & 7, 2 | 3 | 7) { [c, src] } else { [src, c] };
        let result = self.glsl(spirv::glsl::FMA, ty, &[a, b, c]);
        let result = self.bitcast_u64(result);
        self.store_pair((inst >> 16 & 0xff) as u32, result);
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
        Self::fp64_rounding(inst);
        let ty = self.type_f64();
        let a = self.load_pair((inst >> 24 & 0xff) as u32);
        let a = self.bitcast_f64(a);
        let a = self.fp64_src(a, inst >> 73 & 1 != 0, inst >> 72 & 1 != 0);
        let b = self.fp64_alu_src(inst);
        let result = self.ir.emit_fmul(ty, a, b);
        let result = self.bitcast_u64(result);
        self.store_pair((inst >> 16 & 0xff) as u32, result);
    }
    pub fn dsetp(&mut self, inst: u128) {
        let a = self.load_pair((inst >> 24 & 0xff) as u32);
        let a = self.bitcast_f64(a);
        let a = self.fp64_src(a, inst >> 73 & 1 != 0, inst >> 72 & 1 != 0);
        let b = self.fp64_alu_src(inst);
        let value = self.fp_compare((inst >> 76 & 15) as u32, a, b);
        self.store_setp(inst, value);
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
        let pp = (((inst >> 87) & 0x7) << 0) as u32;
        let pp_not = (((inst >> 90) & 0x1) << 0);
        assert!(pp == PT && pp_not == 0, "EXIT Pp not implemented");
        let exit = self.cached_const_u32(u32::MAX);
        self.ir.emit_store(self.var_pc, exit);
    }
    pub fn f2f(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0);
        let rnd = ((inst >> 78) & 3) as u32;
        let ftz = ((inst >> 80) & 1) != 0;
        let fmt = (((inst >> 84) & 0x7) << 3) | (((inst >> 75) & 0x7) << 0);
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
        assert!(inst >> 75 & 1 == 0 && inst >> 77 & 1 == 0, "F2FP .RELU and .SATFINITE are not implemented");
        assert!(inst >> 76 & 1 == 0 && inst >> 86 & 1 == 0, "F2FP to BF16, E8M7 or E6M9 needs a bfloat16 type");
        assert!(inst >> 79 & 7 == 0, "F2FP rounding other than RN is not implemented");
        let (b, c) = self.srcs_bc(inst);
        let b = self.bitcast_f32(b);
        let word = if inst >> 78 & 1 == 0 {
            let a = self.load_register((inst >> 24 & 0xff) as u32);
            let a = self.bitcast_f32(a);
            self.pack_half2([b, a])
        } else {
            let packed = self.pack_half2([b, b]);
            let c = self.high_half_if(c, inst >> 72 & 1 != 0);
            let sixteen = self.cached_const_u32(16);
            self.ir.emit_bit_field_insert(self.type_u32[1], packed, c, sixteen, sixteen)
        };
        self.store_register((inst >> 16 & 0xff) as u32, word);
    }
    pub fn f2i(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let dstfmt = ((((inst >> 75) & 0x3) << 1) | (((inst >> 72) & 0x1) << 0)) as u32;
        let rnd = ((inst >> 78) & 3) as usize;
        let ftz = ((inst >> 80) & 1) != 0;
        let srcfmt = ((inst >> 84) & 3) as u32;
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
        let signed = inst >> 76 & 1 != 0;
        let relu = inst >> 75 & 1 != 0;
        let lo = if signed && !relu { -128.0f32 } else { 0.0 };
        let hi = if signed { 127.0f32 } else { 255.0 };
        let [lo, hi] = [lo, hi].map(|c| self.ir.emit_constant_typed(self.type_f32[1], c));
        let round = if inst >> 78 & 3 == 3 { spirv::glsl::TRUNC } else { spirv::glsl::ROUND_EVEN };
        let (b, c) = self.srcs_bc(inst);
        let a = self.load_register((inst >> 24 & 0xff) as u32);
        let [a, b] = [a, b].map(|x| {
            let x = self.bitcast_f32(x);
            let x = self.glsl(round, self.type_f32[1], &[x]);
            let x = self.glsl(81, self.type_f32[1], &[x, lo, hi]);
            let x = self.ir.emit_convert_f_to_s(self.type_s32[1], x);
            self.bitcast_u32(x)
        });
        let c = self.high_half_if(c, inst >> 72 & 1 != 0);
        let word = self.pack_narrow(a, b, c, 8);
        self.store_register((inst >> 16 & 0xff) as u32, word);
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
        let take_min = self.pp_set(inst);
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
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let rnd = ((inst >> 78) & 3) as usize;
        let ftz = ((inst >> 80) & 1) != 0;
        let fmt = (((inst >> 84) & 0x3) << 2) | (((inst >> 75) & 0x3) << 0);
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
        let rd = (inst >> 16 & 0xff) as u32;
        let ftz = inst >> 80 & 1 != 0;
        let a = self.fp_reg(inst, 24, 73, 72, ftz);
        let b = self.fp_alu_src(inst, ftz);
        let p = self.pp_set(inst);
        let result = self.ir.emit_select(self.type_f32[1], p, a, b);
        let result = self.bitcast_u32(result);
        self.store_register(rd, result);
    }
    pub fn fset(&mut self, inst: u128) {
        let rd = (inst >> 16 & 0xff) as u32;
        let ftz = inst >> 80 & 1 != 0;
        let a = self.fp_reg(inst, 24, 73, 72, ftz);
        let b = self.fp_alu_src(inst, ftz);
        let comparison = self.fp_compare((inst >> 76 & 0xf) as u32, a, b);
        let p = self.pp_bit(inst);
        let result = self.bool_op(inst >> 74 & 3, comparison, p);
        let mask = self.ir.emit_isub(self.type_u32[1], self.const_u32_0, result);
        self.store_register(rd, mask);
    }
    pub fn fsetp(&mut self, inst: u128) {
        let ftz = inst >> 80 & 1 != 0;
        let a = self.fp_reg(inst, 24, 73, 72, ftz);
        let b = self.fp_alu_src(inst, ftz);
        let comparison = self.fp_compare((inst >> 76 & 0xf) as u32, a, b);
        self.store_setp(inst, comparison);
    }
    pub fn fswzadd(&mut self, inst: u128) {
        let (ty, f32_ty, bool_ty) = (self.type_u32[1], self.type_f32[1], self.type_bool[1]);
        let rnd = inst >> 78 & 3;
        assert!(rnd == 0, "FSWZADD rounding mode {rnd} needs directed rounding, only RN is implemented");
        let ftz = inst >> 80 & 1 != 0;
        let [a, c] = [24, 64].map(|bit| {
            let bits = self.load_register((inst >> bit & 0xff) as u32);
            self.fp_operand(bits, false, false, ftz)
        });
        let control = self.cached_const_u32((inst >> 32 & 0xff) as u32);
        let [two, three] = [2, 3].map(|i| self.cached_const_u32(i));
        // Lane 0 of a quad takes its op from the top two bits of the control byte.
        let lane = self.special_register(0);
        let lane = self.ir.emit_bitwise_and(ty, lane, three);
        let lane = self.ir.emit_isub(ty, three, lane);
        let shift = self.ir.emit_imul(ty, lane, two);
        let op = self.ir.emit_bit_field_u_extract(ty, control, shift, two);
        let [sub, subr, mov2] = [1, 2, 3].map(|i| {
            let id = self.cached_const_u32(i);
            self.ir.emit_iequal(bool_ty, op, id)
        });
        let neg_a = self.ir.emit_fnegate(f32_ty, a);
        let a = self.ir.emit_select(f32_ty, subr, neg_a, a);
        let zero = self.ir.emit_constant_typed(f32_ty, 0.0f32);
        let a = self.ir.emit_select(f32_ty, mov2, zero, a);
        let neg_c = self.ir.emit_fnegate(f32_ty, c);
        let c = self.ir.emit_select(f32_ty, sub, neg_c, c);
        let sum = self.ir.emit_fadd(f32_ty, a, c);
        let bits = self.bitcast_u32(sum);
        let bits = if ftz { self.flush_subnormal(bits) } else { bits };
        self.store_register((inst >> 16 & 0xff) as u32, bits);
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
        let ftz = inst >> 80 & 1 != 0;
        let a = self.half2_reg(inst, 24, 74, 73, 72, ftz);
        let b = self.half2_alu_src(inst, ftz);
        let sum = [0, 1].map(|i| self.ir.emit_fadd(self.type_f32[1], a[i], b[i]));
        self.store_half2(inst, sum, inst >> 78 & 1 != 0, inst >> 77 & 1 != 0, ftz);
    }
    pub fn hfma2(&mut self, inst: u128) {
        let ofmt = (inst >> 85 & 1) << 1 | inst >> 78 & 1;
        assert!(ofmt < 2, "HFMA2.BF16_V2 needs a bfloat16 type");
        assert!(inst >> 86 & 1 == 0, "HFMA2 .H0_NH1 is not implemented");
        let fmz = inst >> 76 & 1 != 0;
        let ftz = fmz || inst >> 80 & 1 != 0;
        let a = self.half2_reg(inst, 24, 74, 73, 72, ftz);
        let src = self.half2_alu_src(inst, ftz);
        let rc = self.half2_reg(inst, 64, 81, 83, 84, ftz);
        let [b, c] = if matches!(inst >> 9 & 7, 2 | 3 | 7) { [rc, src] } else { [src, rc] };
        let zero = self.ir.emit_constant_typed(self.type_f32[1], 0.0f32);
        let val = [0, 1].map(|i| {
            let [a, b] = if fmz { self.fmz_operands(a[i], b[i]) } else { [a[i], b[i]] };
            let val = self.glsl(spirv::glsl::FMA, self.type_f32[1], &[a, b, c[i]]);
            if inst >> 79 & 1 == 0 {
                return val;
            }
            let negative = self.ir.emit_ford_less_than(self.type_bool[1], val, zero);
            self.ir.emit_select(self.type_f32[1], negative, zero, val)
        });
        self.store_half2(inst, val, ofmt == 1, inst >> 77 & 1 != 0, ftz);
    }
    pub fn hfma2_mma(&mut self, inst: u128) {
        self.hfma2(inst);
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
        let ofmt = (inst >> 85 & 1) << 1 | inst >> 78 & 1;
        assert!(ofmt < 2, "HMNMX2.BF16_V2 needs a bfloat16 type");
        assert!(inst >> 82 & 1 == 0, "HMNMX2.XORSIGN is not implemented");
        let ftz = inst >> 80 & 1 != 0;
        let a = self.half2_reg(inst, 24, 74, 73, 72, ftz);
        let b = self.half2_alu_src(inst, ftz);
        let take_min = self.pp_set(inst);
        let nan = self.ir.emit_constant_typed(self.type_f32[1], f32::NAN);
        let val = [0, 1].map(|i| {
            let [min, max] = [79, 80].map(|op| self.glsl(op, self.type_f32[1], &[a[i], b[i]]));
            let val = self.ir.emit_select(self.type_f32[1], take_min, min, max);
            if inst >> 81 & 1 == 0 {
                return val;
            }
            let [a_nan, b_nan] = [a[i], b[i]].map(|x| self.ir.emit_f_is_nan(self.type_bool[1], x));
            let either = self.ir.emit_logical_or(self.type_bool[1], a_nan, b_nan);
            self.ir.emit_select(self.type_f32[1], either, nan, val)
        });
        self.store_half2(inst, val, ofmt == 1, false, ftz);
    }
    pub fn hmul2(&mut self, inst: u128) {
        let fmz = inst >> 76 & 1 != 0;
        let ftz = fmz || inst >> 80 & 1 != 0;
        let a = self.half2_reg(inst, 24, 74, 73, 72, ftz);
        let b = self.half2_alu_src(inst, ftz);
        let product = [0, 1].map(|i| {
            let [a, b] = if fmz { self.fmz_operands(a[i], b[i]) } else { [a[i], b[i]] };
            self.ir.emit_fmul(self.type_f32[1], a, b)
        });
        self.store_half2(inst, product, inst >> 78 & 1 != 0, inst >> 77 & 1 != 0, ftz);
    }
    pub fn hset2(&mut self, inst: u128) {
        let [lo, hi] = self.half2_compare(inst, false);
        let sixteen = self.cached_const_u32(16);
        let lanes = self.ir.emit_bit_field_insert(self.type_u32[1], lo, hi, sixteen, sixteen);
        let truth = self.cached_const_u32(if inst >> 71 & 1 != 0 { 0x3c00 } else { 0xffff });
        let word = self.ir.emit_imul(self.type_u32[1], lanes, truth);
        self.store_register((inst >> 16 & 0xff) as u32, word);
    }
    pub fn hsetp2(&mut self, inst: u128) {
        let [pu, pv] = self.half2_compare(inst, inst >> 71 & 1 != 0);
        self.store_pred_bit(false, (inst >> 81 & 7) as u32, pu);
        self.store_pred_bit(false, (inst >> 84 & 7) as u32, pv);
    }
    pub fn i2f(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let extract = ((inst >> 60) & 3) as u32;
        let size = ((inst >> 75) & 3) as u32;
        let rnd = ((inst >> 78) & 3) as u32;
        let srcfmt = ((((inst >> 84) & 0x3) << 1) | (((inst >> 74) & 0x1) << 0)) as u32;
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
        assert!(inst >> 78 & 3 == 0, "I2FP.RZ is not implemented");
        let b = self.alu_src(inst);
        let val = if inst >> 74 & 1 != 0 {
            let b = self.ir.emit_bitcast(self.type_s32[1], b);
            self.ir.emit_convert_s_to_f(self.type_f32[1], b)
        } else {
            self.ir.emit_convert_u_to_f(self.type_f32[1], b)
        };
        let bits = self.bitcast_u32(val);
        self.store_register((inst >> 16 & 0xff) as u32, bits);
    }
    pub fn i2i(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let dstfmt = ((inst >> 76) & 3) as u32;
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
        let dstfmt = (inst >> 76 & 7) as usize;
        let bits = [8, 8, 16, 16, 2, 2, 4, 4][dstfmt];
        let signed = dstfmt & 1 != 0;
        let (b, c) = self.srcs_bc(inst);
        let a = self.load_register((inst >> 24 & 0xff) as u32);
        // Bits 75:74 are .SAT (0), plain truncation (1) or .SATRELU (2).
        let satrelu = inst >> 74 & 3;
        let [a, b] = if satrelu == 1 {
            [a, b]
        } else {
            let lo = if signed && satrelu == 0 { -(1 << (bits - 1)) } else { 0 };
            let hi = if signed { (1 << (bits - 1)) - 1 } else { (1 << bits) - 1 };
            let [lo, hi] = [lo, hi].map(|c: i32| self.cached_const_u32(c as u32));
            [a, b].map(|x| self.glsl(spirv::glsl::S_CLAMP, self.type_u32[1], &[x, lo, hi]))
        };
        let c = self.high_half_if(c, inst >> 72 & 3 == 1);
        let word = self.pack_narrow(a, b, c, bits);
        self.store_register((inst >> 16 & 0xff) as u32, word);
    }
    pub fn iabs(&mut self, inst: u128) {
        let value = self.alu_src(inst);
        let shift = self.cached_const_u32(31);
        let sign = self.ir.emit_shift_right_arithmetic(self.type_u32[1], value, shift);
        let value = self.ir.emit_bitwise_xor(self.type_u32[1], value, sign);
        let value = self.ir.emit_isub(self.type_u32[1], value, sign);
        self.store_register((inst >> 16 & 0xff) as u32, value);
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
        let ty = self.type_u32[1];
        let a = self.load_register((inst >> 24 & 0xff) as u32);
        let b = self.alu_src(inst);
        let c = self.load_register((inst >> 64 & 0xff) as u32);
        let mut result = self.int_src(c, inst >> 75 & 1 != 0);
        let mode = inst >> 76 & 3;
        let lanes = if mode == 0 { 4 } else { 2 };
        let a_bits = if mode == 0 { 8 } else { 16 };
        let a_count = self.cached_const_u32(a_bits);
        let b_count = self.cached_const_u32(8);
        for i in 0..lanes {
            let a_offset = self.cached_const_u32(i * a_bits);
            let b_offset = self.cached_const_u32((i + if mode == 3 { 2 } else { 0 }) * 8);
            let a = if inst >> 73 & 1 != 0 {
                self.ir.emit_bit_field_s_extract(ty, a, a_offset, a_count)
            } else {
                self.ir.emit_bit_field_u_extract(ty, a, a_offset, a_count)
            };
            let b = if inst >> 74 & 1 != 0 {
                self.ir.emit_bit_field_s_extract(ty, b, b_offset, b_count)
            } else {
                self.ir.emit_bit_field_u_extract(ty, b, b_offset, b_count)
            };
            let product = self.ir.emit_imul(ty, a, b);
            result = self.ir.emit_iadd(ty, result, product);
        }
        self.store_register((inst >> 16 & 0xff) as u32, result);
    }
    pub fn imad(&mut self, inst: u128) {
        let signed = inst >> 73 & 1 != 0;
        let x = inst >> 74 & 1 != 0;
        let form = (inst >> 9) & 7;
        let a = self.alu_reg(inst, 24);
        let (b, c32) = self.srcs_bc(inst);
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
        let carry = if x { self.pp_bit(inst) } else { self.const_u32_0 };
        let mode = inst as u32 & 7;
        if mode == 5 || mode == 7 {
            let ty = self.type_u64();
            let [a, b] = [a, b].map(|x| if signed { self.ir.emit_s_convert(ty, x) } else { self.ir.emit_u_convert(ty, x) });
            let product = self.ir.emit_imul(ty, a, b);
            if mode == 7 {
                let shift = self.ir.emit_constant_typed(ty, 32u64);
                let hi = self.ir.emit_shift_right_logical(ty, product, shift);
                let hi = self.ir.emit_u_convert(self.type_u32[1], hi);
                let result = self.ir.emit_iadd(self.type_u32[1], hi, c32);
                let result = self.ir.emit_iadd(self.type_u32[1], result, carry);
                return self.store_alu(inst, result);
            }
            let c = match form {
                2 => self.ir.emit_s_convert(ty, c32),
                3 | 7 => self.alu_src_pair(inst),
                _ => self.alu_pair(inst, 64),
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
            let result = self.ir.emit_iadd(ty, product, c);
            let result = self.ir.emit_iadd(ty, result, carry);
            self.store_alu_pair(inst, result);
        } else {
            let product = self.ir.emit_imul(self.type_u32[1], a, b);
            let result = self.ir.emit_iadd(self.type_u32[1], product, c32);
            let result = self.ir.emit_iadd(self.type_u32[1], result, carry);
            self.store_alu(inst, result);
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
        let a = self.load_register((inst >> 24 & 0xff) as u32);
        let b = self.alu_src(inst);
        let signed = inst >> 73 & 1 != 0;
        let less = if signed {
            self.ir.emit_sless_than(self.type_bool[1], a, b)
        } else {
            self.ir.emit_uless_than(self.type_bool[1], a, b)
        };
        let min = self.ir.emit_select(self.type_u32[1], less, a, b);
        let max = self.ir.emit_select(self.type_u32[1], less, b, a);
        let take_min = self.pp_set(inst);
        let result = self.ir.emit_select(self.type_u32[1], take_min, min, max);
        self.store_register((inst >> 16 & 0xff) as u32, result);
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
        let signed = inst >> 73 & 1 != 0;
        let cmp = (inst >> 76 & 7) as u32;
        let (a, b) = (self.alu_reg(inst, 24), self.src_b(inst));
        // The compare field is a mask of LT (1), EQ (2) and GT (4).
        let compare = |this: &mut Self, cmp| {
            let ty = this.type_bool[1];
            let cond = match (cmp, signed) {
                (0, _) => return this.const_u32_0,
                (7, _) => return this.const_u32_1,
                (1, false) => this.ir.emit_uless_than(ty, a, b),
                (1, true) => this.ir.emit_sless_than(ty, a, b),
                (2, _) => this.ir.emit_iequal(ty, a, b),
                (3, false) => this.ir.emit_uless_than_equal(ty, a, b),
                (3, true) => this.ir.emit_sless_than_equal(ty, a, b),
                (4, false) => this.ir.emit_ugreater_than(ty, a, b),
                (4, true) => this.ir.emit_sgreater_than(ty, a, b),
                (5, _) => this.ir.emit_inot_equal(ty, a, b),
                (6, false) => this.ir.emit_ugreater_than_equal(ty, a, b),
                _ => this.ir.emit_sgreater_than_equal(ty, a, b),
            };
            this.bool_bit(cond)
        };
        let mut result = compare(self, cmp);
        // .EX compares the high words of a wider value, so a tie takes Pr, the low words' result.
        if inst >> 72 & 1 != 0 && !matches!(cmp, 0 | 7) {
            let pr = self.pred_bit(is_uniform(inst), (inst >> 68 & 7) as u32, inst >> 71 & 1 != 0);
            let strict = compare(self, cmp & 5);
            let equal = compare(self, 2);
            let tied = self.ir.emit_bitwise_and(self.type_u32[1], equal, pr);
            result = self.ir.emit_bitwise_or(self.type_u32[1], strict, tied);
        }
        self.store_setp(inst, result);
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
        let pp = (((inst >> 87) & 0x7) << 0) as u32;
        let pp_not = (((inst >> 90) & 0x1) << 0);
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
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let address = self.global_address(inst, 32, 32);
        self.load_memory(ResourceKind::Global, address, sz, rd);
    }
    pub fn ldc(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let ra = (((inst >> 24) & 0xff) << 0) as u32;
        let ra_offset = (((inst >> 38) & 0xffff) << 0) as u16 as i16 as u32;
        let sc_bank = (((inst >> 54) & 0x1f) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let stride = (((inst >> 78) & 0x3) << 0) as u32;
        let bindless = opcode(inst) == 0x1582;
        let base = self.load_register(if bindless { (inst >> 64) as u32 & 0xff } else { ra });
        let mut bank = if bindless { self.load_ureg(ra & 0x3f) } else { self.cached_const_u32(sc_bank) };
        let base = if stride >= 2 {
            let sixteen = self.cached_const_u32(16);
            let segment = self.ir.emit_shift_right_logical(self.type_u32[1], base, sixteen);
            bank = self.ir.emit_iadd(self.type_u32[1], bank, segment);
            let mask = self.cached_const_u32(0xffff);
            self.ir.emit_bitwise_and(self.type_u32[1], base, mask)
        } else {
            base
        };
        for i in 0..access_words(sz) {
            let offset = self.cached_const_u32(ra_offset.wrapping_add(4 * i));
            let addr = self.ir.emit_iadd(self.type_u32[1], base, offset);
            let word = if bindless || stride >= 2 {
                self.load_dynamic_cbuf(bank, addr)
            } else {
                self.load_cbuf(ResourceKind::Cbuf(sc_bank), addr)
            };
            let value = if sz <= 4 { self.load_narrow(word, addr, sz) } else { word };
            self.store_register(nth_reg(rd, RZ, i), value);
        }
    }
    pub fn ldg(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let address = self.global_address(inst, 32, 40);
        self.load_memory(ResourceKind::Global, address, sz, rd);
    }
    pub fn ldgdepbar(&mut self, _inst: u128) {
        // LDGSTS completes synchronously in this model.
    }
    pub fn ldgsts(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let ra = (((inst >> 24) & 0xff) << 0) as u32;
        let ra_offset = (((inst >> 32) as i32) << 20 >> 20) as u32;
        let rb_offset = (((inst >> 44) as i32) << 12 >> 12) as u32;
        let ra_urc = (((inst >> 64) & 0x3f) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let [ra_offset, rb_offset] = [ra_offset, rb_offset].map(|v| self.cached_const_u32(v));
        let mut global = self.load_register(ra);
        let mut shared = self.load_register(rd);
        match opcode(inst) {
            0x1fae => {
                let ur = self.load_ureg(ra_urc);
                global = self.ir.emit_iadd(self.type_u32[1], global, ur);
            }
            0x1dae => {
                let ur = self.load_ureg(ra_urc);
                shared = self.ir.emit_iadd(self.type_u32[1], shared, ur);
            }
            _ => unreachable!(),
        }
        global = self.ir.emit_iadd(self.type_u32[1], global, ra_offset);
        shared = self.ir.emit_iadd(self.type_u32[1], shared, rb_offset);
        for i in 0..access_words(sz) {
            let offset = self.cached_const_u32(4 * i);
            let global = self.ir.emit_iadd(self.type_u32[1], global, offset);
            let shared = self.ir.emit_iadd(self.type_u32[1], shared, offset);
            let src = self.global_word_at(global);
            let value = self.ir.emit_load(self.type_u32[1], src);
            let dst = self.shared_word_at(shared);
            self.ir.emit_store(dst, value);
        }
    }
    pub fn ldl(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let address = self.global_address(inst, 32, 40);
        self.load_memory(ResourceKind::Local, address, sz, rd);
    }
    pub fn lds(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let address = self.shared_address(inst, 32);
        self.load_memory(ResourceKind::Shared, address, sz, rd);
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
        let negate = inst >> 72 & 1 != 0;
        let sx32 = inst >> 73 & 1 != 0;
        let x = inst >> 74 & 1 != 0;
        let scale = (((inst >> 75) & 0x1f) << 0) as u32;
        let high = inst >> 80 & 1 != 0;
        let ty = self.type_u32[1];
        let shift = self.cached_const_u32(scale);
        let a = self.alu_reg(inst, 24);
        let a = if high {
            let ty64 = self.type_u64();
            let a = if sx32 {
                self.ir.emit_s_convert(ty64, a)
            } else {
                let hi = self.alu_reg(inst, 64);
                self.join_u64(a, hi)
            };
            let shift = self.ir.emit_constant_typed(ty64, (32 - scale) as u64);
            let a = self.ir.emit_shift_right_logical(ty64, a, shift);
            self.ir.emit_u_convert(ty, a)
        } else {
            self.ir.emit_shift_left_logical(ty, a, shift)
        };
        let a = if x && negate { self.ir.emit_not(ty, a) } else { self.int_src(a, negate) };
        let b = self.src_b(inst);
        let negate_b = inst >> 63 & 1 != 0 && !matches!(inst >> 9 & 7, 2 | 4);
        let b = if x && negate_b { self.ir.emit_not(ty, b) } else { self.int_src(b, negate_b) };
        let ab = self.ir.emit_iadd(ty, a, b);
        let mut carry = self.ir.emit_uless_than(self.type_bool[1], ab, a);
        if !x && negate {
            let overflow = self.ir.emit_iequal(self.type_bool[1], a, self.const_u32_0);
            carry = self.ir.emit_logical_or(self.type_bool[1], carry, overflow);
        }
        if !x && negate_b {
            let overflow = self.ir.emit_iequal(self.type_bool[1], b, self.const_u32_0);
            carry = self.ir.emit_logical_or(self.type_bool[1], carry, overflow);
        }
        let result = if x {
            let cin = self.pp_bit(inst);
            let result = self.ir.emit_iadd(ty, ab, cin);
            let cin_carry = self.ir.emit_uless_than(self.type_bool[1], result, ab);
            carry = self.ir.emit_logical_or(self.type_bool[1], carry, cin_carry);
            result
        } else {
            ab
        };
        let carry = self.bool_bit(carry);
        self.store_pred_bit(is_uniform(inst), (inst >> 81 & 7) as u32, carry);
        self.store_alu(inst, result);
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
        let p = self.pp_bit(inst);
        let nonzero = self.ir.emit_inot_equal(self.type_bool[1], res, self.const_u32_0);
        let nonzero = self.bool_bit(nonzero);
        let bit = if inst >> 80 & 1 != 0 { self.ir.emit_bitwise_and(ty, nonzero, p) } else { self.ir.emit_bitwise_or(ty, nonzero, p) };
        self.store_pred_bit(is_uniform(inst), (inst >> 81 & 7) as u32, bit);
    }
    pub fn match_(&mut self, inst: u128) {
        let ra = (inst >> 24 & 0xff) as u32;
        let words = if inst >> 73 & 1 != 0 { vec![ra, nth_reg(ra, RZ, 1)] } else { vec![ra] };
        let mut same = self.cached_const_u32(u32::MAX);
        for reg in words {
            let own = self.load_register(reg);
            let bits = self.warp_values(own).into_iter().map(|v| {
                let eq = self.ir.emit_iequal(self.type_bool[1], v, own);
                self.bool_bit(eq)
            }).collect();
            let mask = self.pack_lanes(bits);
            same = self.ir.emit_bitwise_and(self.type_u32[1], same, mask);
        }
        let rd = (inst >> 16 & 0xff) as u32;
        if inst >> 79 & 1 != 0 {
            return self.store_register(rd, same);
        }
        let all = self.cached_const_u32(u32::MAX);
        let uniform = self.ir.emit_iequal(self.type_bool[1], same, all);
        let mask = self.ir.emit_select(self.type_u32[1], uniform, all, self.const_u32_0);
        self.store_register(rd, mask);
        let bit = self.bool_bit(uniform);
        self.store_pred_bit(false, (inst >> 81 & 7) as u32, bit);
    }
    pub fn membar(&mut self, inst: u128) {
        let dstfmt = (((inst >> 76) & 0x7) << 0) as u32;
        use spirv::memory_semantics::{ACQUIRE_RELEASE, UNIFORM_MEMORY, WORKGROUP_MEMORY};
        let (scope, semantics) = match dstfmt {
            0 | 1 => (spirv::scope::WORKGROUP, ACQUIRE_RELEASE | WORKGROUP_MEMORY),
            _ => (spirv::scope::DEVICE, ACQUIRE_RELEASE | UNIFORM_MEMORY | WORKGROUP_MEMORY),
        };
        let scope = self.cached_const_u32(scope);
        let semantics = self.cached_const_u32(semantics);
        self.ir.emit_memory_barrier(scope, semantics);
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
    pub fn nop(&mut self, _inst: u128) {
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
        let byte = (inst >> 76 & 3) as u32 * 8;
        let shift = self.cached_const_u32(byte);
        let mask = self.alu_src(inst);
        let predicate_mask = self.cached_const_u32(0x7f);
        let mask = self.ir.emit_bitwise_and(self.type_u32[1], mask, predicate_mask);
        let predicate = self.load_predicate();
        let selected = self.ir.emit_bitwise_and(self.type_u32[1], predicate, mask);
        let selected = self.ir.emit_shift_left_logical(self.type_u32[1], selected, shift);
        let mask = self.ir.emit_shift_left_logical(self.type_u32[1], mask, shift);
        let base = self.load_register((inst >> 24 & 0xff) as u32);
        let inverse = self.ir.emit_not(self.type_u32[1], mask);
        let keep = self.ir.emit_bitwise_and(self.type_u32[1], base, inverse);
        let result = self.ir.emit_bitwise_or(self.type_u32[1], keep, selected);
        self.store_register((inst >> 16 & 0xff) as u32, result);
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
        let uniform = inst & 0xfff == 0x89c;
        let p = self.pred_bit(uniform, (inst >> 87 & 7) as u32, inst >> 90 & 1 != 0);
        let q = self.pred_bit(uniform, (inst >> 77 & 7) as u32, inst >> 80 & 1 != 0);
        let r = self.pred_bit(uniform || inst >> 67 & 1 != 0, (inst >> 68 & 7) as u32, inst >> 71 & 1 != 0);
        let two = self.cached_const_u32(2);
        let p = self.ir.emit_shift_left_logical(self.type_u32[1], p, two);
        let q = self.ir.emit_shift_left_logical(self.type_u32[1], q, self.const_u32_1);
        let index = self.ir.emit_bitwise_or(self.type_u32[1], p, q);
        let index = self.ir.emit_bitwise_or(self.type_u32[1], index, r);
        let eval = |this: &mut Self, lut: u32| {
            let lut = this.cached_const_u32(lut);
            let value = this.ir.emit_shift_right_logical(this.type_u32[1], lut, index);
            this.ir.emit_bitwise_and(this.type_u32[1], value, this.const_u32_1)
        };
        let lut = ((inst >> 72 & 0x1f) << 3 | (inst >> 64 & 7)) as u32;
        let result = eval(self, lut);
        let second = eval(self, (inst >> 16 & 0xff) as u32);
        self.store_pred_bit(uniform, (inst >> 81 & 7) as u32, result);
        self.store_pred_bit(uniform, (inst >> 84 & 7) as u32, second);
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
        let bytes = self.join_u64(a, b);
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
        let byte = (inst >> 76 & 3) as u32 * 8;
        let value = self.load_register((inst >> 24 & 0xff) as u32);
        let shift = self.cached_const_u32(byte);
        let value = self.ir.emit_shift_right_logical(self.type_u32[1], value, shift);
        let mask = self.alu_src(inst);
        let predicate_mask = self.cached_const_u32(0x7f);
        let mask = self.ir.emit_bitwise_and(self.type_u32[1], mask, predicate_mask);
        let old = self.load_predicate();
        let inverse = self.ir.emit_not(self.type_u32[1], mask);
        let keep = self.ir.emit_bitwise_and(self.type_u32[1], old, inverse);
        let insert = self.ir.emit_bitwise_and(self.type_u32[1], value, mask);
        let result = self.ir.emit_bitwise_or(self.type_u32[1], keep, insert);
        self.store_predicate(result);
    }
    pub fn r2ur(&mut self, inst: u128) {
        assert!(inst >> 81 & 7 == 7, "R2UR with a predicate destination");
        // One invocation is the whole warp, so the value is uniform as is and .OR has nothing to combine.
        let value = self.load_register((inst >> 24 & 0xff) as u32);
        self.store_ureg((inst >> 16 & 0x3f) as u32, value);
    }
    pub fn red(&mut self, inst: u128) {
        let rb = (((inst >> 32) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0xf) << 0) as u32;
        let pp = (((inst >> 87) & 0x7) << 0) as u32;
        self.global_atomic(inst, pp, sz, rb);
    }
    pub fn redux(&mut self, inst: u128) {
        let ty = self.type_u32[1];
        let a = self.load_register((inst >> 24 & 0xff) as u32);
        let signed = inst >> 73 & 1 != 0;
        let lanes = self.warp_values(a);
        let mut acc = lanes[0];
        for &v in &lanes[1..] {
            acc = match inst >> 78 & 7 {
                op @ 0..=2 => self.bool_op(op, acc, v),
                3 => self.ir.emit_iadd(ty, acc, v),
                4 => self.glsl(if signed { spirv::glsl::S_MIN } else { spirv::glsl::U_MIN }, ty, &[acc, v]),
                5 => self.glsl(if signed { spirv::glsl::S_MAX } else { spirv::glsl::U_MAX }, ty, &[acc, v]),
                op => panic!("REDUX op {op} is invalid"),
            };
        }
        self.store_ureg((inst >> 16 & 0x3f) as u32, acc);
    }
    pub fn ret(&mut self, inst: u128) {
        let pp = (((inst >> 87) & 0x7) << 0) as u32;
        let pp_not = (((inst >> 90) & 0x1) << 0);
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
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let sr = (((inst >> 72) & 0xff) << 0) as u32;
        let val = self.special_register(sr);
        self.store_register(rd, val);
    }
    pub fn s2ur(&mut self, inst: u128) {
        let urd = (((inst >> 16) & 0x3f) << 0) as u32;
        let sr = (((inst >> 72) & 0xff) << 0) as u32;
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
        let p = self.pp_set(inst);
        let (a, b) = (self.alu_reg(inst, 24), self.src_b(inst));
        let result = self.ir.emit_select(self.type_u32[1], p, a, b);
        self.store_alu(inst, result);
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
        let mut x = self.join_u64(low, high);
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
        let (ty, bool_ty) = (self.type_u32[1], self.type_bool[1]);
        let b = if matches!(inst >> 9 & 7, 4 | 7) { self.cached_const_u32((inst >> 53 & 0x1f) as u32) } else { self.load_register((inst >> 32 & 0xff) as u32) };
        let c = if matches!(inst >> 9 & 7, 2 | 7) { self.cached_const_u32((inst >> 40 & 0x1fff) as u32) } else { self.load_register((inst >> 64 & 0xff) as u32) };
        let a = self.load_register((inst >> 24 & 0xff) as u32);
        let [five, eight, lane_bits] = [5, 8, 31].map(|i| self.cached_const_u32(i));
        let lane = self.special_register(0);
        let b = self.ir.emit_bitwise_and(ty, b, lane_bits);
        let clamp = self.ir.emit_bitwise_and(ty, c, lane_bits);
        let segment = self.ir.emit_bit_field_u_extract(ty, c, eight, five);
        let outside = self.ir.emit_not(ty, segment);
        let min_lane = self.ir.emit_bitwise_and(ty, lane, segment);
        let clamp = self.ir.emit_bitwise_and(ty, clamp, outside);
        let max_lane = self.ir.emit_bitwise_or(ty, min_lane, clamp);
        let (src, in_range) = match inst >> 58 & 3 {
            0 => {
                let offset = self.ir.emit_bitwise_and(ty, b, outside);
                let src = self.ir.emit_bitwise_or(ty, min_lane, offset);
                (src, self.ir.emit_uless_than_equal(bool_ty, src, max_lane))
            }
            1 => {
                let src = self.ir.emit_isub(ty, lane, b);
                (src, self.ir.emit_sgreater_than_equal(bool_ty, src, max_lane))
            }
            mode => {
                let src = if mode == 2 { self.ir.emit_iadd(ty, lane, b) } else { self.ir.emit_bitwise_xor(ty, lane, b) };
                (src, self.ir.emit_uless_than_equal(bool_ty, src, max_lane))
            }
        };
        let src = self.ir.emit_select(ty, in_range, src, lane);
        let val = self.warp_exchange(a, &[src])[0];
        self.store_register((inst >> 16 & 0xff) as u32, val);
        let bit = self.bool_bit(in_range);
        self.store_pred_bit(false, (inst >> 81 & 7) as u32, bit);
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
        let rc = (((inst >> 64) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let address = self.global_address(inst, 64, 32);
        self.store_memory(ResourceKind::Global, address, sz, rc);
    }
    pub fn stg(&mut self, inst: u128) {
        let rb = (((inst >> 32) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let address = self.global_address(inst, 64, 40);
        self.store_memory(ResourceKind::Global, address, sz, rb);
    }
    pub fn stl(&mut self, inst: u128) {
        let rb = (((inst >> 32) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let address = self.global_address(inst, 64, 40);
        self.store_memory(ResourceKind::Local, address, sz, rb);
    }
    pub fn sts(&mut self, inst: u128) {
        let rb = (((inst >> 32) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let address = self.shared_address(inst, 64);
        self.store_memory(ResourceKind::Shared, address, sz, rb);
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
        self.iadd3(inst)
    }
    pub fn iadd3(&mut self, inst: u128) {
        let wide = inst & 7 == 7;
        let ty = if wide { self.type_u64() } else { self.type_u32[1] };
        let srcs = if wide {
            let b = if inst >> 9 & 7 == 4 {
                let imm = self.cached_const_u32((inst >> 32) as u32);
                self.ir.emit_s_convert(ty, imm)
            } else {
                self.alu_pair(inst, 32)
            };
            [self.alu_pair(inst, 24), b, self.alu_pair(inst, 64)]
        } else {
            let (b, c) = self.srcs_bc(inst);
            [self.alu_reg(inst, 24), b, c]
        };
        let negated = [inst >> 72 & 1 != 0, inst >> 9 & 7 == 1 && inst >> 63 & 1 != 0, inst >> 75 & 1 != 0];
        let [a, b, c] = [0, 1, 2].map(|i| if negated[i] { self.ir.emit_not(ty, srcs[i]) } else { srcs[i] });
        // -x is ~x + 1, so without .X every negated operand adds one; .X adds the carry predicates instead.
        let carry_in = if inst >> 74 & 1 != 0 {
            let p = self.pp_bit(inst);
            let q = self.pred_bit(is_uniform(inst), (inst >> 77 & 7) as u32, inst >> 80 & 1 != 0);
            self.ir.emit_iadd(self.type_u32[1], p, q)
        } else {
            self.cached_const_u32(negated.iter().filter(|&&n| n).count() as u32)
        };
        let carry_in = if wide { self.ir.emit_u_convert(ty, carry_in) } else { carry_in };
        let (mut sum, mut carries) = (a, self.const_u32_0);
        for addend in [b, c, carry_in] {
            let next = self.ir.emit_iadd(ty, sum, addend);
            let wrapped = self.ir.emit_uless_than(self.type_bool[1], next, sum);
            let wrapped = self.bool_bit(wrapped);
            carries = self.ir.emit_iadd(self.type_u32[1], carries, wrapped);
            sum = next;
        }
        if wide { self.store_alu_pair(inst, sum) } else { self.store_alu(inst, sum) }
        for (pred, carried) in [(inst >> 81 & 7, 0), (inst >> 84 & 7, 1)] {
            let carried = self.cached_const_u32(carried);
            let set = self.ir.emit_ugreater_than(self.type_bool[1], carries, carried);
            let set = self.bool_bit(set);
            self.store_pred_bit(is_uniform(inst), pred as u32, set);
        }
    }
    pub fn uiadd3_64(&mut self, inst: u128) {
        self.uiadd3(inst)
    }
    pub fn uimad(&mut self, inst: u128) {
        self.imad(inst)
    }
    pub fn uisetp(&mut self, inst: u128) {
        self.isetp(inst)
    }
    pub fn uldc(&mut self, inst: u128) {
        let urd = (((inst >> 16) & 0x3f) << 0) as u32;
        let ra_offset = (((inst >> 38) & 0xffff) << 0) as u32;
        let sc_bank = (((inst >> 54) & 0x1f) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let op = opcode(inst);
        let ura = (inst >> 24 & 0x3f) as u32;
        for i in 0..access_words(sz) {
            let offset = match op {
                0x18b8 => (inst >> 38) as u32,
                _ => ra_offset,
            };
            let offset = self.cached_const_u32(offset.wrapping_add(4 * i));
            let base = match op {
                0x18b8 | 0x1abb => self.load_ureg(ura),
                _ => self.const_u32_0,
            };
            let address = self.ir.emit_iadd(self.type_u32[1], base, offset);
            let word = match op {
                0xab9 | 0x1abb => self.load_cbuf(ResourceKind::Cbuf(sc_bank), address),
                0x1ab9 => {
                    let bank = self.load_ureg(ura);
                    self.load_dynamic_cbuf(bank, address)
                }
                0x18b8 => {
                    let ptr = self.global_word_at(address);
                    self.ir.emit_load(self.type_u32[1], ptr)
                }
                _ => unreachable!(),
            };
            let value = if sz <= 4 { self.load_narrow(word, address, sz) } else { word };
            self.store_ureg(nth_reg(urd, URZ, i), value);
        }
    }
    pub fn ulea(&mut self, inst: u128) {
        self.lea(inst)
    }
    pub fn ulop3(&mut self, inst: u128) {
        self.lop3(inst)
    }
    pub fn umov(&mut self, inst: u128) {
        let value = if inst >> 9 & 7 == 4 {
            self.cached_const_u32((inst >> 32) as u32)
        } else {
            self.load_ureg((inst >> 32 & 0x3f) as u32)
        };
        self.store_ureg((inst >> 16 & 0x3f) as u32, value);
    }
    pub fn up2ur(&mut self, inst: u128) {
        let ty = self.type_u32[1];
        let shift = self.cached_const_u32((inst >> 76 & 3) as u32 * 8);
        let mask = self.up_mask(inst);
        let selected = self.load_upredicate();
        let selected = self.ir.emit_bitwise_and(ty, selected, mask);
        let selected = self.ir.emit_shift_left_logical(ty, selected, shift);
        let mask = self.ir.emit_shift_left_logical(ty, mask, shift);
        let mask = self.ir.emit_not(ty, mask);
        let base = self.load_ureg((inst >> 24 & 0x3f) as u32);
        let kept = self.ir.emit_bitwise_and(ty, base, mask);
        let result = self.ir.emit_bitwise_or(ty, kept, selected);
        self.store_ureg((inst >> 16 & 0x3f) as u32, result);
    }
    pub fn uplop3(&mut self, inst: u128) {
        assert!(inst & 0xfff == 0x89c, "UPLOP3 with register sources");
        self.plop3(inst)
    }
    pub fn upopc(&mut self, inst: u128) {
        self.popc(inst)
    }
    pub fn uprmt(&mut self, inst: u128) {
        self.prmt(inst)
    }
    pub fn ur2up(&mut self, inst: u128) {
        let ty = self.type_u32[1];
        let shift = self.cached_const_u32((inst >> 76 & 3) as u32 * 8);
        let mask = self.up_mask(inst);
        let value = self.load_ureg((inst >> 24 & 0x3f) as u32);
        let value = self.ir.emit_shift_right_logical(ty, value, shift);
        let value = self.ir.emit_bitwise_and(ty, value, mask);
        let mask = self.ir.emit_not(ty, mask);
        let kept = self.load_upredicate();
        let kept = self.ir.emit_bitwise_and(ty, kept, mask);
        let result = self.ir.emit_bitwise_or(ty, kept, value);
        self.store_upredicate(result);
    }
    pub fn usel(&mut self, inst: u128) {
        self.sel(inst)
    }
    pub fn usgxt(&mut self, inst: u128) {
        self.sgxt(inst)
    }
    pub fn ushf(&mut self, inst: u128) {
        self.shf(inst)
    }
    pub fn vabsdiff(&mut self, inst: u128) {
        let ty = self.type_u32[1];
        let a = self.load_register((inst >> 24 & 0xff) as u32);
        let (b, c) = self.srcs_bc(inst);
        let below = if inst >> 73 & 1 != 0 {
            self.ir.emit_sless_than(self.type_bool[1], a, b)
        } else {
            self.ir.emit_uless_than(self.type_bool[1], a, b)
        };
        let ab = self.ir.emit_isub(ty, a, b);
        let ba = self.ir.emit_isub(ty, b, a);
        let difference = self.ir.emit_select(ty, below, ba, ab);
        let result = self.ir.emit_iadd(ty, difference, c);
        self.store_register((inst >> 16 & 0xff) as u32, result);
    }
    pub fn vabsdiff4(&mut self, inst: u128) {
        let ty = self.type_u32[1];
        let a = self.load_register((inst >> 24 & 0xff) as u32);
        let (b, c) = self.srcs_bc(inst);
        let count = self.cached_const_u32(8);
        let mut result = if inst >> 75 & 1 != 0 { c } else { self.const_u32_0 };
        for i in 0..4 {
            let offset = self.cached_const_u32(i * 8);
            let (a, b) = if inst >> 73 & 1 != 0 {
                (
                    self.ir.emit_bit_field_s_extract(ty, a, offset, count),
                    self.ir.emit_bit_field_s_extract(ty, b, offset, count),
                )
            } else {
                (
                    self.ir.emit_bit_field_u_extract(ty, a, offset, count),
                    self.ir.emit_bit_field_u_extract(ty, b, offset, count),
                )
            };
            let below = self.ir.emit_sless_than(self.type_bool[1], a, b);
            let ab = self.ir.emit_isub(ty, a, b);
            let ba = self.ir.emit_isub(ty, b, a);
            let difference = self.ir.emit_select(ty, below, ba, ab);
            if inst >> 75 & 1 != 0 {
                result = self.ir.emit_iadd(ty, result, difference);
            } else {
                result = self.ir.emit_bit_field_insert(ty, result, difference, offset, count);
            }
        }
        self.store_register((inst >> 16 & 0xff) as u32, result);
    }
    pub fn vote(&mut self, inst: u128) {
        let (ballot, result) = self.vote_ballot(inst);
        self.store_register((inst >> 16 & 0xff) as u32, ballot);
        self.store_pred_bit(false, (inst >> 81 & 7) as u32, result);
    }
    pub fn voteu(&mut self, inst: u128) {
        let (ballot, result) = self.vote_ballot(inst);
        self.store_ureg((inst >> 16 & 0x3f) as u32, ballot);
        self.store_pred_bit(true, (inst >> 81 & 7) as u32, result);
    }
    fn vote_ballot(&mut self, inst: u128) -> (u32, u32) {
        let p = self.pp_bit(inst);
        let bits = self.warp_values(p);
        let ballot = self.pack_lanes(bits);
        let bool_ty = self.type_bool[1];
        let all = self.cached_const_u32(u32::MAX);
        let every = self.ir.emit_iequal(bool_ty, ballot, all);
        let none = self.ir.emit_iequal(bool_ty, ballot, self.const_u32_0);
        let result = match inst >> 72 & 3 {
            0 => every,
            1 => self.ir.emit_logical_not(bool_ty, none),
            2 => self.ir.emit_logical_or(bool_ty, every, none),
            _ => panic!("VOTE mode 3 is invalid"),
        };
        (ballot, self.bool_bit(result))
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

#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::run_body;

    #[test]
    fn half2_lanes_round_trip_through_a_register() {
        if !crate::gpu::device().is_ok_and(|d| d.shader_float16) {
            return eprintln!("skipping: no shaderFloat16");
        }
        let program = |d: &mut super::Decoder| {
            let half2 = d.load_register(0);
            let half2 = d.unpack_half2(half2);
            let ty = d.type_f16x2();
            let sum = d.ir.emit_fadd(ty, half2, half2);
            let packed = d.bitcast_u32(sum);
            d.store_register(1, packed);
            let hi = d.half2_swizzle(sum, 3);
            let hi = d.bitcast_u32(hi);
            d.store_register(2, hi);
        };
        let Some((lanes, _)) = run_body(program, &[vec![0xC000_3E00]], &[], &[]) else {
            return;
        };
        assert_eq!(lanes[0][1..3], [0xC400_4200, 0xC400_C400]);
    }
}
