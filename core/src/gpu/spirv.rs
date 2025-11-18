//! SPIR-V IR emitter for GPU instructions

#[derive(Debug, Clone)]
pub struct Emitter {
    code: Vec<u32>,
}
impl Emitter {
    pub fn new() -> Self {
        Self { code: Vec::new() }
    }
    pub fn emit_header(&mut self) {
        self.code.push(0x07230203);
    }
    fn emit_generic(&mut self, opcode: u32, data: &[u32]) {
        let start = self.code.len();
        self.code.push(0);
        for &e in data {
            self.code.push(e);
        }
        // correct new opcode
        let end = self.code.len();
        self.code[start] = (u32::try_from(end - start).unwrap() << 16) | opcode;
    }
    pub fn emit_nop(&mut self) {
        self.emit_generic(0, &[]);
    }
    pub fn emit_undef(&mut self, result_type: u32, result: u32) {
        self.emit_generic(1, &[result_type, result]);
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
    pub fn emit_decoration_group(&mut self, result: u32) {
        self.emit_generic(73, &[result]);
    }
    pub fn emit_group_decorate(&mut self, group: u32, targets: &[u32]) {
        let len = 2 + u32::try_from(targets.len()).unwrap();
        self.code.push((len << 16) | 74);
        self.code.push(group);
        for &l in targets {
            self.code.push(l);
        }
    }
    pub fn emit_type_void(&mut self, result: u32) {
        self.emit_generic(19, &[result]);
    }
    pub fn emit_type_bool(&mut self, result: u32) {
        self.emit_generic(20, &[result]);
    }
    pub fn emit_type_inst(&mut self, result: u32, width: u32, sign: u32) {
        assert!(sign == 0 || sign == 1);
        self.emit_generic(21, &[result, width, sign]);
    }
    pub fn emit_type_float(&mut self, result: u32, width: u32) {
        self.emit_generic(22, &[result, width]);
    }
    pub fn emit_type_vector(&mut self, result: u32, type_: u32, count: u32) {
        assert!(count >= 2);
        self.emit_generic(23, &[result, type_, count]);
    }
    pub fn emit_type_matrix(&mut self, result: u32, type_: u32, count: u32) {
        assert!(count >= 2);
        self.emit_generic(24, &[result, type_, count]);
    }
    pub fn emit_type_image(
        &mut self,
        result: u32,
        type_: u32,
        dim: u32,
        depth: u32,
        arrayed: u32,
        ms: u32,
        sampled: u32,
        format: u32,
        acc_qual: &[u32],
    ) {
        assert!(depth <= 2);
        assert!(arrayed == 0 || arrayed == 1);
        assert!(ms == 0 || ms == 1);
        assert!(sampled <= 2);
        let mut data = vec![result, type_, dim, depth, arrayed, ms, sampled, format];
        for &e in acc_qual {
            data.push(e);
        }
        self.emit_generic(25, &data);
    }
    pub fn emit_type_sampler(&mut self, result: u32) {
        self.emit_generic(26, &[result]);
    }
    pub fn emit_type_sampled_image(&mut self, result: u32, type_: u32) {
        self.emit_generic(27, &[result, type_]);
    }
    pub fn emit_type_array(&mut self, result: u32, type_: u32, length: u32) {
        self.emit_generic(28, &[result, type_, length]);
    }
}

impl Default for Emitter {
    fn default() -> Self {
        Self::new()
    }
}
