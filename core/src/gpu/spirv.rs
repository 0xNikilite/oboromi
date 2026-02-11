//! SPIR-V binary module emitter.
//!
//! Emits a valid SPIR-V module word-by-word. All IDs are 1-based.
//! The caller must emit instructions in valid SPIR-V layout order:
//!
//!   1. `emit_header`
//!   2. `emit_capability`
//!   3. `emit_ext_inst_import`
//!   4. `emit_memory_model`
//!   5. `emit_entry_point`
//!   6. `emit_execution_mode`
//!   7. `emit_name` / `emit_member_name`
//!   8. `emit_decorate` / `emit_member_decorate`
//!   9. Type / Constant / Global Variable declarations
//!  10. Function definitions (function → label → body → return → function_end)
//!
//! Call `finalize()` after all emission to patch the ID bound in the header.

use std::iter;

// ════════════════════════════════════════════════════════════════════
//  Literal encoding trait
// ════════════════════════════════════════════════════════════════════

pub trait Literal {
    type Words: Iterator<Item = u32>;
    fn to_words(self) -> Self::Words;
}

impl Literal for u8 {
    type Words = iter::Once<u32>;
    fn to_words(self) -> Self::Words { iter::once(self as u32) }
}
impl Literal for i8 {
    type Words = iter::Once<u32>;
    fn to_words(self) -> Self::Words { iter::once(self as u8 as u32) }
}
impl Literal for u16 {
    type Words = iter::Once<u32>;
    fn to_words(self) -> Self::Words { iter::once(self as u32) }
}
impl Literal for i16 {
    type Words = iter::Once<u32>;
    fn to_words(self) -> Self::Words { iter::once(self as u16 as u32) }
}
impl Literal for u32 {
    type Words = iter::Once<u32>;
    fn to_words(self) -> Self::Words { iter::once(self) }
}
impl Literal for i32 {
    type Words = iter::Once<u32>;
    fn to_words(self) -> Self::Words { iter::once(self as u32) }
}
impl Literal for f32 {
    type Words = iter::Once<u32>;
    fn to_words(self) -> Self::Words { iter::once(self.to_bits()) }
}
impl Literal for u64 {
    type Words = std::array::IntoIter<u32, 2>;
    fn to_words(self) -> Self::Words { [self as u32, (self >> 32) as u32].into_iter() }
}
impl Literal for i64 {
    type Words = std::array::IntoIter<u32, 2>;
    fn to_words(self) -> Self::Words { (self as u64).to_words() }
}
impl Literal for f64 {
    type Words = std::array::IntoIter<u32, 2>;
    fn to_words(self) -> Self::Words { self.to_bits().to_words() }
}

// ════════════════════════════════════════════════════════════════════
//  String encoding helper
// ════════════════════════════════════════════════════════════════════

fn encode_string(buf: &mut Vec<u32>, s: &str) {
    let bytes = s.as_bytes();
    let mut word = 0u32;
    for (i, &b) in bytes.iter().enumerate() {
        word |= (b as u32) << ((i % 4) * 8);
        if i % 4 == 3 {
            buf.push(word);
            word = 0;
        }
    }
    buf.push(word); // final word includes null terminator (zero-padded)
}

// ════════════════════════════════════════════════════════════════════
//  SPIR-V constants
// ════════════════════════════════════════════════════════════════════

pub mod capability {
    pub const SHADER: u32 = 1;
    pub const FLOAT16: u32 = 9;
    pub const FLOAT64: u32 = 10;
    pub const INT64: u32 = 11;
    pub const INT16: u32 = 22;
    pub const INT8: u32 = 39;
    pub const STORAGE_BUFFER_16BIT: u32 = 4433;
    pub const STORAGE_BUFFER_8BIT: u32 = 4448;
    pub const PHYSICAL_STORAGE_BUFFER: u32 = 5347;
    pub const VARIABLE_POINTERS: u32 = 4442;
}

pub mod storage_class {
    pub const UNIFORM_CONSTANT: u32 = 0;
    pub const INPUT: u32 = 1;
    pub const UNIFORM: u32 = 2;
    pub const OUTPUT: u32 = 3;
    pub const WORKGROUP: u32 = 4;
    pub const CROSS_WORKGROUP: u32 = 5;
    pub const PRIVATE: u32 = 6;
    pub const FUNCTION: u32 = 7;
    pub const PUSH_CONSTANT: u32 = 9;
    pub const STORAGE_BUFFER: u32 = 12;
    pub const PHYSICAL_STORAGE_BUFFER: u32 = 5349;
}

pub mod execution_model {
    pub const VERTEX: u32 = 0;
    pub const FRAGMENT: u32 = 4;
    pub const GLCOMPUTE: u32 = 5;
}

pub mod execution_mode {
    pub const LOCAL_SIZE: u32 = 17;
    pub const ORIGIN_UPPER_LEFT: u32 = 7;
}

pub mod decoration {
    pub const BLOCK: u32 = 2;
    pub const BUFFER_BLOCK: u32 = 3;
    pub const ROW_MAJOR: u32 = 4;
    pub const COL_MAJOR: u32 = 5;
    pub const ARRAY_STRIDE: u32 = 6;
    pub const MATRIX_STRIDE: u32 = 7;
    pub const BUILTIN: u32 = 11;
    pub const NO_PERSPECTIVE: u32 = 13;
    pub const FLAT: u32 = 14;
    pub const NON_WRITABLE: u32 = 24;
    pub const NON_READABLE: u32 = 25;
    pub const LOCATION: u32 = 30;
    pub const BINDING: u32 = 33;
    pub const DESCRIPTOR_SET: u32 = 34;
    pub const OFFSET: u32 = 35;
}

#[derive(Debug, Clone, Copy)]
pub enum BuiltIn {
    Position = 0,
    PointSize = 1,
    VertexIndex = 42,
    InstanceIndex = 43,
    FragCoord = 15,
    FrontFacing = 17,
    FragDepth = 22,
    NumWorkgroups = 24,
    WorkgroupSize = 25,
    WorkgroupId = 26,
    LocalInvocationId = 27,
    GlobalInvocationId = 28,
    LocalInvocationIndex = 29,
    SubgroupSize = 36,
    SubgroupLocalInvocationId = 41,
}

// ════════════════════════════════════════════════════════════════════
//  Emitter
// ════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct Emitter {
    words: Vec<u32>,
    next_id: u32,
    bound_idx: usize,
}

impl Default for Emitter {
    fn default() -> Self { Self::new() }
}

impl Emitter {
    pub fn new() -> Self {
        Self {
            words: Vec::with_capacity(4096),
            next_id: 1, // SPIR-V IDs are 1-based; 0 is reserved
            bound_idx: 0,
        }
    }

    /// Allocate a fresh SPIR-V result ID.
    #[inline]
    pub fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Access the emitted word stream.
    pub fn words(&self) -> &[u32] { &self.words }

    /// Total words emitted so far.
    pub fn len(&self) -> usize { self.words.len() }

    /// Number of IDs allocated.
    pub fn id_bound(&self) -> u32 { self.next_id }

    // ── Internal instruction emitter ────────────────────────────

    /// Emit a single SPIR-V instruction: `(word_count << 16 | opcode)` followed by operands.
    fn inst(&mut self, opcode: u32, operands: &[u32]) {
        let wc = (1 + operands.len()) as u32;
        self.words.push((wc << 16) | (opcode & 0xFFFF));
        self.words.extend_from_slice(operands);
    }

    /// Typed unary: result = op(a). Returns result ID.
    fn typed_un(&mut self, op: u32, ty: u32, a: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(op, &[ty, r, a]);
        r
    }

    /// Typed binary: result = op(a, b). Returns result ID.
    fn typed_bin(&mut self, op: u32, ty: u32, a: u32, b: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(op, &[ty, r, a, b]);
        r
    }

    /// Typed ternary: result = op(a, b, c). Returns result ID.
    fn typed_tri(&mut self, op: u32, ty: u32, a: u32, b: u32, c: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(op, &[ty, r, a, b, c]);
        r
    }

    // ════════════════════════════════════════════════════════════
    //  Header
    // ════════════════════════════════════════════════════════════

    /// Emit the 5-word SPIR-V module header. Must be called first.
    /// Version defaults to SPIR-V 1.5.
    pub fn emit_header(&mut self) {
        self.words.push(0x07230203);    // magic
        self.words.push(0x00010500);    // version 1.5
        self.words.push(0);             // generator (unregistered)
        self.bound_idx = self.words.len();
        self.words.push(0);             // bound (patched by finalize)
        self.words.push(0);             // schema
    }

    /// Patch the ID bound in the header. Call after all emission is done.
    pub fn finalize(&mut self) {
        if self.bound_idx < self.words.len() {
            self.words[self.bound_idx] = self.next_id;
        }
    }

    // ════════════════════════════════════════════════════════════
    //  Capability / Extension / Mode  (opcodes 11, 14-17)
    // ════════════════════════════════════════════════════════════

    pub fn emit_capability(&mut self, cap: u32) {
        self.inst(17, &[cap]);
    }

    pub fn emit_extension(&mut self, name: &str) {
        let mut data = Vec::new();
        encode_string(&mut data, name);
        self.inst(10, &data);
    }

    pub fn emit_ext_inst_import(&mut self, name: &str) -> u32 {
        let r = self.alloc_id();
        let mut data = vec![r];
        encode_string(&mut data, name);
        self.inst(11, &data);
        r
    }

    pub fn emit_memory_model(&mut self, addressing: u32, memory: u32) {
        self.inst(14, &[addressing, memory]);
    }

    pub fn emit_entry_point(&mut self, exec_model: u32, func: u32, name: &str, interfaces: &[u32]) {
        let mut data = vec![exec_model, func];
        encode_string(&mut data, name);
        data.extend_from_slice(interfaces);
        self.inst(15, &data);
    }

    pub fn emit_execution_mode(&mut self, entry: u32, mode: u32, literals: &[u32]) {
        let mut data = vec![entry, mode];
        data.extend_from_slice(literals);
        self.inst(16, &data);
    }

    // ════════════════════════════════════════════════════════════
    //  Debug  (opcodes 5-6)
    // ════════════════════════════════════════════════════════════

    pub fn emit_name(&mut self, target: u32, name: &str) {
        let mut data = vec![target];
        encode_string(&mut data, name);
        self.inst(5, &data);
    }

    pub fn emit_member_name(&mut self, ty: u32, member: u32, name: &str) {
        let mut data = vec![ty, member];
        encode_string(&mut data, name);
        self.inst(6, &data);
    }

    // ════════════════════════════════════════════════════════════
    //  Annotations  (opcodes 71-74)
    // ════════════════════════════════════════════════════════════

    pub fn emit_decorate(&mut self, target: u32, deco: u32, literals: &[u32]) {
        let mut data = vec![target, deco];
        data.extend_from_slice(literals);
        self.inst(71, &data);
    }

    pub fn emit_member_decorate(&mut self, struct_type: u32, member: u32, deco: u32, literals: &[u32]) {
        let mut data = vec![struct_type, member, deco];
        data.extend_from_slice(literals);
        self.inst(72, &data);
    }

    pub fn emit_decoration_group(&mut self) -> u32 {
        let r = self.alloc_id();
        self.inst(73, &[r]);
        r
    }

    pub fn emit_group_decorate(&mut self, group: u32, targets: &[u32]) {
        let mut data = vec![group];
        data.extend_from_slice(targets);
        self.inst(74, &data);
    }

    // ════════════════════════════════════════════════════════════
    //  Types  (opcodes 19-33)
    // ════════════════════════════════════════════════════════════

    pub fn emit_type_void(&mut self) -> u32 {
        let r = self.alloc_id();
        self.inst(19, &[r]);
        r
    }

    pub fn emit_type_bool(&mut self) -> u32 {
        let r = self.alloc_id();
        self.inst(20, &[r]);
        r
    }

    pub fn emit_type_int(&mut self, width: u32, sign: u32) -> u32 {
        debug_assert!(width == 8 || width == 16 || width == 32 || width == 64);
        debug_assert!(sign <= 1);
        let r = self.alloc_id();
        self.inst(21, &[r, width, sign]);
        r
    }

    pub fn emit_type_float(&mut self, width: u32) -> u32 {
        debug_assert!(width == 16 || width == 32 || width == 64);
        let r = self.alloc_id();
        self.inst(22, &[r, width]);
        r
    }

    pub fn emit_type_vector(&mut self, component_type: u32, count: u32) -> u32 {
        debug_assert!(count >= 2 && count <= 16);
        let r = self.alloc_id();
        self.inst(23, &[r, component_type, count]);
        r
    }

    pub fn emit_type_matrix(&mut self, column_type: u32, columns: u32) -> u32 {
        debug_assert!(columns >= 2);
        let r = self.alloc_id();
        self.inst(24, &[r, column_type, columns]);
        r
    }

    pub fn emit_type_image(
        &mut self, sampled_type: u32, dim: u32, depth: u32,
        arrayed: u32, ms: u32, sampled: u32, format: u32,
    ) -> u32 {
        let r = self.alloc_id();
        self.inst(25, &[r, sampled_type, dim, depth, arrayed, ms, sampled, format]);
        r
    }

    pub fn emit_type_sampler(&mut self) -> u32 {
        let r = self.alloc_id();
        self.inst(26, &[r]);
        r
    }

    pub fn emit_type_sampled_image(&mut self, image_type: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(27, &[r, image_type]);
        r
    }

    pub fn emit_type_array(&mut self, element_type: u32, length: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(28, &[r, element_type, length]);
        r
    }

    pub fn emit_type_runtime_array(&mut self, element_type: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(29, &[r, element_type]);
        r
    }

    pub fn emit_type_struct(&mut self, members: &[u32]) -> u32 {
        let r = self.alloc_id();
        let mut data = vec![r];
        data.extend_from_slice(members);
        self.inst(30, &data);
        r
    }

    pub fn emit_type_pointer(&mut self, storage_class: u32, pointee_type: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(32, &[r, storage_class, pointee_type]);
        r
    }

    pub fn emit_type_function(&mut self, return_type: u32, params: &[u32]) -> u32 {
        let r = self.alloc_id();
        let mut data = vec![r, return_type];
        data.extend_from_slice(params);
        self.inst(33, &data);
        r
    }

    // ════════════════════════════════════════════════════════════
    //  Constants  (opcodes 41-44, 48)
    // ════════════════════════════════════════════════════════════

    pub fn emit_constant_true(&mut self, ty: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(41, &[ty, r]);
        r
    }

    pub fn emit_constant_false(&mut self, ty: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(42, &[ty, r]);
        r
    }

    pub fn emit_constant(&mut self, ty: u32, value_words: &[u32]) -> u32 {
        let r = self.alloc_id();
        let mut data = vec![ty, r];
        data.extend_from_slice(value_words);
        self.inst(43, &data);
        r
    }

    pub fn emit_constant_typed<T: Literal>(&mut self, ty: u32, value: T) -> u32 {
        let r = self.alloc_id();
        let mut data = vec![ty, r];
        data.extend(value.to_words());
        self.inst(43, &data);
        r
    }

    pub fn emit_constant_composite(&mut self, ty: u32, constituents: &[u32]) -> u32 {
        let r = self.alloc_id();
        let mut data = vec![ty, r];
        data.extend_from_slice(constituents);
        self.inst(44, &data);
        r
    }

    pub fn emit_spec_constant(&mut self, ty: u32, value_words: &[u32]) -> u32 {
        let r = self.alloc_id();
        let mut data = vec![ty, r];
        data.extend_from_slice(value_words);
        self.inst(48, &data);
        r
    }

    // ════════════════════════════════════════════════════════════
    //  Memory  (opcodes 59, 61-62, 65-66)
    // ════════════════════════════════════════════════════════════

    pub fn emit_variable(&mut self, ty: u32, storage_class: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(59, &[ty, r, storage_class]);
        r
    }

    pub fn emit_variable_init(&mut self, ty: u32, storage_class: u32, initializer: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(59, &[ty, r, storage_class, initializer]);
        r
    }

    pub fn emit_load(&mut self, ty: u32, pointer: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(61, &[ty, r, pointer]);
        r
    }

    pub fn emit_store(&mut self, pointer: u32, value: u32) {
        self.inst(62, &[pointer, value]);
    }

    pub fn emit_access_chain(&mut self, ty: u32, base: u32, indexes: &[u32]) -> u32 {
        let r = self.alloc_id();
        let mut data = vec![ty, r, base];
        data.extend_from_slice(indexes);
        self.inst(65, &data);
        r
    }

    pub fn emit_in_bounds_access_chain(&mut self, ty: u32, base: u32, indexes: &[u32]) -> u32 {
        let r = self.alloc_id();
        let mut data = vec![ty, r, base];
        data.extend_from_slice(indexes);
        self.inst(66, &data);
        r
    }

    pub fn emit_copy_memory(&mut self, target: u32, source: u32) {
        self.inst(63, &[target, source]);
    }

    // ════════════════════════════════════════════════════════════
    //  Functions  (opcodes 54-57)
    // ════════════════════════════════════════════════════════════

    pub fn emit_function(&mut self, return_type: u32, control: u32, func_type: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(54, &[return_type, r, control, func_type]);
        r
    }

    pub fn emit_function_parameter(&mut self, ty: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(55, &[ty, r]);
        r
    }

    pub fn emit_function_end(&mut self) {
        self.inst(56, &[]);
    }

    pub fn emit_function_call(&mut self, result_type: u32, function: u32, args: &[u32]) -> u32 {
        let r = self.alloc_id();
        let mut data = vec![result_type, r, function];
        data.extend_from_slice(args);
        self.inst(57, &data);
        r
    }

    // ════════════════════════════════════════════════════════════
    //  Composite  (opcodes 79-83)
    // ════════════════════════════════════════════════════════════

    pub fn emit_vector_shuffle(&mut self, ty: u32, v1: u32, v2: u32, components: &[u32]) -> u32 {
        let r = self.alloc_id();
        let mut data = vec![ty, r, v1, v2];
        data.extend_from_slice(components);
        self.inst(79, &data);
        r
    }

    pub fn emit_composite_construct(&mut self, ty: u32, constituents: &[u32]) -> u32 {
        let r = self.alloc_id();
        let mut data = vec![ty, r];
        data.extend_from_slice(constituents);
        self.inst(80, &data);
        r
    }

    pub fn emit_composite_extract(&mut self, ty: u32, composite: u32, indexes: &[u32]) -> u32 {
        let r = self.alloc_id();
        let mut data = vec![ty, r, composite];
        data.extend_from_slice(indexes);
        self.inst(81, &data);
        r
    }

    pub fn emit_composite_insert(&mut self, ty: u32, object: u32, composite: u32, indexes: &[u32]) -> u32 {
        let r = self.alloc_id();
        let mut data = vec![ty, r, object, composite];
        data.extend_from_slice(indexes);
        self.inst(82, &data);
        r
    }

    pub fn emit_copy_object(&mut self, ty: u32, operand: u32) -> u32 {
        self.typed_un(83, ty, operand)
    }

    // ════════════════════════════════════════════════════════════
    //  Conversion  (opcodes 109-115, 124)
    // ════════════════════════════════════════════════════════════

    /// float → unsigned int
    pub fn emit_convert_f_to_u(&mut self, ty: u32, val: u32) -> u32 { self.typed_un(109, ty, val) }
    /// float → signed int
    pub fn emit_convert_f_to_s(&mut self, ty: u32, val: u32) -> u32 { self.typed_un(110, ty, val) }
    /// signed int → float
    pub fn emit_convert_s_to_f(&mut self, ty: u32, val: u32) -> u32 { self.typed_un(111, ty, val) }
    /// unsigned int → float
    pub fn emit_convert_u_to_f(&mut self, ty: u32, val: u32) -> u32 { self.typed_un(112, ty, val) }
    /// unsigned int width change
    pub fn emit_u_convert(&mut self, ty: u32, val: u32) -> u32 { self.typed_un(113, ty, val) }
    /// signed int width change
    pub fn emit_s_convert(&mut self, ty: u32, val: u32) -> u32 { self.typed_un(114, ty, val) }
    /// float width change
    pub fn emit_f_convert(&mut self, ty: u32, val: u32) -> u32 { self.typed_un(115, ty, val) }
    /// bit-preserving type cast
    pub fn emit_bitcast(&mut self, ty: u32, val: u32) -> u32 { self.typed_un(124, ty, val) }

    // ════════════════════════════════════════════════════════════
    //  Arithmetic  (opcodes 126-141)
    // ════════════════════════════════════════════════════════════

    pub fn emit_snegate(&mut self, ty: u32, a: u32) -> u32 { self.typed_un(126, ty, a) }
    pub fn emit_fnegate(&mut self, ty: u32, a: u32) -> u32 { self.typed_un(127, ty, a) }
    pub fn emit_iadd(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(128, ty, a, b) }
    pub fn emit_fadd(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(129, ty, a, b) }
    pub fn emit_isub(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(130, ty, a, b) }
    pub fn emit_fsub(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(131, ty, a, b) }
    pub fn emit_imul(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(132, ty, a, b) }
    pub fn emit_fmul(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(133, ty, a, b) }
    pub fn emit_udiv(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(134, ty, a, b) }
    pub fn emit_sdiv(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(135, ty, a, b) }
    pub fn emit_fdiv(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(136, ty, a, b) }
    pub fn emit_umod(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(137, ty, a, b) }
    pub fn emit_srem(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(138, ty, a, b) }
    pub fn emit_smod(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(139, ty, a, b) }
    pub fn emit_frem(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(140, ty, a, b) }
    pub fn emit_fmod(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(141, ty, a, b) }

    // ════════════════════════════════════════════════════════════
    //  Logical  (opcodes 164-169)
    // ════════════════════════════════════════════════════════════

    pub fn emit_logical_equal(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(164, ty, a, b) }
    pub fn emit_logical_not_equal(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(165, ty, a, b) }
    pub fn emit_logical_or(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(166, ty, a, b) }
    pub fn emit_logical_and(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(167, ty, a, b) }
    pub fn emit_logical_not(&mut self, ty: u32, a: u32) -> u32 { self.typed_un(168, ty, a) }
    pub fn emit_select(&mut self, ty: u32, cond: u32, a: u32, b: u32) -> u32 { self.typed_tri(169, ty, cond, a, b) }

    // ════════════════════════════════════════════════════════════
    //  Integer comparison  (opcodes 170-179)
    // ════════════════════════════════════════════════════════════

    pub fn emit_iequal(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(170, ty, a, b) }
    pub fn emit_inot_equal(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(171, ty, a, b) }
    pub fn emit_ugreater_than(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(172, ty, a, b) }
    pub fn emit_sgreater_than(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(173, ty, a, b) }
    pub fn emit_ugreater_than_equal(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(174, ty, a, b) }
    pub fn emit_sgreater_than_equal(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(175, ty, a, b) }
    pub fn emit_uless_than(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(176, ty, a, b) }
    pub fn emit_sless_than(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(177, ty, a, b) }
    pub fn emit_uless_than_equal(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(178, ty, a, b) }
    pub fn emit_sless_than_equal(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(179, ty, a, b) }

    // ════════════════════════════════════════════════════════════
    //  Float comparison  (opcodes 180-191)
    // ════════════════════════════════════════════════════════════

    pub fn emit_ford_equal(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(180, ty, a, b) }
    pub fn emit_funord_equal(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(181, ty, a, b) }
    pub fn emit_ford_not_equal(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(182, ty, a, b) }
    pub fn emit_funord_not_equal(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(183, ty, a, b) }
    pub fn emit_ford_less_than(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(184, ty, a, b) }
    pub fn emit_funord_less_than(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(185, ty, a, b) }
    pub fn emit_ford_greater_than(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(186, ty, a, b) }
    pub fn emit_funord_greater_than(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(187, ty, a, b) }
    pub fn emit_ford_less_than_equal(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(188, ty, a, b) }
    pub fn emit_funord_less_than_equal(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(189, ty, a, b) }
    pub fn emit_ford_greater_than_equal(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(190, ty, a, b) }
    pub fn emit_funord_greater_than_equal(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(191, ty, a, b) }
    pub fn emit_f_is_nan(&mut self, ty: u32, a: u32) -> u32 { self.typed_un(156, ty, a) }
    pub fn emit_f_is_inf(&mut self, ty: u32, a: u32) -> u32 { self.typed_un(157, ty, a) }
    pub fn emit_is_nan(&mut self, ty: u32, a: u32) -> u32 { self.typed_un(156, ty, a) }
    pub fn emit_is_inf(&mut self, ty: u32, a: u32) -> u32 { self.typed_un(157, ty, a) }

    // ════════════════════════════════════════════════════════════
    //  Bitwise & Shifts  (opcodes 194-205)
    // ════════════════════════════════════════════════════════════

    pub fn emit_shift_right_logical(&mut self, ty: u32, base: u32, shift: u32) -> u32 { self.typed_bin(194, ty, base, shift) }
    pub fn emit_shift_right_arithmetic(&mut self, ty: u32, base: u32, shift: u32) -> u32 { self.typed_bin(195, ty, base, shift) }
    pub fn emit_shift_left_logical(&mut self, ty: u32, base: u32, shift: u32) -> u32 { self.typed_bin(196, ty, base, shift) }
    pub fn emit_bitwise_or(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(197, ty, a, b) }
    pub fn emit_bitwise_xor(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(198, ty, a, b) }
    pub fn emit_bitwise_and(&mut self, ty: u32, a: u32, b: u32) -> u32 { self.typed_bin(199, ty, a, b) }
    pub fn emit_not(&mut self, ty: u32, a: u32) -> u32 { self.typed_un(200, ty, a) }

    pub fn emit_bit_field_insert(&mut self, ty: u32, base: u32, insert: u32, offset: u32, count: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(201, &[ty, r, base, insert, offset, count]);
        r
    }
    pub fn emit_bit_field_s_extract(&mut self, ty: u32, base: u32, offset: u32, count: u32) -> u32 { self.typed_tri(202, ty, base, offset, count) }
    pub fn emit_bit_field_u_extract(&mut self, ty: u32, base: u32, offset: u32, count: u32) -> u32 { self.typed_tri(203, ty, base, offset, count) }
    pub fn emit_bit_reverse(&mut self, ty: u32, a: u32) -> u32 { self.typed_un(204, ty, a) }
    pub fn emit_bit_count(&mut self, ty: u32, a: u32) -> u32 { self.typed_un(205, ty, a) }

    // ════════════════════════════════════════════════════════════
    //  Control flow  (opcodes 245-255)
    // ════════════════════════════════════════════════════════════

    pub fn emit_phi(&mut self, ty: u32, sources: &[(u32, u32)]) -> u32 {
        let r = self.alloc_id();
        let mut data = vec![ty, r];
        for &(variable, parent) in sources {
            data.push(variable);
            data.push(parent);
        }
        self.inst(245, &data);
        r
    }

    pub fn emit_loop_merge(&mut self, merge_block: u32, continue_target: u32, control: u32) {
        self.inst(246, &[merge_block, continue_target, control]);
    }

    pub fn emit_selection_merge(&mut self, merge_block: u32, control: u32) {
        self.inst(247, &[merge_block, control]);
    }

    pub fn emit_label(&mut self) -> u32 {
        let r = self.alloc_id();
        self.inst(248, &[r]);
        r
    }

    pub fn emit_branch(&mut self, target: u32) {
        self.inst(249, &[target]);
    }

    pub fn emit_branch_conditional(&mut self, cond: u32, true_label: u32, false_label: u32) {
        self.inst(250, &[cond, true_label, false_label]);
    }

    pub fn emit_switch(&mut self, selector: u32, default: u32, targets: &[(u32, u32)]) {
        let mut data = vec![selector, default];
        for &(literal, label) in targets {
            data.push(literal);
            data.push(label);
        }
        self.inst(251, &data);
    }

    pub fn emit_return(&mut self) {
        self.inst(253, &[]);
    }

    pub fn emit_return_value(&mut self, value: u32) {
        self.inst(254, &[value]);
    }

    pub fn emit_unreachable(&mut self) {
        self.inst(255, &[]);
    }

    // ════════════════════════════════════════════════════════════
    //  Barrier  (opcodes 224-225)
    // ════════════════════════════════════════════════════════════

    /// `execution` and `memory` are Scope IDs (constants), `semantics` is MemorySemantics.
    pub fn emit_control_barrier(&mut self, execution: u32, memory: u32, semantics: u32) {
        self.inst(224, &[execution, memory, semantics]);
    }

    pub fn emit_memory_barrier(&mut self, memory: u32, semantics: u32) {
        self.inst(225, &[memory, semantics]);
    }

    // ════════════════════════════════════════════════════════════
    //  Extended instruction (GLSL.std.450 etc.)  (opcode 12)
    // ════════════════════════════════════════════════════════════

    pub fn emit_ext_inst(&mut self, ty: u32, set: u32, instruction: u32, operands: &[u32]) -> u32 {
        let r = self.alloc_id();
        let mut data = vec![ty, r, set, instruction];
        data.extend_from_slice(operands);
        self.inst(12, &data);
        r
    }

    // ════════════════════════════════════════════════════════════
    //  Image  (opcodes 86-100)
    // ════════════════════════════════════════════════════════════

    pub fn emit_sampled_image(&mut self, ty: u32, image: u32, sampler: u32) -> u32 {
        self.typed_bin(86, ty, image, sampler)
    }

    pub fn emit_image_sample_implicit_lod(&mut self, ty: u32, sampled_image: u32, coord: u32, operands: &[u32]) -> u32 {
        let r = self.alloc_id();
        let mut data = vec![ty, r, sampled_image, coord];
        data.extend_from_slice(operands);
        self.inst(87, &data);
        r
    }

    pub fn emit_image_sample_explicit_lod(&mut self, ty: u32, sampled_image: u32, coord: u32, image_ops: u32, operands: &[u32]) -> u32 {
        let r = self.alloc_id();
        let mut data = vec![ty, r, sampled_image, coord, image_ops];
        data.extend_from_slice(operands);
        self.inst(88, &data);
        r
    }

    pub fn emit_image_fetch(&mut self, ty: u32, image: u32, coord: u32, operands: &[u32]) -> u32 {
        let r = self.alloc_id();
        let mut data = vec![ty, r, image, coord];
        data.extend_from_slice(operands);
        self.inst(95, &data);
        r
    }

    pub fn emit_image_read(&mut self, ty: u32, image: u32, coord: u32, operands: &[u32]) -> u32 {
        let r = self.alloc_id();
        let mut data = vec![ty, r, image, coord];
        data.extend_from_slice(operands);
        self.inst(98, &data);
        r
    }

    pub fn emit_image_write(&mut self, image: u32, coord: u32, texel: u32, operands: &[u32]) {
        let mut data = vec![image, coord, texel];
        data.extend_from_slice(operands);
        self.inst(99, &data);
    }

    pub fn emit_image(&mut self, ty: u32, sampled_image: u32) -> u32 {
        self.typed_un(100, ty, sampled_image)
    }

    pub fn emit_image_query_size_lod(&mut self, ty: u32, image: u32, lod: u32) -> u32 {
        self.typed_bin(103, ty, image, lod)
    }

    pub fn emit_image_query_size(&mut self, ty: u32, image: u32) -> u32 {
        self.typed_un(104, ty, image)
    }

    // ════════════════════════════════════════════════════════════
    //  Atomic  (opcodes 227-242)
    // ════════════════════════════════════════════════════════════

    pub fn emit_atomic_load(&mut self, ty: u32, pointer: u32, scope: u32, semantics: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(227, &[ty, r, pointer, scope, semantics]);
        r
    }

    pub fn emit_atomic_store(&mut self, pointer: u32, scope: u32, semantics: u32, value: u32) {
        self.inst(228, &[pointer, scope, semantics, value]);
    }

    pub fn emit_atomic_exchange(&mut self, ty: u32, pointer: u32, scope: u32, semantics: u32, value: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(229, &[ty, r, pointer, scope, semantics, value]);
        r
    }

    pub fn emit_atomic_compare_exchange(&mut self, ty: u32, pointer: u32, scope: u32, equal_sem: u32, unequal_sem: u32, value: u32, comparator: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(230, &[ty, r, pointer, scope, equal_sem, unequal_sem, value, comparator]);
        r
    }

    pub fn emit_atomic_iadd(&mut self, ty: u32, pointer: u32, scope: u32, semantics: u32, value: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(234, &[ty, r, pointer, scope, semantics, value]);
        r
    }

    pub fn emit_atomic_isub(&mut self, ty: u32, pointer: u32, scope: u32, semantics: u32, value: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(235, &[ty, r, pointer, scope, semantics, value]);
        r
    }

    pub fn emit_atomic_smin(&mut self, ty: u32, pointer: u32, scope: u32, semantics: u32, value: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(236, &[ty, r, pointer, scope, semantics, value]);
        r
    }

    pub fn emit_atomic_umin(&mut self, ty: u32, pointer: u32, scope: u32, semantics: u32, value: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(237, &[ty, r, pointer, scope, semantics, value]);
        r
    }

    pub fn emit_atomic_smax(&mut self, ty: u32, pointer: u32, scope: u32, semantics: u32, value: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(238, &[ty, r, pointer, scope, semantics, value]);
        r
    }

    pub fn emit_atomic_umax(&mut self, ty: u32, pointer: u32, scope: u32, semantics: u32, value: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(239, &[ty, r, pointer, scope, semantics, value]);
        r
    }

    pub fn emit_atomic_and(&mut self, ty: u32, pointer: u32, scope: u32, semantics: u32, value: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(240, &[ty, r, pointer, scope, semantics, value]);
        r
    }

    pub fn emit_atomic_or(&mut self, ty: u32, pointer: u32, scope: u32, semantics: u32, value: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(241, &[ty, r, pointer, scope, semantics, value]);
        r
    }

    pub fn emit_atomic_xor(&mut self, ty: u32, pointer: u32, scope: u32, semantics: u32, value: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(242, &[ty, r, pointer, scope, semantics, value]);
        r
    }

    // ════════════════════════════════════════════════════════════
    //  Derivative  (opcodes 207-213)
    // ════════════════════════════════════════════════════════════

    pub fn emit_dpdx(&mut self, ty: u32, p: u32) -> u32 { self.typed_un(207, ty, p) }
    pub fn emit_dpdy(&mut self, ty: u32, p: u32) -> u32 { self.typed_un(208, ty, p) }
    pub fn emit_fwidth(&mut self, ty: u32, p: u32) -> u32 { self.typed_un(209, ty, p) }
    pub fn emit_dpdx_fine(&mut self, ty: u32, p: u32) -> u32 { self.typed_un(210, ty, p) }
    pub fn emit_dpdy_fine(&mut self, ty: u32, p: u32) -> u32 { self.typed_un(211, ty, p) }
    pub fn emit_fwidth_fine(&mut self, ty: u32, p: u32) -> u32 { self.typed_un(212, ty, p) }
    pub fn emit_dpdx_coarse(&mut self, ty: u32, p: u32) -> u32 { self.typed_un(213, ty, p) }
    pub fn emit_dpdy_coarse(&mut self, ty: u32, p: u32) -> u32 { self.typed_un(214, ty, p) }
    pub fn emit_fwidth_coarse(&mut self, ty: u32, p: u32) -> u32 { self.typed_un(215, ty, p) }

    // ════════════════════════════════════════════════════════════
    //  Group / Subgroup  (opcodes 261-267, 337-345)
    // ════════════════════════════════════════════════════════════

    pub fn emit_group_non_uniform_elect(&mut self, ty: u32, scope: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(333, &[ty, r, scope]);
        r
    }

    pub fn emit_group_non_uniform_broadcast(&mut self, ty: u32, scope: u32, value: u32, id: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(337, &[ty, r, scope, value, id]);
        r
    }

    pub fn emit_group_non_uniform_broadcast_first(&mut self, ty: u32, scope: u32, value: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(338, &[ty, r, scope, value]);
        r
    }

    pub fn emit_group_non_uniform_ballot(&mut self, ty: u32, scope: u32, predicate: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(339, &[ty, r, scope, predicate]);
        r
    }

    pub fn emit_group_non_uniform_shuffle(&mut self, ty: u32, scope: u32, value: u32, id: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(345, &[ty, r, scope, value, id]);
        r
    }

    pub fn emit_group_non_uniform_shuffle_xor(&mut self, ty: u32, scope: u32, value: u32, mask: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(346, &[ty, r, scope, value, mask]);
        r
    }

    pub fn emit_group_non_uniform_iadd(&mut self, ty: u32, scope: u32, op: u32, value: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(349, &[ty, r, scope, op, value]);
        r
    }

    pub fn emit_group_non_uniform_fadd(&mut self, ty: u32, scope: u32, op: u32, value: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(350, &[ty, r, scope, op, value]);
        r
    }

    pub fn emit_group_non_uniform_smin(&mut self, ty: u32, scope: u32, op: u32, value: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(353, &[ty, r, scope, op, value]);
        r
    }

    pub fn emit_group_non_uniform_umin(&mut self, ty: u32, scope: u32, op: u32, value: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(354, &[ty, r, scope, op, value]);
        r
    }

    pub fn emit_group_non_uniform_fmin(&mut self, ty: u32, scope: u32, op: u32, value: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(355, &[ty, r, scope, op, value]);
        r
    }

    pub fn emit_group_non_uniform_smax(&mut self, ty: u32, scope: u32, op: u32, value: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(356, &[ty, r, scope, op, value]);
        r
    }

    pub fn emit_group_non_uniform_umax(&mut self, ty: u32, scope: u32, op: u32, value: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(357, &[ty, r, scope, op, value]);
        r
    }

    pub fn emit_group_non_uniform_fmax(&mut self, ty: u32, scope: u32, op: u32, value: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(358, &[ty, r, scope, op, value]);
        r
    }

    pub fn emit_group_non_uniform_bitwise_and(&mut self, ty: u32, scope: u32, op: u32, value: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(359, &[ty, r, scope, op, value]);
        r
    }

    pub fn emit_group_non_uniform_bitwise_or(&mut self, ty: u32, scope: u32, op: u32, value: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(360, &[ty, r, scope, op, value]);
        r
    }

    pub fn emit_group_non_uniform_bitwise_xor(&mut self, ty: u32, scope: u32, op: u32, value: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(361, &[ty, r, scope, op, value]);
        r
    }

    // ════════════════════════════════════════════════════════════
    //  Miscellaneous
    // ════════════════════════════════════════════════════════════

    pub fn emit_nop(&mut self) {
        self.inst(0, &[]);
    }

    pub fn emit_undef(&mut self, ty: u32) -> u32 {
        let r = self.alloc_id();
        self.inst(1, &[ty, r]);
        r
    }

    pub fn emit_kill(&mut self) {
        self.inst(252, &[]);
    }

    // ════════════════════════════════════════════════════════════
    //  Validation
    // ════════════════════════════════════════════════════════════

    /// Basic structural validation of the emitted module.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.words.len() < 5 {
            return Err("Module too short for valid header");
        }
        if self.words[0] != 0x07230203 {
            return Err("Invalid SPIR-V magic number");
        }
        // Walk instructions after header
        let mut pos = 5;
        while pos < self.words.len() {
            let header = self.words[pos];
            let wc = (header >> 16) as usize;
            let opcode = header & 0xFFFF;
            if wc == 0 {
                return Err("Instruction with zero word count");
            }
            if pos + wc > self.words.len() {
                return Err("Instruction extends beyond module end");
            }
            // OpNop is valid with wc=1
            if opcode == 0 && wc != 1 {
                return Err("OpNop must have word count 1");
            }
            pos += wc;
        }
        if pos != self.words.len() {
            return Err("Trailing data after last instruction");
        }
        Ok(())
    }

    /// Write the module as raw bytes (little-endian, suitable for file or Vulkan).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.words.len() * 4);
        for &w in &self.words {
            out.extend_from_slice(&w.to_le_bytes());
        }
        out
    }
}

// ════════════════════════════════════════════════════════════════════
//  GLSL.std.450 instruction indices
// ════════════════════════════════════════════════════════════════════

pub mod glsl {
    pub const ROUND: u32 = 1;
    pub const ROUND_EVEN: u32 = 2;
    pub const TRUNC: u32 = 3;
    pub const F_ABS: u32 = 4;
    pub const S_ABS: u32 = 5;
    pub const F_SIGN: u32 = 6;
    pub const S_SIGN: u32 = 7;
    pub const FLOOR: u32 = 8;
    pub const CEIL: u32 = 9;
    pub const FRACT: u32 = 10;
    pub const SIN: u32 = 13;
    pub const COS: u32 = 14;
    pub const TAN: u32 = 15;
    pub const ASIN: u32 = 16;
    pub const ACOS: u32 = 17;
    pub const ATAN: u32 = 18;
    pub const SINH: u32 = 19;
    pub const COSH: u32 = 20;
    pub const TANH: u32 = 21;
    pub const ATAN2: u32 = 25;
    pub const POW: u32 = 26;
    pub const EXP: u32 = 27;
    pub const LOG: u32 = 28;
    pub const EXP2: u32 = 29;
    pub const LOG2: u32 = 30;
    pub const SQRT: u32 = 31;
    pub const INVERSE_SQRT: u32 = 32;
    pub const F_MIN: u32 = 37;
    pub const U_MIN: u32 = 38;
    pub const S_MIN: u32 = 39;
    pub const F_MAX: u32 = 40;
    pub const U_MAX: u32 = 41;
    pub const S_MAX: u32 = 42;
    pub const F_CLAMP: u32 = 43;
    pub const U_CLAMP: u32 = 44;
    pub const S_CLAMP: u32 = 45;
    pub const F_MIX: u32 = 46;
    pub const FMA: u32 = 50;
    pub const FIND_I_LSB: u32 = 73;
    pub const FIND_S_MSB: u32 = 74;
    pub const FIND_U_MSB: u32 = 75;
}

// ════════════════════════════════════════════════════════════════════
//  Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_and_finalize() {
        let mut e = Emitter::new();
        e.emit_header();
        let id1 = e.alloc_id();
        let id2 = e.alloc_id();
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        e.finalize();
        assert_eq!(e.words()[0], 0x07230203);
        assert_eq!(e.words()[3], 3); // bound = next_id
    }

    #[test]
    fn minimal_compute_module() {
        let mut e = Emitter::new();
        e.emit_header();

        e.emit_capability(capability::SHADER);
        let glsl = e.emit_ext_inst_import("GLSL.std.450");
        e.emit_memory_model(0, 1); // Logical, GLSL450

        let void_ty = e.emit_type_void();
        let func_ty = e.emit_type_function(void_ty, &[]);

        let func = e.emit_function(void_ty, 0, func_ty);
        e.emit_entry_point(execution_model::GLCOMPUTE, func, "main", &[]);
        e.emit_execution_mode(func, execution_mode::LOCAL_SIZE, &[64, 1, 1]);

        let _label = e.emit_label();
        e.emit_return();
        e.emit_function_end();

        e.finalize();
        assert!(e.validate().is_ok());

        let u32_ty = e.emit_type_int(32, 0);
        let _ = e.emit_ext_inst(u32_ty, glsl, glsl::S_ABS, &[]);
    }

    #[test]
    fn validate_catches_bad_magic() {
        let e = Emitter { words: vec![0xDEADBEEF, 0, 0, 1, 0], next_id: 1, bound_idx: 3 };
        assert!(e.validate().is_err());
    }

    #[test]
    fn literal_encoding() {
        assert_eq!(42u8.to_words().collect::<Vec<_>>(), vec![42]);
        assert_eq!(1000u16.to_words().collect::<Vec<_>>(), vec![1000]);
        assert_eq!(0xDEADBEEFu32.to_words().collect::<Vec<_>>(), vec![0xDEADBEEF]);
        let f: f32 = 1.0;
        assert_eq!(f.to_words().collect::<Vec<_>>(), vec![f.to_bits()]);
        let big: u64 = 0x0102030405060708;
        let words: Vec<_> = big.to_words().collect();
        assert_eq!(words, vec![0x05060708, 0x01020304]);
    }
}
