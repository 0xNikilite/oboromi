#![allow(unused_parens)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap};
use ash::vk;
use crate::gpu::VkState;
use crate::gpu::threed::PrimitiveTopology;
use crate::gpu::spirv::{self, storage_class};

mod access;
mod atomics;
mod cfg;
pub use cfg::{plan, Translation};
mod compare;
mod control;
mod convert;
mod fp;
mod int_alu;
mod memory;
mod operands;
mod ray;
mod shared;
mod special;
mod structured;
mod tensor;
pub use tensor::{cooperative_tile, f16_hmma_shape};
mod texture;
mod uniform;
mod warp;

const MAX_REG_COUNT: usize = 254;
const MAX_UNIFORM_REG_COUNT: usize = 63;
pub const STATE_WORDS: usize = MAX_REG_COUNT + 1 + MAX_UNIFORM_REG_COUNT + 1 + 1;
pub const SUPPORT_BINDING: u32 = 0;
// Bank n of the 32 a stage has (BIND_GROUP_CONSTANT_BUFFER SHADER_SLOT is 5 bits in clc797.h) is binding 1 + n.
pub const CBUF_BINDING: u32 = 1;
pub const CBUF_BANKS: u32 = 32;
pub const GLOBAL_BINDING: u32 = CBUF_BINDING + CBUF_BANKS;
pub const TEXTURE_BINDING: u32 = GLOBAL_BINDING + 3;
pub const PMTRIG_WORDS: usize = 1 << 16;
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
// Per lane after the BSSY tokens B0..B15: its arrival on each guest barrier, the last BAR.RED result for B2R.RESULT,
// the WARPSYNC mask it waits on and the byte address its EXIT jumps to, 0 for none.
const BAR_ARRIVALS: u32 = 16;
const BAR_RESULT: u32 = 32;
const WARPSYNC_MASK: u32 = 33;
const ATEXIT_PC: u32 = 34;
const SYNC_WORDS: u32 = 35;
// A schedule slot holds the pc vote, then per guest barrier the arrivals plus their BAR.RED predicates in the high half,
// and the arrivals' thread count OR-ed with bit 12 for a running lane yet to arrive.
const SCHEDULE_STRIDE: u32 = 1 + 2 * 16;
const VOTEU: u32 = 0x086;
const UPLOP3: u32 = 0x89c;
const P0_TO_P6: u32 = 0x7f;
const S2UR: u32 = 0x1c3;
const R2UR: u32 = 0x3c2;
pub const SHARED_WORDS: u32 = 256;
const ROUNDING: [u32; 4] = [spirv::glsl::ROUND_EVEN, spirv::glsl::FLOOR, spirv::glsl::CEIL, spirv::glsl::TRUNC];
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
    Mma,
    Schedule,
    Global64,
    GlobalVector(u32),
    Hints,
    Texture2D { cbuf_bank: u32, byte_offset: u32 },
    Surface2D { cbuf_bank: u32, byte_offset: u32, format: vk::Format },
    VertexInput { location: u32 },
    Varying { location: u32, flat: bool },
    FragmentOutput { location: u32 },
    /// The vec4 at attribute `attr` of every vertex of the primitive or patch.
    PerVertex { attr: u32, output: bool },
    Patch { location: u32 },
    Ttu,
    RayQuery,
    AccelerationStructure { cbuf_bank: u32, byte_offset: u32 },
}

/// A module binds its acceleration structure after its textures and surfaces.
pub fn acceleration_structure_binding(kinds: &[ResourceKind]) -> u32 {
    TEXTURE_BINDING + kinds.iter().filter(|kind| matches!(kind, ResourceKind::Texture2D { .. } | ResourceKind::Surface2D { .. })).count() as u32
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Stage {
    Compute { local_size: [u32; 3] },
    /// Bit n of `uint` and `sint` is set when generic attribute n is fetched as an unsigned or signed integer.
    Vertex { uint: u32, sint: u32 },
    /// `input` is the draw's primitive. From the SPH: `output` is OUTPUT_TOPOLOGY (clc797sph.h:60), `max_vertices`
    /// MAX_OUTPUT_VERTEX_COUNT (:64), `invocations` and `vertices` THREADS_PER_INPUT_PRIMITIVE (:58). `mode`, `spacing` and
    /// `order` are DOMAIN_TYPE, SPACING and OUTPUT_PRIMITIVES of SET_TESSELLATION_PARAMETERS (clc797.h:691).
    Geometry { input: PrimitiveTopology, output: u32, max_vertices: u32, invocations: u32 },
    TessControl { vertices: u32 },
    TessEval { mode: u32, spacing: u32, order: u32 },
    /// Bit n of `flat` and `screen_linear` is the interpolation of generic attribute n; `depth` writes FragDepth.
    Fragment { flat: u32, screen_linear: u32, depth: bool },
}

#[derive(Debug, Clone, Copy)]
pub struct Resource {
    pub kind: ResourceKind,
    pub var: u32,
}

pub struct Decoder<'a> {
    pub ir: &'a mut spirv::Emitter,
    stage: Stage,
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
    device: Option<&'a VkState>,
    pub force_cooperative_hmma: Option<bool>,
    tensor: TensorMode,
    cooperative_types: HashMap<[u32; 4], u32>,
    type_texture_2d: u32,
    type_sampled_texture_2d: u32,
    type_surface_2d: [u32; 2],
    type_acceleration_structure: u32,
    acceleration_structure_address: Option<(u32, u32)>,
    derivatives: bool,
    invocations: u32,
    glsl_ext: u32,
    pub resources: Vec<Resource>,
    pub stores_global: bool,
    // Cache for emitted u32 constants to avoid duplicate OpConstant
    const_cache: HashMap<u32, u32>,
    timing: Timing,
    translation: Translation,
    entry: usize,
    type_ptr_local: u32,
    locals: Option<Locals>,
    pending_nan: BTreeMap<u32, u32>,
}

#[derive(Clone, Copy, Default)]
pub struct Hints {
    pub timing: bool,
    pub translation: Translation,
    pub tensor: TensorMode,
}

#[derive(Clone, Copy, PartialEq)]
pub enum TensorMode {
    Exact,
    Fast,
}

impl Default for TensorMode {
    fn default() -> Self {
        if crate::gpu::SPEED_HACKS.lock().unwrap().fast_tensor { Self::Fast } else { Self::Exact }
    }
}

// Function variables standing in for the state slots a structured program touches, declared at the function's
// `entry` and seeded from the aggregate at `start`.
struct Locals {
    vars: BTreeMap<u32, u32>,
    entry: usize,
    start: usize,
}

enum Timing {
    Off,
    Fallback,
    Clock { period: f32, emitted: bool },
}

fn opcode(inst: u128) -> u32 {
    ((inst >> 91 & 1) << 12 | inst & 0xfff) as u32
}

// In instructions: bits 81:34 hold a signed byte offset divided by four.
fn branch_offset(inst: u128) -> i64 {
    ((inst >> 34) as i64) << 16 >> 18
}

pub fn branch_target(pc: u32, inst: u128) -> Option<u32> {
    let offset = branch_offset(inst);
    match opcode(inst) {
        BRA | BRA_UREG | BRA_UPRED | CALL_REL => Some((pc as i64 + 1 + offset) as u32),
        CALL_ABS | JMP_ABS => Some(offset as u32),
        _ => None,
    }
}

fn dynamic_target(inst: u128) -> bool {
    matches!(opcode(inst), 0x357 | 0x957 | CALL_ABS_R | CALL_REL_R | 0x1943 | 0x1944 | 0xb43 | BRX | BRXU | JMX | JMXU | 0xb4a | 0x1b4a | 0x174a | 0x194a | 0x154a | 0x1950)
}

fn guard_pred(inst: u128) -> Option<cfg::Pred> {
    let (index, not) = ((inst >> 12 & 7) as u32, inst >> 15 & 1 != 0);
    let op = opcode(inst) & 0x1ff;
    (index != PT).then(|| cfg::Pred { uniform: matches!(op, 0x80..=0xbf | S2UR) && op != VOTEU, index, not })
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

fn deferred_nan(inst: u128) -> bool {
    match opcode(inst) & 0x1ff {
        0x020 | 0x021 | 0x023 | 0x009 | 0x108 => true, // FMUL, FADD, FFMA, FMNMX, MUFU
        0x107 => inst >> 84 & 3 == 2 && inst >> 75 & 3 == 2, // FRND.F32.F32
        _ => false,
    }
}

fn ignores_nan_payload(inst: u128) -> bool {
    deferred_nan(inst) || matches!(opcode(inst) & 0x1ff, 0x00a | 0x00b) // FSET, FSETP
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

// The input execution mode of a geometry shader drawn with `input` and the vertices of each of its primitives.
fn geometry_input(input: PrimitiveTopology) -> (u32, u32) {
    use {PrimitiveTopology::*, spirv::execution_mode::*};
    match input {
        Points => (INPUT_POINTS, 1),
        Lines | LineLoop | LineStrip => (INPUT_LINES, 2),
        LineListAdjacency | LineStripAdjacency => (INPUT_LINES_ADJACENCY, 4),
        Triangles | TriangleStrip | TriangleFan => (TRIANGLES, 3),
        TriangleListAdjacency | TriangleStripAdjacency => (INPUT_TRIANGLES_ADJACENCY, 6),
        _ => panic!("a geometry shader cannot draw {input:?}"),
    }
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

pub fn emit_module(device: Option<&VkState>, stage: Stage, body: impl FnOnce(&mut Decoder)) -> spirv::Emitter {
    let mut hints = Hints::default();
    if !matches!(stage, Stage::Compute { .. }) {
        hints.translation = Translation::ForceDispatcher;
    }
    emit_module_hinted(device, stage, hints, body)
}

pub fn emit_module_hinted(device: Option<&VkState>, stage: Stage, hints: Hints, body: impl FnOnce(&mut Decoder)) -> spirv::Emitter {
    let mut emitter = spirv::Emitter::new();
    emitter.emit_header();
    emitter.emit_capability(spirv::capability::SHADER);
    emitter.emit_memory_model(0, 1);
    let mut decoder = Decoder::new(&mut emitter, stage, hints, device);
    decoder.init();
    let void_ty = decoder.get_type_void();
    let func_ty = decoder.ir.emit_type_function(void_ty, &[]);
    let func = decoder.ir.emit_function(void_ty, 0, func_ty);
    decoder.ir.emit_label();
    decoder.entry = decoder.ir.function_len();
    if let Stage::Compute { local_size } = stage {
        decoder.invocations = local_size.iter().product();
    }
    body(&mut decoder);
    decoder.emit_entry_point(func, device);
    decoder.ir.emit_return();
    decoder.ir.emit_function_end();
    decoder.ir.finalize();
    emitter
}

impl<'a> Decoder<'a> {
    pub fn new(ir: &'a mut spirv::Emitter, stage: Stage, hints: Hints, device: Option<&'a VkState>) -> Self {
        Self {
            ir,
            stage,
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
            device,
            force_cooperative_hmma: None,
            tensor: hints.tensor,
            cooperative_types: HashMap::new(),
            type_texture_2d: 0,
            type_sampled_texture_2d: 0,
            type_surface_2d: [0; 2],
            type_acceleration_structure: 0,
            acceleration_structure_address: None,
            derivatives: false,
            invocations: 0,
            glsl_ext: 0,
            resources: Vec::new(),
            stores_global: false,
            const_cache: HashMap::new(),
            translation: hints.translation,
            entry: 0,
            type_ptr_local: 0,
            locals: None,
            pending_nan: BTreeMap::new(),
            timing: if !hints.timing { Timing::Off } else if let Some(device) = device.filter(|d| d.shader_device_clock) {
                Timing::Clock { period: device.timestamp_period, emitted: false }
            } else {
                Timing::Fallback
            },
        }
    }

    pub fn get_type_void(&self) -> u32 {
        self.type_void
    }

    pub fn emit_entry_point(&mut self, func: u32, device: Option<&VkState>) {
        if let Some(var) = self.resource(ResourceKind::Hints) {
            let textures = self.resources.iter().filter(|r| matches!(r.kind, ResourceKind::Texture2D { .. } | ResourceKind::Surface2D { .. })).count() as u32;
            self.ir.emit_decorate(var, spirv::decoration::BINDING, &[TEXTURE_BINDING + textures]);
        }
        // SPIR-V 1.4+ entry points must list every global variable they use, not just Input/Output.
        let resources = self.resources.iter().map(|r| r.var);
        let interface: Vec<u32> = [self.var_abstract_state, self.var_pc, self.var_rpc[0], self.var_rpc[1], self.var_live, self.var_step, self.var_tokens].into_iter().chain(resources).chain(self.mma_vars.into_iter().filter(|&var| var != 0)).chain(self.pending_nan.values().copied()).collect();
        use spirv::{capability as cap, execution_mode as mode, execution_model as model};
        let execution_model = match self.stage {
            Stage::Compute { .. } => model::GLCOMPUTE,
            Stage::Vertex { .. } => model::VERTEX,
            Stage::TessControl { .. } => model::TESSELLATION_CONTROL,
            Stage::TessEval { .. } => model::TESSELLATION_EVALUATION,
            Stage::Geometry { .. } => model::GEOMETRY,
            Stage::Fragment { .. } => model::FRAGMENT,
        };
        self.bind_acceleration_structure();
        self.ir.emit_entry_point(execution_model, func, "main", &interface);
        match self.stage {
            Stage::Compute { local_size } => self.ir.emit_execution_mode(func, mode::LOCAL_SIZE, &local_size),
            Stage::Vertex { .. } => {}
            Stage::Geometry { input, output, max_vertices, invocations } => {
                self.ir.emit_capability(cap::GEOMETRY);
                let output = match output {
                    1 => mode::OUTPUT_POINTS,
                    6 => mode::OUTPUT_LINE_STRIP,
                    7 => mode::OUTPUT_TRIANGLE_STRIP,
                    _ => panic!("OUTPUT_TOPOLOGY {output} is not a geometry shader output"),
                };
                for (mode, literals) in [(geometry_input(input).0, &[][..]), (output, &[]), (mode::OUTPUT_VERTICES, &[max_vertices]), (mode::INVOCATIONS, &[invocations])] {
                    self.ir.emit_execution_mode(func, mode, literals);
                }
            }
            Stage::TessControl { vertices } => {
                self.ir.emit_capability(cap::TESSELLATION);
                self.ir.emit_execution_mode(func, mode::OUTPUT_VERTICES, &[vertices]);
            }
            Stage::TessEval { mode: domain, spacing, order } => {
                self.ir.emit_capability(cap::TESSELLATION);
                let order = [Some(mode::POINT_MODE), None, Some(mode::VERTEX_ORDER_CW), Some(mode::VERTEX_ORDER_CCW)][order as usize];
                let modes = [[mode::ISOLINES, mode::TRIANGLES, mode::QUADS][domain as usize], [mode::SPACING_EQUAL, mode::SPACING_FRACTIONAL_ODD, mode::SPACING_FRACTIONAL_EVEN][spacing as usize]];
                for mode in modes.into_iter().chain(order) {
                    self.ir.emit_execution_mode(func, mode, &[]);
                }
            }
            Stage::Fragment { depth, .. } => {
                self.ir.emit_execution_mode(func, mode::ORIGIN_UPPER_LEFT, &[]);
                if depth {
                    self.ir.emit_execution_mode(func, mode::DEPTH_REPLACING, &[]);
                }
            }
        }
        if self.resource(ResourceKind::BuiltIn(spirv::BuiltIn::SampleId as u32)).is_some() {
            self.ir.emit_capability(cap::SAMPLE_RATE_SHADING);
        }
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
        if self.derivatives && matches!(self.stage, Stage::Compute { .. }) {
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
        self.type_ptr_local = self.ir.emit_type_pointer(storage_class::FUNCTION, self.type_u32[1]);

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
            let value = if slot < MAX_REG_COUNT as u32 { self.load_register(slot) } else { self.load_slot(slot) };
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
        if matches!(self.stage, Stage::Fragment { .. }) {
            self.ir.emit_capability(spirv::capability::GROUP_NON_UNIFORM);
            self.ir.emit_capability(spirv::capability::GROUP_NON_UNIFORM_SHUFFLE);
            self.ir.emit_capability(spirv::capability::GROUP_NON_UNIFORM_BALLOT);
            let scope = self.cached_const_u32(spirv::scope::SUBGROUP);
            let own = self.builtin(spirv::BuiltIn::SubgroupLocalInvocationId);
            let yes = self.ir.emit_constant_true(self.type_bool[1]);
            let present = self.ir.emit_group_non_uniform_ballot(self.type_u32[4], scope, yes);
            let mask = self.cached_const_u32(!31);
            let base = self.ir.emit_bitwise_and(ty, own, mask);
            let value = self.ir.emit_select(ty, self.active, val, self.const_u32_0);
            let active = self.bool_bit(self.active);
            return lanes.iter().map(|&lane| {
                let source = self.ir.emit_iadd(ty, base, lane);
                let exists = self.ir.emit_group_non_uniform_ballot_bit_extract(self.type_bool[1], scope, present, source);
                let source = self.ir.emit_select(ty, exists, source, own);
                [value, active].map(|word| {
                    let word = self.ir.emit_group_non_uniform_shuffle(ty, scope, word, source);
                    self.ir.emit_select(ty, exists, word, self.const_u32_0)
                })
            }).collect();
        }
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
        self.ir.emit_decorate(var, deco::DESCRIPTOR_SET, &[self.descriptor_set()]);
        if kind != ResourceKind::Hints {
            self.ir.emit_decorate(var, deco::BINDING, &[binding]);
        }
        self.resources.push(Resource { kind, var });
        var
    }

    // Vulkan only takes structured control flow, so guest branches store a pc that one loop around a switch dispatches on.
    // Warp collectives and BAR meet at workgroup barriers, so a program with them steps every invocation through the same
    // iterations: each iteration runs the lowest pc a lane can run and lanes at any other pc sit it out.
    pub fn translate_program(&mut self, words: &[u128]) {
        self.acceleration_structure_address = ray::acceleration_structure_address(words);
        for &inst in words.iter().filter(|&&inst| deferred_nan(inst)) {
            let reg = (inst >> 16 & 0xff) as u32;
            if reg != RZ { self.nan_flag(reg); }
        }
        if self.translation != Translation::ForceDispatcher {
            match plan(words) {
                Ok(plan) => return self.translate_structured(words, &plan),
                Err(reason) => assert!(self.translation == Translation::Auto, "no structured translation: {reason}"),
            }
        }
        let len = words.len() as u32;
        // BRX, JMX, CALL Ra and BMOV ATEXIT_PC only know their target at run time, so such a program dispatches on every pc; RET
        // lands after a CALL.
        let indirect = words.iter().any(|&inst| dynamic_target(inst));
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
        if uniform && matches!(self.stage, Stage::Compute { .. }) {
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
            let flags: Vec<_> = self.pending_nan.values().map(|&var| (var, self.ir.emit_load(self.type_u32[1], var))).collect();
            let before = self.ir.emit_load(self.type_abstract_state, self.var_abstract_state);
            self.translate(inst);
            let after = self.ir.emit_load(self.type_abstract_state, self.var_abstract_state);
            let kept = self.ir.emit_select(self.type_abstract_state, cond, after, before);
            for (var, before) in flags {
                let after = self.ir.emit_load(self.type_u32[1], var);
                let kept = self.ir.emit_select(self.type_u32[1], cond, after, before);
                self.ir.emit_store(var, kept);
            }
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
        guard_pred(inst).map(|pred| self.holds(&[pred]))
    }

    // A pixel program binds its constant buffers and textures apart from the vertex program's, in descriptor set 1.
    fn descriptor_set(&self) -> u32 {
        u32::from(matches!(self.stage, Stage::Fragment { .. }))
    }

    fn resource(&self, kind: ResourceKind) -> Option<u32> {
        self.resources.iter().find(|r| r.kind == kind).map(|r| r.var)
    }

    fn state_ptr(&mut self, path: &[u32]) -> u32 {
        let ids: Vec<u32> = path.iter().map(|&i| self.cached_const_u32(i)).collect();
        self.ir.emit_access_chain(self.type_ptr_u32[1], self.var_abstract_state, &ids)
    }

    fn state_slot(&mut self, slot: u32) -> u32 {
        match self.locals {
            Some(_) if slot < STATE_WORDS as u32 - 1 => self.local(slot),
            _ => self.aggregate_slot(slot),
        }
    }

    fn local(&mut self, slot: u32) -> u32 {
        let locals = self.locals.as_ref().unwrap();
        if let Some(&var) = locals.vars.get(&slot) {
            return var;
        }
        // SPIR-V 2.4: a function's OpVariables are the first instructions of its first block.
        let (entry, len) = (locals.entry, self.ir.function_len());
        let var = self.ir.emit_variable(self.type_ptr_local, storage_class::FUNCTION);
        self.ir.move_tail(len, entry);
        let start = self.locals.as_ref().unwrap().start + self.ir.function_len() - len;
        let len = self.ir.function_len();
        let ptr = self.aggregate_slot(slot);
        let value = self.ir.emit_load(self.type_u32[1], ptr);
        self.ir.emit_store(var, value);
        self.ir.move_tail(len, start);
        let locals = self.locals.as_mut().unwrap();
        locals.start = start;
        locals.vars.insert(slot, var);
        var
    }

    fn aggregate_slot(&mut self, slot: u32) -> u32 {
        let (regs, uregs) = (MAX_REG_COUNT as u32, MAX_UNIFORM_REG_COUNT as u32);
        match slot {
            s if s < regs => self.state_ptr(&[0, s]),
            s if s == regs => self.state_ptr(&[1]),
            s if s <= regs + uregs => self.state_ptr(&[2, s - regs - 1]),
            s if s == regs + uregs + 1 => self.state_ptr(&[3]),
            _ => self.var_live,
        }
    }

    fn load_slot(&mut self, slot: u32) -> u32 {
        let ptr = self.state_slot(slot);
        self.ir.emit_load(self.type_u32[1], ptr)
    }

    fn store_slot(&mut self, slot: u32, val: u32) {
        let ptr = self.state_slot(slot);
        self.ir.emit_store(ptr, val);
    }

    fn load_ureg(&mut self, ur: u32) -> u32 {
        if ur == URZ {
            return self.const_u32_0;
        }
        self.load_slot(MAX_REG_COUNT as u32 + 1 + ur)
    }

    fn store_ureg(&mut self, ur: u32, val: u32) {
        if ur != URZ {
            self.store_slot(MAX_REG_COUNT as u32 + 1 + ur, val);
        }
    }

    fn load_upredicate(&mut self) -> u32 {
        self.load_slot(STATE_WORDS as u32 - 2)
    }

    fn store_upredicate(&mut self, val: u32) {
        self.store_slot(STATE_WORDS as u32 - 2, val);
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

    fn nan_flag(&mut self, reg: u32) -> u32 {
        *self.pending_nan.entry(reg).or_insert_with(|| {
            self.ir.emit_variable_init(self.type_ptr_u32[1], storage_class::PRIVATE, self.const_u32_0)
        })
    }

    fn store_fp_register(&mut self, reg: u32, bits: u32) {
        if reg == RZ { return; }
        assert!(reg < MAX_REG_COUNT as u32, "Register index out of bounds");
        let flag = self.nan_flag(reg);
        self.store_slot(reg, bits);
        self.ir.emit_store(flag, self.const_u32_1);
    }

    fn load_register(&mut self, reg: u32) -> u32 {
        let bits = self.load_float_register(reg);
        let Some(&flag) = self.pending_nan.get(&reg) else { return bits };
        let pending = self.ir.emit_load(self.type_u32[1], flag);
        let pending = self.ir.emit_inot_equal(self.type_bool[1], pending, self.const_u32_0);
        let val = self.bitcast_f32(bits);
        let canonical = self.canonical_nan_bits(val);
        let bits = self.ir.emit_select(self.type_u32[1], pending, canonical, bits);
        self.store_register(reg, bits);
        bits
    }

    fn load_float_register(&mut self, reg: u32) -> u32 {
        if reg == RZ {
            return self.const_u32_0;
        }
        assert!(reg < MAX_REG_COUNT as u32, "Register index out of bounds");
        self.load_slot(reg)
    }

    fn store_register(&mut self, reg: u32, val: u32) {
        if reg == RZ { return; }
        assert!(reg < MAX_REG_COUNT as u32, "Register index out of bounds");
        self.store_slot(reg, val);
        if let Some(&flag) = self.pending_nan.get(&reg) {
            self.ir.emit_store(flag, self.const_u32_0);
        }
    }

    fn load_predicate(&mut self) -> u32 {
        self.load_slot(MAX_REG_COUNT as u32)
    }

    fn store_predicate(&mut self, val: u32) {
        self.store_slot(MAX_REG_COUNT as u32, val)
    }

    pub fn finish(&mut self) {
    }
}

include!("../sm86_decoder_generated.rs");
