//! SPIR-V IR emitter for GPU instructions
// https://registry.khronos.org/SPIR-V/specs/unified1/SPIRV.html

// Hopefully heap ellision kicks in, knowing rust codegen that may not happen at all :)
// Heap elision is no longer needed!! we use iterators now!

use std::iter;

pub trait SpirvNumericLiteral {
    type Iter: Iterator<Item = u32>;
    fn to_words(self) -> Self::Iter;
}

impl SpirvNumericLiteral for u8 {
    type Iter = iter::Once<u32>;
    fn to_words(self) -> Self::Iter {
        iter::once(self as u32)
    }
}

impl SpirvNumericLiteral for u16 {
    type Iter = iter::Once<u32>;
    fn to_words(self) -> Self::Iter {
        iter::once(self as u32)
    }
}

impl SpirvNumericLiteral for u32 {
    type Iter = iter::Once<u32>;
    fn to_words(self) -> Self::Iter {
        iter::once(self)
    }
}

#[derive(Debug, Clone)]
pub struct Emitter {
    code: Vec<u32>,
    id_count: u32,
}

impl Emitter {
    pub fn new() -> Self {
        Self {
            code: Vec::default(),
            id_count: 0,
        }
    }

    pub fn new_id(&mut self) -> u32 {
        self.id_count += 1;
        self.id_count - 1
    }

    pub fn emit_header(&mut self) {
        self.code.push(0x07230203);
    }

    fn emit_generic(&mut self, opcode: u32, data: &[u32]) {
        let start = self.code.len();
        self.code.push(0); // Placeholder for header
        for &e in data {
            self.code.push(e);
        }
        // fixup opcode and length
        let end = self.code.len();
        self.code[start] = (u32::try_from(end - start).unwrap() << 16) | opcode;
    }

    pub fn emit_nop(&mut self) {
        self.emit_generic(0, &[]);
    }

    pub fn emit_undef(&mut self, result_type: u32) -> u32 {
        let result = self.new_id();
        self.emit_generic(1, &[result_type, result]);
        result
    }

    pub fn emit_decorate(&mut self, target: u32, deco: u32, literals: &[u32]) {
        let len = 3 + u32::try_from(literals.len()).unwrap();
        self.code.push((len << 16) | 71);
        self.code.push(target);
        self.code.push(deco);
        for &l in literals {
            self.code.push(l);
        }
    }

    pub fn emit_member_decorate(&mut self, type_: u32, member: u32, deco: u32, literals: &[u32]) {
        let len = 3 + u32::try_from(literals.len()).unwrap();
        self.code.push((len << 16) | 72);
        self.code.push(type_);
        self.code.push(member);
        self.code.push(deco);
        for &l in literals {
            self.code.push(l);
        }
    }

    pub fn emit_decoration_group(&mut self) -> u32 {
        let result = self.new_id();
        self.emit_generic(73, &[result]);
        result
    }

    pub fn emit_group_decorate(&mut self, group: u32, targets: &[u32]) {
        let len = 2 + u32::try_from(targets.len()).unwrap();
        self.code.push((len << 16) | 74);
        self.code.push(group);
        for &l in targets {
            self.code.push(l);
        }
    }

    pub fn emit_type_void(&mut self) -> u32 {
        let result = self.new_id();
        self.emit_generic(19, &[result]);
        result
    }

    pub fn emit_type_bool(&mut self) -> u32 {
        let result = self.new_id();
        self.emit_generic(20, &[result]);
        result
    }

    pub fn emit_type_int(&mut self, width: u32, sign: u32) -> u32 {
        let result = self.new_id();
        assert!(sign == 0 || sign == 1);
        self.emit_generic(21, &[result, width, sign]);
        result
    }

    pub fn emit_type_float(&mut self, width: u32) -> u32 {
        let result = self.new_id();
        self.emit_generic(22, &[result, width]);
        result
    }

    pub fn emit_type_vector(&mut self, type_: u32, count: u32) -> u32 {
        let result = self.new_id();
        assert!(count >= 2);
        self.emit_generic(23, &[result, type_, count]);
        result
    }

    pub fn emit_type_matrix(&mut self, type_: u32, count: u32) -> u32 {
        let result = self.new_id();
        assert!(count >= 2);
        self.emit_generic(24, &[result, type_, count]);
        result
    }

    pub fn emit_type_image(&mut self, type_: u32, dim: u32, depth: u32, arrayed: u32, ms: u32, sampled: u32, format: u32, acc_qual: &[u32]) -> u32 {
        let result = self.new_id();
        assert!(depth <= 2);
        assert!(arrayed == 0 || arrayed == 1);
        assert!(ms == 0 || ms == 1);
        assert!(sampled <= 2);
        let mut data = vec![result, type_, dim, depth, arrayed, ms, sampled, format];
        for &e in acc_qual {
            data.push(e);
        }
        self.emit_generic(25, &data);
        result
    }

    pub fn emit_type_sampler(&mut self) -> u32 {
        let result = self.new_id();
        self.emit_generic(26, &[result]);
        result
    }

    pub fn emit_type_sampled_image(&mut self, type_: u32) -> u32 {
        let result = self.new_id();
        self.emit_generic(27, &[result, type_]);
        result
    }

    pub fn emit_type_array(&mut self, type_: u32, length: u32) -> u32 {
        let result = self.new_id();
        self.emit_generic(28, &[result, type_, length]);
        result
    }

    pub fn emit_type_runtime_array(&mut self, type_: u32) -> u32 {
        let result = self.new_id();
        self.emit_generic(29, &[result, type_]);
        result
    }

    // TODO: 29 to 42

    pub fn emit_constant_true(&mut self, type_: u32) -> u32 {
        let result = self.new_id();
        self.emit_generic(41, &[type_, result]);
        result
    }

    pub fn emit_constant_false(&mut self, type_: u32) -> u32 {
        let result = self.new_id();
        self.emit_generic(42, &[type_, result]);
        result
    }

    pub fn emit_constant(&mut self, type_: u32, value: &[u32]) -> u32 {
        let result = self.new_id();
        let mut data = vec![type_, result];
        for &e in value {
            data.push(e);
        }
        self.emit_generic(43, &data);
        result
    }

    pub fn emit_constant_typed<T: SpirvNumericLiteral>(&mut self, type_: u32, value: T) -> u32 {
        let result = self.new_id();
        let mut data = vec![type_, result];
        data.extend(value.to_words());
        self.emit_generic(43, &data);
        result
    }
}

impl Default for Emitter {
    fn default() -> Self {
        Self::new()
    }
}
