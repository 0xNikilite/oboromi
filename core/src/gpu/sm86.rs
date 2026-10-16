#![allow(unused_parens)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use std::collections::HashMap;
use crate::gpu::spirv::{self, storage_class};

const MAX_REG_COUNT: usize = 254;
const MAX_UNIFORM_REG_COUNT: usize = 63;
pub const STATE_WORDS: usize = MAX_REG_COUNT + 1 + MAX_UNIFORM_REG_COUNT + 1;
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
    pub set: u32,
    pub binding: u32,
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
        0x947 | 0x1947 | 0x1547 | 0x944 => Some((pc as i64 + 1 + offset) as u32),
        0x943 => Some(offset as u32),
        _ => None,
    }
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
            glsl_ext: 0,
            resources: Vec::new(),
            const_cache: HashMap::new(),
        }
    }

    pub fn get_type_void(&self) -> u32 {
        self.type_void
    }

    pub fn emit_entry_point(&mut self, func: u32) {
        // SPIR-V 1.4+ entry points must list every global variable they use, not just Input/Output.
        let resources = self.resources.iter().map(|r| r.var);
        let interface: Vec<u32> = [self.var_abstract_state, self.var_pc].into_iter().chain(resources).collect();
        self.ir
            .emit_entry_point(spirv::execution_model::GLCOMPUTE, func, "main", &interface);
        self.ir
            .emit_execution_mode(func, spirv::execution_mode::LOCAL_SIZE, &[1, 1, 1]);
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
            // std430 layout: a Block struct holding a stride-4 runtime array at offset 0.
            self.ir.emit_decorate(array, deco::ARRAY_STRIDE, &[4]);
            self.ir.emit_decorate(block, deco::BLOCK, &[]);
            self.ir.emit_member_decorate(block, 0, deco::OFFSET, &[0]);
        }
        let var = self.ir.emit_variable(self.type_ptr_ssbo, STORAGE_BUFFER);
        self.ir.emit_decorate(var, deco::DESCRIPTOR_SET, &[0]);
        self.ir.emit_decorate(var, deco::BINDING, &[binding]);
        self.resources.push(Resource { kind, set: 0, binding, var });
        var
    }

    // Vulkan only takes structured control flow, so guest branches store a pc that one loop around a switch dispatches on.
    pub fn translate_program(&mut self, words: &[u128]) {
        let len = words.len() as u32;
        // BRX, JMX and CALL Ra only know their target at run time, so such a program dispatches on every pc; RET lands after a CALL.
        let indirect = words.iter().any(|inst| matches!(opcode(*inst), 0x343 | 0x344 | 0x949 | 0x94c));
        let mut leaders: Vec<u32> = if indirect { (0..=len).collect() } else { vec![0, len] };
        for (pc, &inst) in (0..).zip(words) {
            leaders.extend(branch_target(pc, inst));
            if matches!(
                opcode(inst),
                0x343 | 0x344 | 0x943 | 0x944 | 0x947 | 0x949 | 0x94c | 0x94d | 0x950 | 0x95b | 0x1547 | 0x1947
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
        if pg == 7 && !pg_not {
            return self.translate(inst);
        }
        let mask = self.load_predicate_mask(pg, pg_not);
        let cond = self.ir.emit_inot_equal(self.type_bool[1], mask, self.const_u32_0);
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

    // Register io word order: regs[254], pred, uregs[63], upred.
    fn state_slot(&mut self, slot: u32) -> u32 {
        let (regs, uregs) = (MAX_REG_COUNT as u32, MAX_UNIFORM_REG_COUNT as u32);
        match slot {
            s if s < regs => self.state_ptr(&[0, s]),
            s if s == regs => self.state_ptr(&[1]),
            s if s <= regs + uregs => self.state_ptr(&[2, s - regs - 1]),
            _ => self.state_ptr(&[3]),
        }
    }

    // URZ is index 63: reads zero, writes are dropped.
    fn load_ureg(&mut self, ur: u32) -> u32 {
        if ur == 63 {
            return self.const_u32_0;
        }
        let ptr = self.state_ptr(&[2, ur]);
        self.ir.emit_load(self.type_u32[1], ptr)
    }

    fn store_ureg(&mut self, ur: u32, val: u32) {
        if ur != 63 {
            let ptr = self.state_ptr(&[2, ur]);
            self.ir.emit_store(ptr, val);
        }
    }

    fn store_ureg_predicated(&mut self, ur: u32, pred: u32, invert: bool, val: u32) {
        if ur == 63 {
            return;
        }
        let bit = if pred == 7 {
            self.const_u32_1
        } else {
            let upred = self.load_upredicate();
            let shift = self.cached_const_u32(pred);
            let shifted = self.ir.emit_shift_right_logical(self.type_u32[1], upred, shift);
            self.ir.emit_bitwise_and(self.type_u32[1], shifted, self.const_u32_1)
        };
        let mask = self.ir.emit_isub(self.type_u32[1], self.const_u32_0, bit);
        let mask = if invert { self.ir.emit_not(self.type_u32[1], mask) } else { mask };
        let old = self.load_ureg(ur);
        let val = self.select_masked(mask, val, old);
        self.store_ureg(ur, val);
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
    // Forms 2, 3 and 7 feed that operand to C and move B to Rc at 64..72. The immediate is the whole 32-bit operand.
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
            ResourceKind::Cbuf(slot) if slot < 8 => 1 + slot,
            ResourceKind::Support => 0,
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
        self.resources.push(Resource { kind, set: 0, binding, var });
        var
    }

    // Guest address A is word (A - base) / 4 of the binding 9 SSBO, base being support word 0; below 4 GiB the low words suffice.
    fn global_word(&mut self, inst: u128, word: u32) -> u32 {
        assert!(inst >> 91 & 1 == 0 || inst >> 76 & 1 == 1, "uniform register in a global address");
        let var = match self.resource(ResourceKind::Global) {
            Some(var) => var,
            None => self.declare_ssbo(ResourceKind::Global, 9),
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

    // OpBitcast puts vector component 0 in the low bits, so (Rn, Rn+1) is (lo, hi).
    fn load_pair(&mut self, reg: u32) -> u32 {
        let hi = if reg == 255 { reg } else { reg + 1 };
        let words = [self.load_register(reg), self.load_register(hi)];
        let pair = self.ir.emit_composite_construct(self.type_u32[2], &words);
        let ty = self.type_u64();
        self.ir.emit_bitcast(ty, pair)
    }

    fn store_pair(&mut self, reg: u32, val: u32) {
        if reg == 255 {
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
            let builtin = builtin as u32;
            let ty = match builtin {
                x if x == BuiltIn::LocalInvocationIndex as u32 => this.type_u32[1],
                _ => this.type_u32[3],
            };
            let var = match this.resource(ResourceKind::BuiltIn(builtin)) {
                Some(var) => var,
                None => {
                    let ptr = this.ir.emit_type_pointer(INPUT, ty);
                    let var = this.ir.emit_variable(ptr, INPUT);
                    this.ir.emit_decorate(var, decoration::BUILTIN, &[builtin]);
                    this.resources.push(Resource { kind: ResourceKind::BuiltIn(builtin), set: 0, binding: 0, var });
                    var
                }
            };
            this.ir.emit_load(ty, var)
        };
        match index {
            0 => {
                let invocation = builtin(self, BuiltIn::LocalInvocationIndex);
                let mask = self.cached_const_u32(31);
                self.ir.emit_bitwise_and(self.type_u32[1], invocation, mask)
            }
            3 => {
                let invocation = builtin(self, BuiltIn::LocalInvocationIndex);
                let shift = self.cached_const_u32(5);
                self.ir.emit_shift_right_logical(self.type_u32[1], invocation, shift)
            }
            32 => {
                let tid = builtin(self, BuiltIn::LocalInvocationId);
                let x = self.ir.emit_composite_extract(self.type_u32[1], tid, &[0]);
                let y = self.ir.emit_composite_extract(self.type_u32[1], tid, &[1]);
                let z = self.ir.emit_composite_extract(self.type_u32[1], tid, &[2]);
                let (o16, c8, o26, c6) = (
                    self.cached_const_u32(16), self.cached_const_u32(8),
                    self.cached_const_u32(26), self.cached_const_u32(6),
                );
                let xy = self.ir.emit_bit_field_insert(self.type_u32[1], x, y, o16, c8);
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
            1 | 2 | 4..=31 | 36 | 40..=133 | 255 => self.const_u32_0,
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

    // Hardware applies |x| before negation; 4 is GLSL.std.450 FAbs.
    fn fp_src(&mut self, val: u32, abs: bool, neg: bool) -> u32 {
        let val = if abs { self.glsl(4, self.type_f32[1], &[val]) } else { val };
        if neg { self.ir.emit_fnegate(self.type_f32[1], val) } else { val }
    }

    fn int_src(&mut self, val: u32, neg: bool) -> u32 {
        if neg { self.neg(val) } else { val }
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
        if reg == 255 { // RZ (Zero Register)
            return self.const_u32_0;
        }
        assert!(reg < MAX_REG_COUNT as u32, "Register index out of bounds");
        let array_ptr = self.ir.emit_access_chain(self.type_ptr_abstract_state_regs, self.var_abstract_state, &[self.const_u32_0]);
        let idx = self.cached_const_u32(reg);
        let reg_ptr = self.ir.emit_access_chain(self.type_ptr_u32[1], array_ptr, &[idx]);
        self.ir.emit_load(self.type_u32[1], reg_ptr)
    }

    fn store_register(&mut self, reg: u32, val: u32) {
        if reg == 255 { return; }
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
        if pred == 0x07 {
            let all_ones = self.cached_const_u32(0xffff_ffff);
            if invert {
                self.ir.emit_not(self.type_u32[1], all_ones)
            } else {
                all_ones
            }
        } else {
            let base = self.load_predicate();
            let shift = self.cached_const_u32(pred);
            // extract single bit: (%pred >> %shift) & 1 → 0 or 1
            let srl_res = self.ir.emit_shift_right_logical(self.type_u32[1], base, shift);
            let and_res = self.ir.emit_bitwise_and(self.type_u32[1], srl_res, self.const_u32_1);
            // broadcast to full mask: 0 - bit → 0x00000000 or 0xFFFFFFFF
            let res = self.ir.emit_isub(self.type_u32[1], self.const_u32_0, and_res);
            if invert {
                self.ir.emit_not(self.type_u32[1], res)
            } else {
                res
            }
        }
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
        if reg == 255 { return; } // Write to RZ is always a no-op
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
        if pred_idx == 7 {
            return; // PT cannot be modified
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
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
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
        let pp = (((inst >> 87) & 0x7) << 0);
        let pp_not = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        assert!(pp == 7 && pp_not == 0, "BRA Pp not implemented");
        // .DIV and .CONV test the warp for divergence, and one invocation per register file is always converged.
        if cond == 2 {
            return;
        }
        let mut target = self.cached_const_u32(branch_target(self.pc, inst).unwrap());
        if opcode(inst) == 0x1547 {
            let bit = if upq == 7 {
                self.const_u32_1
            } else {
                let upred = self.load_upredicate();
                let shift = self.cached_const_u32(upq);
                let shifted = self.ir.emit_shift_right_logical(self.type_u32[1], upred, shift);
                self.ir.emit_bitwise_and(self.type_u32[1], shifted, self.const_u32_1)
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
    pub fn brx(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _simm = (((inst >> 34) & 0xffffffffffff) << 0);
        let _depth = (((inst >> 85) & 0x3) << 0);
        let pp = (((inst >> 87) & 0x7) << 0);
        let pp_not = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        assert!(pp == 7 && pp_not == 0, "BRX Pp not implemented");
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
        let pp = (((inst >> 87) & 0x7) << 0);
        let pp_not = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        assert!(pp == 7 && pp_not == 0, "CALL Pp not implemented");
        match opcode(inst) {
            0x343 | 0x344 => self.jump_to_register(inst, inst & 0xfff == 0x344),
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
        let pg = (((inst >> 12) & 0x7) << 0) as u32;
        let pg_not = (((inst >> 15) & 0x1) << 0) != 0;
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _imm8 = (((inst >> 72) & 0xff) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        let rd = ((inst >> 16) & 0xff) as u32;
        let sr = ((inst >> 72) & 0xff) as u32;
        if (inst >> 80) & 1 == 0 {
            let val = self.special_register(sr);
            self.store_register_predicated(rd, pg, pg_not, val);
        } else {
            let hi = if sr == 255 { sr } else { sr + 1 };
            let words = [self.special_register(sr), self.special_register(hi)];
            if rd != 255 {
                for (i, val) in words.into_iter().enumerate() {
                    self.store_register_predicated(rd + i as u32, pg, pg_not, val);
                }
            }
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
        let pp = (((inst >> 87) & 0x7) << 0);
        let pp_not = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        assert!(pp == 7 && pp_not == 0, "EXIT Pp not implemented");
        let exit = self.cached_const_u32(u32::MAX);
        self.ir.emit_store(self.var_pc, exit);
    }
    pub fn f2f(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _dstfmt_srcfmt = (((inst >> 84) & 0x7) << 3) | (((inst >> 75) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
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
        todo!();
    }
    pub fn f2i(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _extract = (((inst >> 60) & 0x3) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _dstfmt = (((inst >> 75) & 0x3) << 1) | (((inst >> 72) & 0x1) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _mode = (((inst >> 84) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
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
        todo!();
    }
    pub fn fadd(&mut self, inst: u128) {
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
    pub fn fchk(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ffma(&mut self, inst: u128) {
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
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _fmz = (((inst >> 80) & 0x1) << 1) | (((inst >> 76) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn flo(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn fmnmx(&mut self, inst: u128) {
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
        let _nan = (((inst >> 81) & 0x1) << 0);
        let _rndsrc = (((inst >> 82) & 0x1) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn fmul(&mut self, inst: u128) {
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
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _fmz = (((inst >> 80) & 0x1) << 1) | (((inst >> 76) & 0x1) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
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
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _extract = (((inst >> 60) & 0x3) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _fmt = (((inst >> 84) & 0x3) << 2) | (((inst >> 75) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
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
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _extract = (((inst >> 60) & 0x3) << 0);
        let _size = (((inst >> 75) & 0x3) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _srcfmt = (((inst >> 84) & 0x3) << 1) | (((inst >> 74) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
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
        todo!();
    }
    pub fn i2i(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
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
        todo!();
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
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
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
        let pp = (((inst >> 87) & 0x7) << 0);
        let pp_not = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        assert!(pp == 7 && pp_not == 0, "JMX Pp not implemented");
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
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        // KILL is pixel-only on hardware, so a compute invocation that reaches one just ends like at EXIT.
        let exit = self.cached_const_u32(u32::MAX);
        self.ir.emit_store(self.var_pc, exit);
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
        let words = match sz {
            4 | 5 => 1 << (sz - 4),
            _ => todo!("LDC size {sz}"),
        };
        let base = self.load_register(ra);
        for i in 0..words {
            let offset = self.cached_const_u32(ra_offset.wrapping_add(4 * i));
            let addr = self.ir.emit_iadd(self.type_u32[1], base, offset);
            let value = self.load_cbuf(ResourceKind::Cbuf(sc_bank), addr);
            self.store_register(if rd == 255 { rd } else { rd + i }, value);
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
        let words = match sz {
            4..=6 => 1 << (sz - 4),
            _ => todo!("LDG size {sz}"),
        };
        for i in 0..words {
            let ptr = self.global_word(inst, i);
            let value = self.ir.emit_load(self.type_u32[1], ptr);
            self.store_register(if rd == 255 { rd } else { rd + i }, value);
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
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
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
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _imm8 = (((inst >> 72) & 0xff) << 0);
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
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _mufuop = (((inst >> 74) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
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
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn prmt(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _pmode = (((inst >> 72) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
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
        let pp = (((inst >> 87) & 0x7) << 0);
        let pp_not = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        assert!(pp == 7 && pp_not == 0, "RET Pp not implemented");
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
        let pg = (((inst >> 12) & 0x7) << 0) as u32;
        let pg_not = (((inst >> 15) & 0x1) << 0) != 0;
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _imm8 = (((inst >> 72) & 0xff) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        let rd = ((inst >> 16) & 0xff) as u32;
        let sr = ((inst >> 72) & 0xff) as u32;
        let val = self.special_register(sr);
        self.store_register_predicated(rd, pg, pg_not, val);
    }
    pub fn s2ur(&mut self, inst: u128) {
        let pg = (((inst >> 12) & 0x7) << 0) as u32;
        let pg_not = (((inst >> 15) & 0x1) << 0) != 0;
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _imm8 = (((inst >> 72) & 0xff) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        let urd = ((inst >> 16) & 0x3f) as u32;
        let sr = ((inst >> 72) & 0xff) as u32;
        let val = self.special_register(sr);
        self.store_ureg_predicated(urd, pg, pg_not, val);
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
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn shf(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _sz = (((inst >> 73) & 0x3) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
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
        let words = match sz {
            4..=6 => 1 << (sz - 4),
            _ => todo!("STG size {sz}"),
        };
        for i in 0..words {
            let ptr = self.global_word(inst, i);
            let value = self.load_register(if rb == 255 { rb } else { rb + i });
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
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ubrev(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
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
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
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
        let pg = (((inst >> 12) & 0x7) << 0) as u32;
        let pg_not = (((inst >> 15) & 0x1) << 0) as u32;
        let urd = (((inst >> 16) & 0x3f) << 0) as u32;
        let ra_offset = (((inst >> 38) & 0xffff) << 0) as u32;
        let sc_bank = (((inst >> 54) & 0x1f) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        assert!(inst >> 91 & 1 == 0 && pg == 7 && pg_not == 0, "ULDC with a uniform address or predicate");
        let words = match sz {
            4 | 5 => 1 << (sz - 4),
            _ => todo!("ULDC size {sz}"),
        };
        for i in 0..words {
            let offset = self.cached_const_u32(ra_offset + 4 * i);
            let value = self.load_cbuf(ResourceKind::Cbuf(sc_bank), offset);
            self.store_ureg(if urd == 63 { urd } else { urd + i }, value);
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
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _ra_urc = (((inst >> 64) & 0x3f) << 0);
        let _imm8 = (((inst >> 72) & 0xff) << 0);
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
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn uprmt(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _ra_urc = (((inst >> 64) & 0x3f) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
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
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ushf(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _ra_urc = (((inst >> 64) & 0x3f) << 0);
        let _sz = (((inst >> 73) & 0x3) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
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
