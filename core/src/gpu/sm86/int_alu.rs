use super::*;

impl<'a> Decoder<'a> {
    // Bit 63 is the ~ on a register or cbuf B and part of the immediate otherwise.
    fn src_b_maybe_inverted(&mut self, inst: u128) -> u32 {
        let b = self.src_b(inst);
        if inst >> 63 & 1 != 0 && !matches!(inst >> 9 & 7, 2 | 4) { self.ir.emit_not(self.type_u32[1], b) } else { b }
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

    fn int_src(&mut self, val: u32, neg: bool) -> u32 {
        if neg { self.ir.emit_isub(self.type_u32[1], self.const_u32_0, val) } else { val }
    }

    pub fn bitextract(&mut self, _inst: u128) {
        panic!("BITEXTRACT is masked on SM86 (class gather_bitextract_ MASK_INSTR_PREDICATE = (1)): nvdisasm does not decode it and ptxas feeds mma.sp metadata to HMMA.SP as a register");
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
    pub fn brev(&mut self, inst: u128) {
        let b = self.src_b(inst);
        let res = self.ir.emit_bit_reverse(self.type_u32[1], b);
        self.store_alu(inst, res);
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
    pub fn flo(&mut self, inst: u128) {
        let ty = self.type_u32[1];
        let b = self.src_b_maybe_inverted(inst);
        let nonzero = self.ir.emit_inot_equal(self.type_bool[1], b, self.const_u32_0);
        let nonzero = self.bool_bit(nonzero);
        self.store_pred_bit(is_uniform(inst), (inst >> 81 & 7) as u32, nonzero);
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
    pub fn iabs(&mut self, inst: u128) {
        let value = self.alu_src(inst);
        let shift = self.cached_const_u32(31);
        let sign = self.ir.emit_shift_right_arithmetic(self.type_u32[1], value, shift);
        let value = self.ir.emit_bitwise_xor(self.type_u32[1], value, sign);
        let value = self.ir.emit_isub(self.type_u32[1], value, sign);
        self.store_register((inst >> 16 & 0xff) as u32, value);
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
            if mode == 7 {
                let shift = self.ir.emit_constant_typed(ty, 32u64);
                let hi = self.ir.emit_shift_right_logical(ty, result, shift);
                let hi = self.ir.emit_u_convert(self.type_u32[1], hi);
                return self.store_alu(inst, hi);
            }
            self.store_alu_pair(inst, result);
        } else {
            let product = self.ir.emit_imul(self.type_u32[1], a, b);
            let result = self.ir.emit_iadd(self.type_u32[1], product, c32);
            let result = self.ir.emit_iadd(self.type_u32[1], result, carry);
            self.store_alu(inst, result);
        }
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
    pub fn mov(&mut self, inst: u128) {
        const MOV_RR: u32 = 0x202;
        const MOV_RI: u32 = 0x802;
        const MOV_RC: u32 = 0xa02;
        const MOV_RCX: u32 = 0x1a02;
        const MOV_RU: u32 = 0x1c02;
        const MOV_IRF_R: u32 = 0x1478;
        const MOV_IRF_I: u32 = 0x1878;
        const MOV_IRF_C: u32 = 0x1a78;
        const MOV_R_IRF: u32 = 0x1c78;
        let indexed_ptr = |this: &mut Self, ur| {
            let index = this.load_ureg(ur);
            let limit = this.cached_const_u32(MAX_REG_COUNT as u32);
            let valid = this.ir.emit_uless_than(this.type_bool[1], index, limit);
            let index = this.ir.emit_select(this.type_u32[1], valid, index, this.const_u32_0);
            let array = this.ir.emit_access_chain(this.type_ptr_abstract_state_regs, this.var_abstract_state, &[this.const_u32_0]);
            let ptr = this.ir.emit_access_chain(this.type_ptr_u32[1], array, &[index]);
            (ptr, valid)
        };
        let val = match opcode(inst) {
            MOV_RR | MOV_RI | MOV_RC | MOV_RU => self.alu_src(inst),
            MOV_RCX => {
                let bank = self.load_ureg((inst >> 32 & 0x3f) as u32);
                let offset = self.cached_const_u32(((inst >> 40 & 0x3fff) << 2) as u32);
                self.load_dynamic_cbuf(bank, offset)
            }
            MOV_IRF_R | MOV_IRF_I | MOV_IRF_C => {
                let field = (inst >> 32) as u32;
                let val = match opcode(inst) {
                    MOV_IRF_R => self.load_register(field & 0xff),
                    MOV_IRF_I => self.cached_const_u32(field),
                    _ => {
                        let offset = self.cached_const_u32((field >> 8 & 0x3fff) << 2);
                        self.load_cbuf(ResourceKind::Cbuf(field >> 22 & 0x1f), offset)
                    }
                };
                let (ptr, valid) = indexed_ptr(self, (inst >> 16 & 0x3f) as u32);
                let old = self.ir.emit_load(self.type_u32[1], ptr);
                let val = self.ir.emit_select(self.type_u32[1], valid, val, old);
                return self.ir.emit_store(ptr, val);
            }
            MOV_R_IRF => {
                let (ptr, valid) = indexed_ptr(self, (inst >> 32 & 0x3f) as u32);
                let val = self.ir.emit_load(self.type_u32[1], ptr);
                self.ir.emit_select(self.type_u32[1], valid, val, self.const_u32_0)
            }
            op => panic!("invalid MOV opcode {op:#x}"),
        };
        self.store_register((inst >> 16 & 0xff) as u32, val);
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
        let negated = [inst >> 72 & 1 != 0, inst >> 9 & 7 != 4 && inst >> 63 & 1 != 0, inst >> 75 & 1 != 0];
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
}
