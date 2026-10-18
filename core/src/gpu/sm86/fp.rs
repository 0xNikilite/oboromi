use super::*;

impl<'a> Decoder<'a> {
    fn bitcast_u64(&mut self, val: u32) -> u32 {
        let ty = self.type_u64();
        self.ir.emit_bitcast(ty, val)
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
        assert_round_to_nearest(inst);
        self.ir.emit_decorate(val, spirv::decoration::NO_CONTRACTION, &[]);
        let val = if inst >> 77 & 1 != 0 {
            let [zero, one] = [0.0f32, 1.0].map(|c| self.ir.emit_constant_typed(self.type_f32[1], c));
            self.glsl(spirv::glsl::N_CLAMP, self.type_f32[1], &[val, zero, one])
        } else {
            val
        };
        let bits = self.bitcast_u32(val);
        if ftz { self.flush_subnormal(bits) } else { bits }
    }

    // NVIDIA returns the first quiet NaN operand, else the first signaling one quieted, in both cases without its sign.
    fn propagated_nan_bits(&mut self, val: u32, operands: &[u32]) -> u32 {
        let (ty, bool_ty) = (self.type_u64(), self.type_bool[1]);
        let [magnitude, infinity, quiet, default] = [0x7fff_ffff_ffff_ffffu64, 0x7ff0_0000_0000_0000, 0x7ff8_0000_0000_0000, 0xfff8_0000_0000_0000]
            .map(|c| self.ir.emit_constant_typed(ty, c));
        let mut nan_bits = default;
        for quiet_only in [false, true] {
            for &operand in operands.iter().rev() {
                let bits = self.bitcast_u64(operand);
                let bits = self.ir.emit_bitwise_and(ty, bits, magnitude);
                let nan = if quiet_only { self.ir.emit_ugreater_than_equal(bool_ty, bits, quiet) } else { self.ir.emit_ugreater_than(bool_ty, bits, infinity) };
                let quieted = self.ir.emit_bitwise_or(ty, bits, quiet);
                nan_bits = self.ir.emit_select(ty, nan, quieted, nan_bits);
            }
        }
        let nan = self.ir.emit_f_is_nan(bool_ty, val);
        let bits = self.bitcast_u64(val);
        self.ir.emit_select(ty, nan, nan_bits, bits)
    }

    pub fn dadd(&mut self, inst: u128) {
        assert_round_to_nearest(inst);
        let ty = self.type_f64();
        let a = self.fp64_reg(inst, 24, 73, 72);
        // DADD's second operand sits in the C slot, so a register comes from Rc.
        let b = if inst >> 9 & 7 == 1 { self.fp64_reg(inst, 64, 74, 75) } else { self.fp64_alu_src(inst) };
        let result = self.ir.emit_fadd(ty, a, b);
        let result = self.propagated_nan_bits(result, &[a, b]);
        self.store_pair((inst >> 16 & 0xff) as u32, result);
    }
    pub fn dfma(&mut self, inst: u128) {
        assert_round_to_nearest(inst);
        let ty = self.type_f64();
        let a = self.fp64_reg(inst, 24, 73, 72);
        let src = self.fp64_alu_src(inst);
        let c = self.fp64_reg(inst, 64, 74, 75);
        let [b, c] = if matches!(inst >> 9 & 7, 2 | 3 | 7) { [c, src] } else { [src, c] };
        let result = self.glsl(spirv::glsl::FMA, ty, &[a, b, c]);
        let result = self.propagated_nan_bits(result, &[a, b, c]);
        self.store_pair((inst >> 16 & 0xff) as u32, result);
    }
    pub fn dmul(&mut self, inst: u128) {
        assert_round_to_nearest(inst);
        let ty = self.type_f64();
        let a = self.fp64_reg(inst, 24, 73, 72);
        let b = self.fp64_alu_src(inst);
        let result = self.ir.emit_fmul(ty, a, b);
        let result = self.propagated_nan_bits(result, &[a, b]);
        self.store_pair((inst >> 16 & 0xff) as u32, result);
    }
    pub fn fadd(&mut self, inst: u128) {
        let ftz = inst >> 80 & 1 != 0;
        let a = self.fp_reg(inst, 24, 73, 72, ftz);
        let b = self.fp_alu_src(inst, ftz);
        let sum = self.ir.emit_fadd(self.type_f32[1], a, b);
        let bits = self.fp_arith_result(inst, sum, ftz);
        self.store_fp_register((inst >> 16 & 0xff) as u32, bits);
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
        self.store_fp_register((inst >> 16 & 0xff) as u32, bits);
    }
    pub fn fmul(&mut self, inst: u128) {
        let fmz = inst >> 76 & 1 != 0;
        let ftz = fmz || inst >> 80 & 1 != 0;
        let a = self.fp_reg(inst, 24, 73, 72, ftz);
        let b = self.fp_alu_src(inst, ftz);
        let [a, b] = if fmz { self.fmz_operands(a, b) } else { [a, b] };
        let mut product = self.ir.emit_fmul(self.type_f32[1], a, b);
        let rounding = inst >> 78 & 3;
        if rounding == 3 {
            product = self.round_product_toward_zero(product, a, b);
        }
        let scale = (inst >> 84 & 7) as i32 - 4;
        if scale != 0 {
            let factor = self.ir.emit_constant_typed(self.type_f32[1], 2f32.powi(scale));
            product = self.ir.emit_fmul(self.type_f32[1], product, factor);
        }
        let bits = self.fp_arith_result(if rounding == 3 { inst & !(3 << 78) } else { inst }, product, ftz);
        self.store_fp_register((inst >> 16 & 0xff) as u32, bits);
    }
    // Fma gives the exact residual a * b - p, whose sign says whether p was rounded away from zero; a device that does not
    // fuse Fma gets a zero residual and so the nearest product.
    fn round_product_toward_zero(&mut self, product: u32, a: u32, b: u32) -> u32 {
        let (ty, uty, bool_ty) = (self.type_f32[1], self.type_u32[1], self.type_bool[1]);
        let negated = self.ir.emit_fnegate(ty, product);
        let residual = self.glsl(spirv::glsl::FMA, ty, &[a, b, negated]);
        let zero = self.ir.emit_constant_typed(ty, 0.0f32);
        let inexact = self.ir.emit_ford_not_equal(bool_ty, residual, zero);
        let [bits, residual_bits] = [product, residual].map(|x| self.bitcast_u32(x));
        let signs = self.ir.emit_bitwise_xor(uty, bits, residual_bits);
        let opposite = self.ir.emit_sless_than(bool_ty, signs, self.const_u32_0);
        let away = self.ir.emit_logical_and(bool_ty, inexact, opposite);
        let toward_zero = self.ir.emit_isub(uty, bits, self.const_u32_1);
        let bits = self.ir.emit_select(uty, away, toward_zero, bits);
        self.bitcast_f32(bits)
    }
    pub fn frnd(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let rnd = ((inst >> 78) & 3) as usize;
        let ftz = ((inst >> 80) & 1) != 0;
        let fmt = (((inst >> 84) & 0x3) << 2) | (((inst >> 75) & 0x3) << 0);
        match fmt {
            5 => {
                let val = self.f16_alu_src(inst, ftz);
                let val = self.glsl(ROUNDING[rnd], self.type_f32[1], &[val]);
                let bits = self.pack_f16_round(val, false, 0, ftz);
                self.store_register(rd, bits);
            }
            10 => {
                let val = self.fp_alu_src(inst, ftz);
                let val = self.glsl(ROUNDING[rnd], self.type_f32[1], &[val]);
                let val = self.bitcast_u32(val);
                self.store_fp_register(rd, val);
            }
            15 => {
                let val = self.fp64_alu_src(inst);
                let ty = self.type_f64();
                let rounded = self.glsl(ROUNDING[rnd], ty, &[val]);
                let bits = self.propagated_nan_bits(rounded, &[val]);
                self.store_pair(rd, bits);
            }
            _ => panic!("invalid FRND format {fmt}"),
        }
    }
    pub fn fswzadd(&mut self, inst: u128) {
        let (ty, f32_ty, bool_ty) = (self.type_u32[1], self.type_f32[1], self.type_bool[1]);
        assert_round_to_nearest(inst);
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
    pub fn hadd2(&mut self, inst: u128) {
        let ftz = inst >> 80 & 1 != 0;
        let a = self.half2_reg(inst, 24, 74, 73, 72, ftz);
        let b = self.half2_alu_src(inst, ftz);
        let sum = [0, 1].map(|i| self.ir.emit_fadd(self.type_f32[1], a[i], b[i]));
        self.store_half2(inst, sum, inst >> 78 & 1 != 0, inst >> 77 & 1 != 0, ftz);
    }
    // Two halves multiply exactly in f32, so Fast2Sum recovers what rounding a * b + c to val dropped, and marking that in
    // the last bit lets the f16 conversion round the exact sum once.
    fn round_to_odd(&mut self, val: u32, a: u32, b: u32, c: u32) -> u32 {
        let (ty, uty, bool_ty) = (self.type_f32[1], self.type_u32[1], self.type_bool[1]);
        let product = self.ir.emit_fmul(ty, a, b);
        let [abs_product, abs_c] = [product, c].map(|x| self.glsl(spirv::glsl::F_ABS, ty, &[x]));
        let product_larger = self.ir.emit_ford_greater_than_equal(bool_ty, abs_product, abs_c);
        let [larger, smaller] = [[product, c], [c, product]].map(|[x, y]| self.ir.emit_select(ty, product_larger, x, y));
        let kept = self.ir.emit_fsub(ty, val, larger);
        let lost = self.ir.emit_fsub(ty, smaller, kept);
        let zero = self.ir.emit_constant_typed(ty, 0.0f32);
        let inexact = self.ir.emit_ford_not_equal(bool_ty, lost, zero);
        let [bits, lost_bits] = [val, lost].map(|x| self.bitcast_u32(x));
        let odd = self.ir.emit_bitwise_and(uty, bits, self.const_u32_1);
        let even = self.ir.emit_iequal(bool_ty, odd, self.const_u32_0);
        let sticky = self.ir.emit_logical_and(bool_ty, inexact, even);
        let signs = self.ir.emit_bitwise_xor(uty, bits, lost_bits);
        let toward_zero = self.ir.emit_sless_than(bool_ty, signs, self.const_u32_0);
        let [down, up] = [self.ir.emit_isub(uty, bits, self.const_u32_1), self.ir.emit_iadd(uty, bits, self.const_u32_1)];
        let nudged = self.ir.emit_select(uty, toward_zero, down, up);
        let bits = self.ir.emit_select(uty, sticky, nudged, bits);
        self.bitcast_f32(bits)
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
            let val = if ofmt == 1 { val } else { self.round_to_odd(val, a, b, c[i]) };
            if inst >> 79 & 1 == 0 {
                return val;
            }
            let negative = self.ir.emit_ford_less_than_equal(self.type_bool[1], val, zero);
            self.ir.emit_select(self.type_f32[1], negative, zero, val)
        });
        self.store_half2(inst, val, ofmt == 1, inst >> 77 & 1 != 0, ftz);
    }
    pub fn hfma2_mma(&mut self, inst: u128) {
        self.hfma2(inst);
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
}
