use super::*;

impl<'a> Decoder<'a> {
    fn f2fp_src(&mut self, val: u32, relu: bool, satfinite: bool) -> u32 {
        let ty = self.type_f32[1];
        let zero = self.ir.emit_constant_typed(ty, 0.0f32);
        let max = self.ir.emit_constant_typed(ty, 65504.0f32);
        let min = self.ir.emit_constant_typed(ty, -65504.0f32);
        let val = if relu || satfinite {
            let floor = if relu { zero } else { min };
            let below = self.ir.emit_ford_less_than(self.type_bool[1], val, floor);
            self.ir.emit_select(ty, below, floor, val)
        } else {
            val
        };
        if satfinite {
            let above = self.ir.emit_ford_greater_than(self.type_bool[1], val, max);
            self.ir.emit_select(ty, above, max, val)
        } else {
            val
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
            let result = self.ir.emit_select(self.type_u32[1], nan, lo, result);
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
            let result = self.ir.emit_select(u64_ty, below, lo, converted);
            let result = self.ir.emit_select(u64_ty, above, hi, result);
            let result = self.ir.emit_select(u64_ty, nan, lo, result);
            self.store_pair(rd, result);
        }
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
        assert!(inst >> 76 & 1 == 0 && inst >> 86 & 1 == 0, "F2FP to BF16, E8M7 or E6M9 needs a bfloat16 type");
        let rnd = inst >> 79 & 7;
        assert!(matches!(rnd, 0 | 3), "F2FP.RS stochastic rounding cannot be expressed in SPIR-V");
        let relu = inst >> 75 & 1 != 0;
        let satfinite = inst >> 77 & 1 != 0;
        let (b, c) = self.srcs_bc(inst);
        let b = self.bitcast_f32(b);
        let b = self.f2fp_src(b, relu, satfinite);
        let word = if inst >> 78 & 1 == 0 {
            let a = self.load_register((inst >> 24 & 0xff) as u32);
            let a = self.bitcast_f32(a);
            let a = self.f2fp_src(a, relu, satfinite);
            self.pack_half2_round([b, a], rnd == 3)
        } else {
            let packed = self.pack_half2_round([b, b], rnd == 3);
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
        let nan_byte = self.cached_const_u32(0x80);
        let [a, b] = [a, b].map(|x| {
            let x = self.bitcast_f32(x);
            let nan = self.ir.emit_f_is_nan(self.type_bool[1], x);
            let x = self.glsl(round, self.type_f32[1], &[x]);
            let x = self.glsl(spirv::glsl::N_CLAMP, self.type_f32[1], &[x, lo, hi]);
            let x = self.ir.emit_convert_f_to_s(self.type_s32[1], x);
            let x = self.bitcast_u32(x);
            self.ir.emit_select(self.type_u32[1], nan, nan_byte, x)
        });
        let c = self.high_half_if(c, inst >> 72 & 1 != 0);
        let word = self.pack_narrow(a, b, c, 8);
        self.store_register((inst >> 16 & 0xff) as u32, word);
    }
    pub fn i2f(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let extract = ((inst >> 60) & 3) as u32;
        let size = ((inst >> 75) & 3) as u32;
        assert_round_to_nearest(inst);
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
        let rnd = inst >> 78 & 3;
        assert!(matches!(rnd, 0 | 3), "I2FP rounding mode {rnd} is not defined by the SM86 table");
        let b = self.alu_src(inst);
        let signed = inst >> 74 & 1 != 0;
        let integer = if signed { self.ir.emit_bitcast(self.type_s32[1], b) } else { b };
        let val = if signed {
            self.ir.emit_convert_s_to_f(self.type_f32[1], integer)
        } else {
            self.ir.emit_convert_u_to_f(self.type_f32[1], integer)
        };
        let mut bits = self.bitcast_u32(val);
        if rnd == 3 {
            let negative = self.ir.emit_sless_than(self.type_bool[1], integer, self.const_u32_0);
            let magnitude = self.ir.emit_isub(self.type_u32[1], self.const_u32_0, integer);
            let magnitude = if signed { self.ir.emit_select(self.type_u32[1], negative, magnitude, integer) } else { integer };
            let [sign_mask, mantissa_mask, hidden, twenty_three, one_fifty, one_fifty_nine] =
                [0x7fff_ffff, 0x7f_ffff, 0x80_0000, 23, 150, 159].map(|c| self.cached_const_u32(c));
            let signless = self.ir.emit_bitwise_and(self.type_u32[1], bits, sign_mask);
            let exponent = self.ir.emit_shift_right_logical(self.type_u32[1], signless, twenty_three);
            let significand = self.ir.emit_bitwise_and(self.type_u32[1], signless, mantissa_mask);
            let significand = self.ir.emit_bitwise_or(self.type_u32[1], significand, hidden);
            let inexact_range = self.ir.emit_ugreater_than(self.type_bool[1], exponent, one_fifty);
            let shift = self.ir.emit_isub(self.type_u32[1], exponent, one_fifty);
            let shift = self.ir.emit_select(self.type_u32[1], inexact_range, shift, self.const_u32_0);
            let represented = self.ir.emit_shift_left_logical(self.type_u32[1], significand, shift);
            let rounded_up = self.ir.emit_ugreater_than(self.type_bool[1], represented, magnitude);
            let overflow = self.ir.emit_ugreater_than_equal(self.type_bool[1], exponent, one_fifty_nine);
            let rounded_up = self.ir.emit_logical_or(self.type_bool[1], rounded_up, overflow);
            let above = self.ir.emit_logical_and(self.type_bool[1], inexact_range, rounded_up);
            let down = self.ir.emit_isub(self.type_u32[1], bits, self.const_u32_1);
            bits = self.ir.emit_select(self.type_u32[1], above, down, bits);
        }
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
}
