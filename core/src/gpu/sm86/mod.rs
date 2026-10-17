#![allow(unused_parens)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use std::collections::HashMap;
use ash::vk;
use crate::gpu::VkState;
use crate::gpu::spirv::{self, storage_class};

mod access;
mod atomics;
mod compare;
mod control;
mod convert;
mod fp;
mod int_alu;
mod memory;
mod operands;
mod shared;
mod special;
mod tensor;
mod texture;
mod uniform;
mod warp;

const MAX_REG_COUNT: usize = 254;
const MAX_UNIFORM_REG_COUNT: usize = 63;
pub const STATE_WORDS: usize = MAX_REG_COUNT + 1 + MAX_UNIFORM_REG_COUNT + 1 + 1;
pub const SUPPORT_BINDING: u32 = 0;
pub const CBUF_BINDING: u32 = 1;
pub const GLOBAL_BINDING: u32 = 9;
pub const TEXTURE_BINDING: u32 = 12;
const RZ: u32 = 255;
const URZ: u32 = 63;
const SRZ: u32 = 255;
const PT: u32 = 7;
const INT64_ATOMICS: u32 = 12;
const CALL_ABS_R: u32 = 0x343;
const CALL_REL_R: u32 = 0x344;
const CALL_ABS: u32 = 0x943;
const CALL_REL: u32 = 0x944;
const BRA: u32 = 0x947;
const BRA_UPRED: u32 = 0x1547;
const BRA_UREG: u32 = 0x1947;
const BRX: u32 = 0x949;
const BRXU: u32 = 0x1958;
const JMP_ABS: u32 = 0x94a;
const JMX: u32 = 0x94c;
const JMXU: u32 = 0x1959;
const EXIT: u32 = 0x94d;
const RET: u32 = 0x950;
const KILL: u32 = 0x95b;
const BSYNC: u32 = 0x941;
// Per lane after the BSSY tokens B0..B15: its arrival on each guest barrier, the last BAR.RED result for B2R.RESULT
// and the WARPSYNC mask it waits on.
const BAR_ARRIVALS: u32 = 16;
const BAR_RESULT: u32 = 32;
const WARPSYNC_MASK: u32 = 33;
const SYNC_WORDS: u32 = 34;
// A schedule slot holds the pc vote, then per guest barrier the arrivals plus their BAR.RED predicates in the high half,
// and the arrivals' thread count OR-ed with bit 12 for a running lane yet to arrive.
const SCHEDULE_STRIDE: u32 = 1 + 2 * 16;
const VOTEU: u32 = 0x086;
const UPLOP3: u32 = 0x89c;
const P0_TO_P6: u32 = 0x7f;
const S2UR: u32 = 0x1c3;
const R2UR: u32 = 0x3c2;
const SHARED_WORDS: u32 = 256;
const ROUNDING: [u32; 4] = [spirv::glsl::ROUND_EVEN, spirv::glsl::FLOOR, spirv::glsl::CEIL, spirv::glsl::TRUNC];
static MAX_CONST_BANK: usize = 17;
static ALLOW_F16_PARTIAL_WRITES: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResourceKind {
    BuiltIn(u32),
    Cbuf(u32),
    Ssbo(u32),
    Support,
    Global,
    Local,
    Shared,
    Shared64,
    Warp,
    Schedule,
    Global64,
    Texture2D { cbuf_bank: u32, byte_offset: u32 },
    Surface2D { cbuf_bank: u32, byte_offset: u32, format: vk::Format },
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
    var_live: u32,
    var_pc: u32,
    var_rpc: [u32; 2],
    var_step: u32,
    var_tokens: u32,
    here: Option<u32>,
    active: u32,
    pc: u32,
    const_u32_0: u32,
    const_u32_1: u32,
    type_ptr_ssbo: u32,
    type_ptr_ssbo_u32: u32,
    type_ptr_ssbo64: u32,
    type_ptr_ssbo_u64: u32,
    type_ptr_shared_u32: u32,
    type_ptr_shared_u64: u32,
    type_ptr_cbuf: u32,
    type_ptr_cbuf_u32: u32,
    type_u64: u32,
    type_s64: u32,
    type_f64: u32,
    type_f16: u32,
    type_f16x2: u32,
    mma_vars: [u32; 9],
    type_texture_2d: u32,
    type_sampled_texture_2d: u32,
    type_surface_2d: [u32; 2],
    derivatives: bool,
    invocations: u32,
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
        CALL_ABS | JMP_ABS => Some(offset as u32),
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
fn assert_round_to_nearest(inst: u128) {
    let mode = (inst >> 78 & 3) as usize;
    assert!(mode == 0, "{} rounding mode is not implemented", ["RN", "RM", "RP", "RZ"][mode]);
}

fn is_uniform(inst: u128) -> bool {
    inst >> 91 & 1 != 0 && matches!(inst >> 9 & 7, 1 | 2 | 4)
}

// VOTE, VOTEU, SHFL, MATCH, REDUX, BAR, HMMA, IMMA, DMMA, BMMA, LDSM, MOVM and R2UR.OR, which every invocation of the workgroup must reach together.
fn exchanges(inst: u128) -> bool {
    matches!(opcode(inst) & 0x1ff, 0x006 | 0x086 | 0x189 | 0x1a1 | 0x1c4 | 0x11d | 0x03c | 0x037 | 0x03f | 0x03d | 0x03b | 0x03a) || opcode(inst) == R2UR && inst >> 84 & 1 != 0
}

fn partial_warpsync(inst: u128) -> bool {
    opcode(inst) & 0x1ff == 0x148 && !(opcode(inst) == 0x948 && (inst >> 32) as u32 == u32::MAX)
}

fn guest_barriers(words: &[u128]) -> Vec<u32> {
    let bars = words.iter().filter(|&&inst| opcode(inst) & 0x1ff == 0x11d);
    if bars.clone().any(|&inst| opcode(inst) == 0x51d) {
        return (0..16).collect();
    }
    let mut ids: Vec<u32> = bars.map(|&inst| (inst >> 54 & 0xf) as u32).collect();
    ids.sort_unstable();
    ids.dedup();
    ids
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
    decoder.invocations = local_size.iter().product();
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
            var_live: 0,
            var_pc: 0,
            var_rpc: [0; 2],
            var_step: 0,
            var_tokens: 0,
            here: None,
            active: 0,
            pc: 0,
            const_u32_0: 0,
            const_u32_1: 0,
            type_ptr_ssbo: 0,
            type_ptr_ssbo_u32: 0,
            type_ptr_ssbo64: 0,
            type_ptr_ssbo_u64: 0,
            type_ptr_shared_u32: 0,
            type_ptr_shared_u64: 0,
            type_ptr_cbuf: 0,
            type_ptr_cbuf_u32: 0,
            type_u64: 0,
            type_s64: 0,
            type_f64: 0,
            type_f16: 0,
            type_f16x2: 0,
            mma_vars: [0; 9],
            type_texture_2d: 0,
            type_sampled_texture_2d: 0,
            type_surface_2d: [0; 2],
            derivatives: false,
            invocations: 0,
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
        let interface: Vec<u32> = [self.var_abstract_state, self.var_pc, self.var_rpc[0], self.var_rpc[1], self.var_live, self.var_step, self.var_tokens].into_iter().chain(resources).chain(self.mma_vars.into_iter().filter(|&var| var != 0)).collect();
        self.ir
            .emit_entry_point(spirv::execution_model::GLCOMPUTE, func, "main", &interface);
        self.ir
            .emit_execution_mode(func, spirv::execution_mode::LOCAL_SIZE, &local_size);
        use spirv::{capability as cap, execution_mode as mode};
        let Some(device) = device else { return };
        for (capability, execution_mode, widths) in [
            (cap::SIGNED_ZERO_INF_NAN_PRESERVE, mode::SIGNED_ZERO_INF_NAN_PRESERVE, device.signed_zero_inf_nan_preserve),
            (cap::DENORM_PRESERVE, mode::DENORM_PRESERVE, device.denorm_preserve),
        ] {
            if widths.contains(&true) {
                self.ir.emit_capability(capability);
            }
            for (width, _) in [16, 32, 64].into_iter().zip(widths).filter(|&(_, supported)| supported) {
                self.ir.emit_execution_mode(func, execution_mode, &[width]);
            }
        }
        if self.derivatives {
            self.ir.emit_capability(cap::COMPUTE_DERIVATIVE_GROUP_LINEAR);
            self.ir.emit_extension("SPV_KHR_compute_shader_derivatives");
            self.ir.emit_execution_mode(func, mode::DERIVATIVE_GROUP_LINEAR, &[]);
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
        self.var_rpc = [(); 2].map(|_| self.ir.emit_variable(self.type_ptr_u32[1], storage_class::PRIVATE));
        self.var_live = self.ir.emit_variable_init(self.type_ptr_u32[1], storage_class::PRIVATE, self.const_u32_1);
        self.var_step = self.ir.emit_variable_init(self.type_ptr_u32[1], storage_class::PRIVATE, self.const_u32_0);
        let barriers = self.cached_const_u32(SYNC_WORDS);
        let type_tokens = self.ir.emit_type_array(self.type_u32[1], barriers);
        let no_tokens = self.ir.emit_constant_composite(type_tokens, &[self.const_u32_0; SYNC_WORDS as usize]);
        let type_ptr_tokens = self.ir.emit_type_pointer(storage_class::PRIVATE, type_tokens);
        self.var_tokens = self.ir.emit_variable_init(type_ptr_tokens, storage_class::PRIVATE, no_tokens);
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
    // Each source lane yields its value, or 0 when it is not active this step or past the end of a partial last warp, and its active bit.
    fn warp_exchange(&mut self, val: u32, lanes: &[u32]) -> Vec<[u32; 2]> {
        const MAX_INVOCATIONS: u32 = 1024;
        let ty = self.type_u32[1];
        let invocation = self.builtin(spirv::BuiltIn::LocalInvocationIndex);
        let flags = self.cached_const_u32(MAX_INVOCATIONS);
        let val = self.ir.emit_select(ty, self.active, val, self.const_u32_0);
        let flag = self.bool_bit(self.active);
        for (word, offset) in [(val, self.const_u32_0), (flag, flags)] {
            let index = self.ir.emit_iadd(ty, invocation, offset);
            let own = self.workgroup_word(ResourceKind::Warp, 2 * MAX_INVOCATIONS, index);
            self.ir.emit_store(own, word);
        }
        self.workgroup_barrier();
        let first_lane = self.cached_const_u32(!31);
        let base = self.ir.emit_bitwise_and(ty, invocation, first_lane);
        let mut values = Vec::new();
        for &lane in lanes {
            let index = self.ir.emit_iadd(ty, base, lane);
            let words = [self.const_u32_0, flags].map(|offset| {
                let index = self.ir.emit_iadd(ty, index, offset);
                let slot = self.workgroup_word(ResourceKind::Warp, 2 * MAX_INVOCATIONS, index);
                self.ir.emit_load(ty, slot)
            });
            if self.invocations % 32 == 0 {
                values.push(words);
                continue;
            }
            let invocations = self.cached_const_u32(self.invocations);
            let exists = self.ir.emit_uless_than(self.type_bool[1], index, invocations);
            values.push(words.map(|word| self.ir.emit_select(ty, exists, word, self.const_u32_0)));
        }
        self.workgroup_barrier();
        values
    }

    fn workgroup_barrier(&mut self) {
        use spirv::memory_semantics::{ACQUIRE_RELEASE, WORKGROUP_MEMORY};
        let [scope, semantics] = [spirv::scope::WORKGROUP, ACQUIRE_RELEASE | WORKGROUP_MEMORY].map(|c| self.cached_const_u32(c));
        self.ir.emit_control_barrier(scope, scope, semantics);
    }

    fn warp_values(&mut self, val: u32) -> Vec<[u32; 2]> {
        let lanes: Vec<u32> = (0..32).map(|lane| self.cached_const_u32(lane)).collect();
        self.warp_exchange(val, &lanes)
    }

    fn bool_bit(&mut self, cond: u32) -> u32 {
        self.ir.emit_select(self.type_u32[1], cond, self.const_u32_1, self.const_u32_0)
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
    // Warp collectives and BAR meet at workgroup barriers, so a program with them steps every invocation through the same
    // iterations: each iteration runs the lowest pc a lane can run and lanes at any other pc sit it out.
    pub fn translate_program(&mut self, words: &[u128]) {
        let len = words.len() as u32;
        // BRX, JMX and CALL Ra only know their target at run time, so such a program dispatches on every pc; RET lands after a CALL.
        let indirect = words.iter().any(|inst| matches!(opcode(*inst), CALL_ABS_R | CALL_REL_R | 0x1943 | 0x1944 | 0xb43 | BRX | BRXU | JMX | JMXU | 0xb4a | 0x1b4a | 0x174a | 0x194a | 0x154a | 0x1950));
        let mut leaders: Vec<u32> = if indirect { (0..=len).collect() } else { vec![0, len] };
        for (pc, &inst) in (0..).zip(words) {
            leaders.extend(branch_target(pc, inst));
            if opcode(inst) == BSYNC {
                leaders.push(pc);
            }
            if opcode(inst) & 0x1ff == 0x11d || partial_warpsync(inst) {
                leaders.push(pc + 1);
            }
            if matches!(
                opcode(inst),
                CALL_ABS_R | CALL_REL_R | CALL_ABS | CALL_REL | 0x1943 | 0x1944 | 0xb43 | BRA | BRA_UPRED | BRA_UREG | BRX | BRXU | JMP_ABS | 0xb4a | 0x1b4a | 0x174a | 0x194a | 0x154a | JMX | JMXU | EXIT | RET | 0x1950 | KILL
            ) {
                leaders.push(pc + 1);
            }
        }
        leaders.sort_unstable();
        leaders.dedup();
        assert_eq!(leaders.last(), Some(&len), "branch target outside the program");
        let uniform = words.iter().any(|&inst| exchanges(inst) || partial_warpsync(inst));
        let [header, body, dispatched, cont, merge] = [(); 5].map(|_| self.ir.alloc_id());
        let blocks: Vec<(u32, u32)> = leaders[..leaders.len() - 1].iter().map(|&pc| (pc, self.ir.alloc_id())).collect();
        let live = self.ir.emit_load(self.type_u32[1], self.var_live);
        let dead = self.ir.emit_iequal(self.type_bool[1], live, self.const_u32_0);
        let end = self.cached_const_u32(len);
        let start = self.ir.emit_select(self.type_u32[1], dead, end, self.const_u32_0);
        self.ir.emit_store(self.var_pc, start);
        if uniform {
            self.clear_schedule_slot(self.const_u32_0, &guest_barriers(words));
            self.workgroup_barrier();
        }
        self.ir.emit_branch(header);
        self.ir.emit_label_id(header);
        self.ir.emit_loop_merge(merge, cont, 0);
        self.ir.emit_branch(body);
        self.ir.emit_label_id(body);
        let pc = self.ir.emit_load(self.type_u32[1], self.var_pc);
        let [scheduled, progress] = if uniform { self.schedule(words, pc) } else { [pc; 2] };
        self.ir.emit_selection_merge(dispatched, 0);
        self.ir.emit_switch(scheduled, dispatched, &blocks);
        for (&(start, label), &end) in blocks.iter().zip(&leaders[1..]) {
            self.ir.emit_label_id(label);
            let mut next = self.cached_const_u32(end);
            if let Some(here) = self.here {
                next = self.ir.emit_select(self.type_u32[1], here, next, pc);
            }
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
        let progress = if uniform {
            let step = self.ir.emit_load(self.type_u32[1], self.var_step);
            let step = self.ir.emit_iadd(self.type_u32[1], step, self.const_u32_1);
            self.ir.emit_store(self.var_step, step);
            progress
        } else {
            self.ir.emit_load(self.type_u32[1], self.var_pc)
        };
        let done = self.ir.emit_ugreater_than_equal(self.type_bool[1], progress, end);
        self.ir.emit_branch_conditional(done, merge, header);
        self.ir.emit_label_id(merge);
        if self.here.take().is_some() {
            // A lane still short of the end is stuck on a BSYNC, BAR or WARPSYNC nobody can release, and reads back live = 2.
            let pc = self.ir.emit_load(self.type_u32[1], self.var_pc);
            let stuck = self.ir.emit_uless_than(self.type_bool[1], pc, end);
            let stuck = self.bool_bit(stuck);
            let live = self.ir.emit_iadd(self.type_u32[1], live, stuck);
            self.ir.emit_store(self.var_live, live);
        }
    }

    fn barrier_token(&mut self, barrier: u32) -> u32 {
        let index = self.cached_const_u32(barrier);
        self.sync_word(index)
    }

    fn sync_word(&mut self, index: u32) -> u32 {
        self.ir.emit_access_chain(self.type_ptr_u32[1], self.var_tokens, &[index])
    }

    fn translate_guarded(&mut self, inst: u128) {
        if inst >> 12 & 0xf == 0xf {
            return;
        }
        let cond = match (self.guard(inst), self.here) {
            (Some(guard), Some(here)) => Some(self.ir.emit_logical_and(self.type_bool[1], guard, here)),
            (guard, here) => guard.or(here),
        };
        let Some(cond) = cond else { return self.translate(inst) };
        if exchanges(inst) {
            self.active = cond;
            let before = self.ir.emit_load(self.type_abstract_state, self.var_abstract_state);
            self.translate(inst);
            let after = self.ir.emit_load(self.type_abstract_state, self.var_abstract_state);
            let kept = self.ir.emit_select(self.type_abstract_state, cond, after, before);
            return self.ir.emit_store(self.var_abstract_state, kept);
        }
        let [then, after] = [(); 2].map(|_| self.ir.alloc_id());
        self.ir.emit_selection_merge(after, 0);
        self.ir.emit_branch_conditional(cond, then, after);
        self.ir.emit_label_id(then);
        self.translate(inst);
        self.ir.emit_branch(after);
        self.ir.emit_label_id(after);
    }

    fn guard(&mut self, inst: u128) -> Option<u32> {
        let (pg, pg_not) = ((inst >> 12 & 7) as u32, inst >> 15 & 1 != 0);
        if pg == PT {
            return None;
        }
        // Uniform datapath opcodes, except VOTEU, take their guard from UP0..UP6 instead of P0..P6.
        let op = opcode(inst) & 0x1ff;
        let uniform = matches!(op, 0x80..=0xbf | S2UR) && op != VOTEU;
        let bit = self.pred_bit(uniform, pg, pg_not);
        Some(self.ir.emit_inot_equal(self.type_bool[1], bit, self.const_u32_0))
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
            s if s == regs + uregs + 1 => self.state_ptr(&[3]),
            _ => self.var_live,
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
}

include!("../sm86_decoder_generated.rs");
