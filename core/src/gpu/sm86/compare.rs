use super::*;

impl<'a> Decoder<'a> {
    fn pp_set(&mut self, inst: u128) -> u32 {
        let bit = self.pp_bit(inst);
        self.ir.emit_inot_equal(self.type_bool[1], bit, self.const_u32_0)
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
    pub(super) fn store_setp(&mut self, inst: u128, result: u32) {
        let p = self.pp_bit(inst);
        let inverse = self.ir.emit_bitwise_xor(self.type_u32[1], result, self.const_u32_1);
        for (value, lsb) in [(result, 81), (inverse, 84)] {
            let value = self.bool_op(inst >> 74 & 3, value, p);
            self.store_pred_bit(is_uniform(inst), (inst >> lsb & 7) as u32, value);
        }
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

    pub fn dsetp(&mut self, inst: u128) {
        let a = self.fp64_reg(inst, 24, 73, 72);
        let b = self.fp64_alu_src(inst);
        let value = self.fp_compare((inst >> 76 & 15) as u32, a, b);
        self.store_setp(inst, value);
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
        self.store_fp_register((inst >> 16 & 0xff) as u32, bits);
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
            let [min, max] = [spirv::glsl::N_MIN, spirv::glsl::N_MAX].map(|op| self.glsl(op, self.type_f32[1], &[a[i], b[i]]));
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
    pub fn p2r(&mut self, inst: u128) {
        let byte = (inst >> 76 & 3) as u32 * 8;
        let shift = self.cached_const_u32(byte);
        let mask = self.alu_src(inst);
        let predicate_mask = self.cached_const_u32(P0_TO_P6);
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
    pub fn plop3(&mut self, inst: u128) {
        let uniform = inst as u32 & 0xfff == UPLOP3;
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
    pub fn r2p(&mut self, inst: u128) {
        let byte = (inst >> 76 & 3) as u32 * 8;
        let value = self.load_register((inst >> 24 & 0xff) as u32);
        let shift = self.cached_const_u32(byte);
        let value = self.ir.emit_shift_right_logical(self.type_u32[1], value, shift);
        let mask = self.alu_src(inst);
        let predicate_mask = self.cached_const_u32(P0_TO_P6);
        let mask = self.ir.emit_bitwise_and(self.type_u32[1], mask, predicate_mask);
        let old = self.load_predicate();
        let inverse = self.ir.emit_not(self.type_u32[1], mask);
        let keep = self.ir.emit_bitwise_and(self.type_u32[1], old, inverse);
        let insert = self.ir.emit_bitwise_and(self.type_u32[1], value, mask);
        let result = self.ir.emit_bitwise_or(self.type_u32[1], keep, insert);
        self.store_predicate(result);
    }
    pub fn sel(&mut self, inst: u128) {
        let p = self.pp_set(inst);
        let (a, b) = (self.alu_reg(inst, 24), self.src_b(inst));
        let result = self.ir.emit_select(self.type_u32[1], p, a, b);
        self.store_alu(inst, result);
    }
}
