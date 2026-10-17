use super::*;

impl<'a> Decoder<'a> {
    pub fn bmma(&mut self, inst: u128) {
        assert_eq!(inst >> 87 & 0xf, 0, "BMMA with a UPp condition");
        let (rows, k_regs) = match inst >> 75 & 3 {
            0 => (1, 1),
            1 => (2, 1),
            2 => (2, 2),
            size => panic!("BMMA size {size}"),
        };
        let and = inst >> 78 & 1 != 0;
        let ty = self.type_u32[1];
        let operands = self.mma_operands(inst, rows, k_regs, |_, word, _| vec![word]);
        self.mma_s32(inst, operands, false, |s, x, y| {
            let bits = if and { s.ir.emit_bitwise_and(ty, x, y) } else { s.ir.emit_bitwise_xor(ty, x, y) };
            s.ir.emit_bit_count(ty, bits)
        });
    }
    pub fn dmma(&mut self, inst: u128) {
        assert_round_to_nearest(inst);
        assert_eq!(inst >> 87 & 0xf, 0, "DMMA with a UPp condition");
        let ty = self.type_f64();
        let [a, b] = self.mma_operands(inst, 1, 2, |_, word, _| vec![word]);
        let f64s = |s: &mut Self, words: &[u32], abs: u32, neg: u32| -> Vec<u32> {
            let (lo, hi) = words.split_at(4);
            lo.iter().zip(hi).map(|(&lo, &hi)| {
                let pair = s.join_u64(lo, hi);
                let val = s.bitcast_f64(pair);
                s.fp_src(ty, val, inst >> abs & 1 != 0, inst >> neg & 1 != 0)
            }).collect()
        };
        let a = f64s(self, &a[0], 73, 72);
        let [rd, rc] = [16, 64].map(|lsb| (inst >> lsb & 0xff) as u32);
        for (column, words) in (0..).zip(&b) {
            let b = f64s(self, words, 62, 63);
            let bits = self.load_pair(nth_reg(rc, RZ, 2 * column));
            let c = self.bitcast_f64(bits);
            let c = self.fp_src(ty, c, inst >> 74 & 1 != 0, inst >> 75 & 1 != 0);
            let d = a.iter().zip(&b).fold(c, |sum, (&x, &y)| self.glsl(spirv::glsl::FMA, ty, &[x, y, sum]));
            self.store_pair(nth_reg(rd, RZ, 2 * column), d);
        }
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
        todo!("GATHER");
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
        todo!("GENMETADATA");
    }
    // Lane 4g + t of an MMA fragment holds row (or column) g of the tile, and t picks its k columns.
    fn lane_quad(&mut self) -> [u32; 2] {
        let lane = self.special_register(0);
        let [two, three] = [2, 3].map(|c| self.cached_const_u32(c));
        [self.ir.emit_shift_right_logical(self.type_u32[1], lane, two), self.ir.emit_bitwise_and(self.type_u32[1], lane, three)]
    }
    fn quad_lanes(&mut self, first: u32) -> Vec<u32> {
        (0..4).map(|t| {
            let t = self.cached_const_u32(t);
            self.ir.emit_iadd(self.type_u32[1], first, t)
        }).collect()
    }
    // Gathers the A rows g (and g + 8 when rows is 2) and B columns 2t, 2t + 1 that the D elements of lane 4g + t need, each in ascending k.
    fn mma_operands<T>(&mut self, inst: u128, rows: u32, k_regs: u32, mut unpack: impl FnMut(&mut Self, u32, bool) -> Vec<T>) -> [[Vec<T>; 2]; 2] {
        let [ra, rb] = [24, 32].map(|lsb| (inst >> lsb & 0xff) as u32);
        let ty = self.type_u32[1];
        let [group, thread] = self.lane_quad();
        let [two, three, four] = [2, 3, 4].map(|c| self.cached_const_u32(c));
        let own_quad = self.ir.emit_shift_left_logical(ty, group, two);
        let row_lanes = self.quad_lanes(own_quad);
        let column_quad = self.ir.emit_shift_left_logical(ty, thread, three);
        let next_column_quad = self.ir.emit_iadd(ty, column_quad, four);
        let mut column_lanes = self.quad_lanes(column_quad);
        column_lanes.extend(self.quad_lanes(next_column_quad));
        let (mut a, mut b) = ([Vec::new(), Vec::new()], [Vec::new(), Vec::new()]);
        for k_reg in 0..k_regs {
            for (half, row) in (0..rows).zip(&mut a) {
                let own = self.load_register(nth_reg(ra, RZ, rows * k_reg + half));
                for [word, _] in self.warp_exchange(own, &row_lanes) {
                    row.extend(unpack(self, word, false));
                }
            }
            let own = self.load_register(nth_reg(rb, RZ, k_reg));
            let words = self.warp_exchange(own, &column_lanes);
            for (column, words) in b.iter_mut().zip(words.chunks(4)) {
                for &[word, _] in words {
                    column.extend(unpack(self, word, true));
                }
            }
        }
        [a, b]
    }
    // A bfloat16 is the upper half of an f32.
    fn mma_source_pair(&mut self, word: u32, bf16: bool, neg: bool) -> [u32; 2] {
        if !bf16 {
            return self.half2_lanes(word, 0, false, neg, false);
        }
        let [sixteen, high] = [16, 0xffff_0000].map(|c| self.cached_const_u32(c));
        let bits = [self.ir.emit_shift_left_logical(self.type_u32[1], word, sixteen), self.ir.emit_bitwise_and(self.type_u32[1], word, high)];
        bits.map(|bits| {
            let val = self.bitcast_f32(bits);
            self.fp_src(self.type_f32[1], val, false, neg)
        })
    }
    pub fn hmma(&mut self, inst: u128) {
        assert_eq!(opcode(inst), 0x23c, "HMMA with a uniform register file");
        assert!(inst >> 73 & 1 == 0, "HMMA.SP");
        assert_eq!(inst >> 87 & 0xf, 0, "HMMA with a UPp condition");
        let k = match (inst >> 78 & 1) << 1 | inst >> 75 & 1 {
            0 => 8,
            1 => 16,
            2 => 4,
            _ => panic!("HMMA size 3"),
        };
        let format = inst >> 82 & 3;
        let (k_regs, group, min_exponent) = match format {
            0 => (k / 8, 8, -14),
            1 => (k / 8, 8, -126),
            2 => (k / 4, 4, -126),
            _ => panic!("HMMA source format 3"),
        };
        let f32_acc = inst >> 76 & 1 != 0;
        let [rd, rc] = [16, 64].map(|lsb| (inst >> lsb & 0xff) as u32);
        let negate = [inst >> 72 & 1 != 0, inst >> 63 & 1 != 0];
        let [a, b] = self.mma_operands(inst, 2, k_regs, |s, word, is_b| {
            let neg = negate[is_b as usize];
            if format == 2 { vec![s.tf32_source(word, neg)] } else { s.mma_source_pair(word, format == 1, neg).to_vec() }
        });
        let (ty, f32_ty, ptr_ty, private) = (self.type_u32[1], self.type_f32[1], self.type_ptr_u32[1], spirv::storage_class::PRIVATE);
        let [length, d_base] = [72, 68].map(|c| self.cached_const_u32(c));
        if self.mma_vars[0] == 0 {
            let staged_ty = self.ir.emit_type_array(ty, length);
            let staged_ptr_ty = self.ir.emit_type_pointer(private, staged_ty);
            self.mma_vars = [staged_ptr_ty, ptr_ty, ptr_ty, ptr_ty, ptr_ty, ptr_ty, ptr_ty, ptr_ty, ptr_ty].map(|ptr| self.ir.emit_variable(ptr, private));
        }
        let [staged, output, group_index, term, acc, emax, special, ieee_sum, total] = self.mma_vars;
        let values = a.iter().enumerate().flat_map(|(row, x)| x.iter().enumerate().map(move |(k, &x)| (16 * row + k, x)));
        let values: Vec<_> = values.chain(b.iter().enumerate().flat_map(|(column, y)| y.iter().enumerate().map(move |(k, &y)| (32 + 16 * column + k, y)))).collect();
        for (index, val) in values {
            let slot = self.staged_slot(staged, index as u32);
            let bits = self.bitcast_u32(val);
            self.ir.emit_store(slot, bits);
        }
        for half in 0..2 {
            let c = if f32_acc {
                [0, 1].map(|column| self.load_register(nth_reg(rc, RZ, 2 * half + column)))
            } else {
                let word = self.load_register(nth_reg(rc, RZ, half));
                self.half2_lanes(word, 0, false, false, false).map(|c| self.bitcast_u32(c))
            };
            for (column, bits) in (0..).zip(c) {
                let slot = self.staged_slot(staged, 64 + 2 * half + column);
                self.ir.emit_store(slot, bits);
            }
        }
        let f16_ty = self.type_f16();
        let [one, four, sixteen, thirty_two, c_base, group_len, groups, f32_floor, floor] =
            [1, 4, 16, 32, 64, group as u32, a[0].len() as u32 / group as u32, -126i32 as u32, min_exponent as u32].map(|c| self.cached_const_u32(c));
        let zero = self.ir.emit_constant_typed(f32_ty, 0.0f32);
        self.counted_loop(output, four, |s, output| {
            let half = s.ir.emit_shift_right_logical(ty, output, one);
            let column = s.ir.emit_bitwise_and(ty, output, one);
            let a_base = s.ir.emit_imul(ty, half, sixteen);
            let b_base = s.ir.emit_imul(ty, column, sixteen);
            let b_base = s.ir.emit_iadd(ty, b_base, thirty_two);
            let c_index = s.ir.emit_iadd(ty, output, c_base);
            let c_slot = s.ir.emit_access_chain(ptr_ty, staged, &[c_index]);
            let c = s.ir.emit_load(ty, c_slot);
            s.ir.emit_store(acc, c);
            s.counted_loop(group_index, groups, |s, group_index| {
                let offset = s.ir.emit_imul(ty, group_index, group_len);
                let [a_first, b_first] = [a_base, b_base].map(|base| s.ir.emit_iadd(ty, base, offset));
                let acc_bits = s.ir.emit_load(ty, acc);
                let acc_val = s.bitcast_f32(acc_bits);
                let [_, c_bits, c_exponent, c_significand, c_special] = s.mma_term(acc_val, f32_floor, 23);
                let c_special = s.bool_bit(c_special);
                for (var, val) in [(emax, c_exponent), (special, c_special), (ieee_sum, acc_bits)] {
                    s.ir.emit_store(var, val);
                }
                s.counted_loop(term, group_len, |s, term| {
                    let ([x, y], [_, exponent, _], either) = s.mma_pair(staged, [a_first, b_first], term, floor);
                    let largest = s.ir.emit_load(ty, emax);
                    let largest = s.glsl(spirv::glsl::S_MAX, ty, &[largest, exponent]);
                    s.ir.emit_store(emax, largest);
                    let any = s.ir.emit_load(ty, special);
                    let either_bit = s.bool_bit(either);
                    let any = s.ir.emit_bitwise_or(ty, any, either_bit);
                    s.ir.emit_store(special, any);
                    let product = s.ir.emit_fmul(f32_ty, x, y);
                    let product = s.ir.emit_select(f32_ty, either, product, zero);
                    let sum = s.ir.emit_load(ty, ieee_sum);
                    let sum = s.bitcast_f32(sum);
                    let sum = s.ir.emit_fadd(f32_ty, sum, product);
                    let sum = s.bitcast_u32(sum);
                    s.ir.emit_store(ieee_sum, sum);
                });
                let largest = s.ir.emit_load(ty, emax);
                let c_term = s.aligned_term(largest, [c_bits, c_exponent, c_significand], one);
                s.ir.emit_store(total, c_term);
                s.counted_loop(term, group_len, |s, term| {
                    let (_, product, _) = s.mma_pair(staged, [a_first, b_first], term, floor);
                    let aligned = s.aligned_term(largest, product, four);
                    let sum = s.ir.emit_load(ty, total);
                    let sum = s.ir.emit_iadd(ty, sum, aligned);
                    s.ir.emit_store(total, sum);
                });
                let sum = s.ir.emit_load(ty, total);
                let exact = s.truncated_f32(sum, largest);
                let any = s.ir.emit_load(ty, special);
                let any = s.ir.emit_inot_equal(s.type_bool[1], any, s.const_u32_0);
                let ieee = s.ir.emit_load(ty, ieee_sum);
                let sum = s.ir.emit_select(ty, any, ieee, exact);
                let sum = if f32_acc {
                    sum
                } else {
                    let val = s.bitcast_f32(sum);
                    let rounded = s.ir.emit_f_convert(f16_ty, val);
                    let val = s.ir.emit_f_convert(f32_ty, rounded);
                    s.bitcast_u32(val)
                };
                s.ir.emit_store(acc, sum);
            });
            let d_index = s.ir.emit_iadd(ty, output, d_base);
            let d_slot = s.ir.emit_access_chain(ptr_ty, staged, &[d_index]);
            let d = s.ir.emit_load(ty, acc);
            s.ir.emit_store(d_slot, d);
        });
        for half in 0..2 {
            let d = [0, 1].map(|column| {
                let slot = self.staged_slot(staged, 68 + 2 * half + column);
                self.ir.emit_load(ty, slot)
            });
            if f32_acc {
                for (column, bits) in (0..).zip(d) {
                    self.store_register(nth_reg(rd, RZ, 2 * half + column), bits);
                }
            } else {
                let packed = d.map(|bits| self.bitcast_f32(bits));
                let packed = self.pack_half2(packed);
                self.store_register(nth_reg(rd, RZ, half), packed);
            }
        }
    }
    fn staged_slot(&mut self, staged: u32, index: u32) -> u32 {
        let index = self.cached_const_u32(index);
        self.ir.emit_access_chain(self.type_ptr_u32[1], staged, &[index])
    }
    fn counted_loop(&mut self, counter: u32, count: u32, body: impl FnOnce(&mut Self, u32)) {
        const DONT_UNROLL: u32 = 2;
        let [header, block, cont, merge] = [(); 4].map(|_| self.ir.alloc_id());
        self.ir.emit_store(counter, self.const_u32_0);
        self.ir.emit_branch(header);
        self.ir.emit_label_id(header);
        let index = self.ir.emit_load(self.type_u32[1], counter);
        let more = self.ir.emit_uless_than(self.type_bool[1], index, count);
        self.ir.emit_loop_merge(merge, cont, DONT_UNROLL);
        self.ir.emit_branch_conditional(more, block, merge);
        self.ir.emit_label_id(block);
        body(self, index);
        self.ir.emit_branch(cont);
        self.ir.emit_label_id(cont);
        let next = self.ir.emit_iadd(self.type_u32[1], index, self.const_u32_1);
        self.ir.emit_store(counter, next);
        self.ir.emit_branch(header);
        self.ir.emit_label_id(merge);
    }
    fn mma_pair(&mut self, staged: u32, firsts: [u32; 2], term: u32, floor: u32) -> ([u32; 2], [u32; 3], u32) {
        let ty = self.type_u32[1];
        let [x, y] = firsts.map(|first| {
            let index = self.ir.emit_iadd(ty, first, term);
            let slot = self.ir.emit_access_chain(self.type_ptr_u32[1], staged, &[index]);
            let bits = self.ir.emit_load(ty, slot);
            let val = self.bitcast_f32(bits);
            self.mma_term(val, floor, 10)
        });
        let sign = self.ir.emit_bitwise_xor(ty, x[1], y[1]);
        // A product's exponent is ea + eb, not that of the normalized product.
        let exponent = self.ir.emit_iadd(ty, x[2], y[2]);
        let significand = self.ir.emit_imul(ty, x[3], y[3]);
        let either = self.ir.emit_logical_or(self.type_bool[1], x[4], y[4]);
        ([x[0], y[0]], [sign, exponent, significand], either)
    }
    fn tf32_source(&mut self, word: u32, neg: bool) -> u32 {
        let mask = self.cached_const_u32(0xffff_e000);
        let bits = self.ir.emit_bitwise_and(self.type_u32[1], word, mask);
        let val = self.bitcast_f32(bits);
        self.fp_src(self.type_f32[1], val, false, neg)
    }
    fn mma_term(&mut self, val: u32, floor: u32, fraction_bits: u32) -> [u32; 5] {
        let (ty, bool_ty) = (self.type_u32[1], self.type_bool[1]);
        let [twenty_three, field_mask, mantissa_mask, bias, zero_exponent, narrowing] =
            [23, 0xff, 0x7f_ffff, 127, -1000i32 as u32, 23 - fraction_bits].map(|c| self.cached_const_u32(c));
        let bits = self.bitcast_u32(val);
        let field = self.ir.emit_shift_right_logical(ty, bits, twenty_three);
        let field = self.ir.emit_bitwise_and(ty, field, field_mask);
        let special = self.ir.emit_iequal(bool_ty, field, field_mask);
        let hidden = self.glsl(spirv::glsl::U_MIN, ty, &[field, self.const_u32_1]);
        let hidden = self.ir.emit_shift_left_logical(ty, hidden, twenty_three);
        let mantissa = self.ir.emit_bitwise_and(ty, bits, mantissa_mask);
        let significand = self.ir.emit_bitwise_or(ty, mantissa, hidden);
        let biased = self.glsl(spirv::glsl::U_MAX, ty, &[field, self.const_u32_1]);
        let exponent = self.ir.emit_isub(ty, biased, bias);
        let clamped = self.glsl(spirv::glsl::S_MAX, ty, &[exponent, floor]);
        let denormal_shift = self.ir.emit_isub(ty, clamped, exponent);
        let shift = self.ir.emit_iadd(ty, denormal_shift, narrowing);
        let significand = self.ir.emit_shift_right_logical(ty, significand, shift);
        let zero = self.ir.emit_iequal(bool_ty, significand, self.const_u32_0);
        let exponent = self.ir.emit_select(ty, zero, zero_exponent, clamped);
        [val, bits, exponent, significand, special]
    }
    fn aligned_term(&mut self, emax: u32, [sign, exponent, significand]: [u32; 3], unit_shift: u32) -> u32 {
        let (ty, bool_ty) = (self.type_u32[1], self.type_bool[1]);
        let max_shift = self.cached_const_u32(31);
        let widened = self.ir.emit_shift_left_logical(ty, significand, unit_shift);
        let distance = self.ir.emit_isub(ty, emax, exponent);
        let distance = self.glsl(spirv::glsl::U_MIN, ty, &[distance, max_shift]);
        let aligned = self.ir.emit_shift_right_logical(ty, widened, distance);
        let negative = self.ir.emit_sless_than(bool_ty, sign, self.const_u32_0);
        let negated = self.ir.emit_snegate(ty, aligned);
        self.ir.emit_select(ty, negative, negated, aligned)
    }
    fn truncated_f32(&mut self, total: u32, emax: u32) -> u32 {
        let (ty, bool_ty) = (self.type_u32[1], self.type_bool[1]);
        let [max_shift, twenty_three, below_hidden_bias, max_finite, infinity, sign_bit] =
            [31, 23, 125, 253, 0x7f80_0000, 0x8000_0000].map(|c| self.cached_const_u32(c));
        let magnitude = self.glsl(spirv::glsl::S_ABS, ty, &[total]);
        let msb = self.glsl(spirv::glsl::FIND_U_MSB, ty, &[magnitude]);
        let excess = self.ir.emit_isub(ty, msb, twenty_three);
        let dropped = self.glsl(spirv::glsl::S_MAX, ty, &[excess, self.const_u32_0]);
        let shortfall = self.ir.emit_snegate(ty, excess);
        let lifted = self.glsl(spirv::glsl::S_MAX, ty, &[shortfall, self.const_u32_0]);
        let significand = self.ir.emit_shift_right_logical(ty, magnitude, dropped);
        let significand = self.ir.emit_shift_left_logical(ty, significand, lifted);
        let biased_below_hidden = self.ir.emit_iadd(ty, excess, emax);
        let biased_below_hidden = self.ir.emit_iadd(ty, biased_below_hidden, below_hidden_bias);
        let normal = self.ir.emit_shift_left_logical(ty, biased_below_hidden, twenty_three);
        let normal = self.ir.emit_iadd(ty, normal, significand);
        let overflow = self.ir.emit_sgreater_than(bool_ty, biased_below_hidden, max_finite);
        let normal = self.ir.emit_select(ty, overflow, infinity, normal);
        let subnormal_shift = self.ir.emit_snegate(ty, biased_below_hidden);
        let subnormal_shift = self.glsl(spirv::glsl::U_MIN, ty, &[subnormal_shift, max_shift]);
        let subnormal = self.ir.emit_shift_right_logical(ty, significand, subnormal_shift);
        let tiny = self.ir.emit_sless_than(bool_ty, biased_below_hidden, self.const_u32_0);
        let zero = self.ir.emit_iequal(bool_ty, magnitude, self.const_u32_0);
        let tiny = self.ir.emit_logical_or(bool_ty, tiny, zero);
        let bits = self.ir.emit_select(ty, tiny, subnormal, normal);
        let sign = self.ir.emit_bitwise_and(ty, total, sign_bit);
        self.ir.emit_bitwise_or(ty, bits, sign)
    }
    pub fn imma(&mut self, inst: u128) {
        assert!(inst >> 72 & 1 == 0, "IMMA.SP");
        assert_eq!(inst >> 87 & 0xf, 0, "IMMA with a UPp condition");
        let (rows, k) = match inst >> 84 & 6 | inst >> 75 & 1 {
            0 => (1, 16),
            2 => (1, 32),
            4 => (2, 16),
            5 => (2, 32),
            6 => (2, 64),
            size => panic!("IMMA size {size}"),
        };
        let bits = if inst >> 83 & 1 != 0 { 4 } else { 8 };
        let signed = [76, 78].map(|bit| inst >> bit & 1 != 0);
        let ty = self.type_u32[1];
        let width = self.cached_const_u32(bits);
        let operands = self.mma_operands(inst, rows, k * bits / 128, |s, word, is_b| {
            (0..32 / bits).map(|element| {
                let offset = s.cached_const_u32(bits * element);
                if signed[is_b as usize] { s.ir.emit_bit_field_s_extract(ty, word, offset, width) } else { s.ir.emit_bit_field_u_extract(ty, word, offset, width) }
            }).collect()
        });
        self.mma_s32(inst, operands, inst >> 82 & 1 != 0, |s, x, y| s.ir.emit_imul(ty, x, y));
    }
    // D register 2 * half + column of lane 4g + t is C + the sum over k of product(A[k], B[k]) for A row g + 8 * half and B column 2t + column.
    // Session 6 silicon clamps after each half of the 32 products.
    fn mma_s32(&mut self, inst: u128, [a, b]: [[Vec<u32>; 2]; 2], saturate: bool, mut product: impl FnMut(&mut Self, u32, u32) -> u32) {
        let [rd, rc] = [16, 64].map(|lsb| (inst >> lsb & 0xff) as u32);
        let ty = self.type_u32[1];
        for (half, row) in (0..).zip(a.iter().filter(|row| !row.is_empty())) {
            for (column, col) in (0..).zip(&b) {
                let reg = 2 * half + column;
                let c = self.load_register(nth_reg(rc, RZ, reg));
                let mut d = c;
                for (row, col) in row.chunks(16).zip(col.chunks(16)) {
                    let sum = row.iter().zip(col).fold(self.const_u32_0, |sum, (&x, &y)| {
                        let term = product(self, x, y);
                        self.ir.emit_iadd(ty, sum, term)
                    });
                    let next = self.ir.emit_iadd(ty, d, sum);
                    if !saturate {
                        d = next;
                        continue;
                    }
                    let [thirty_one, max] = [31, i32::MAX as u32].map(|v| self.cached_const_u32(v));
                    let [d_flip, sum_flip] = [d, sum].map(|x| self.ir.emit_bitwise_xor(ty, x, next));
                    let flips = self.ir.emit_bitwise_and(ty, d_flip, sum_flip);
                    let overflow = self.ir.emit_sless_than(self.type_bool[1], flips, self.const_u32_0);
                    let sign = self.ir.emit_shift_right_arithmetic(ty, d, thirty_one);
                    let clamped = self.ir.emit_bitwise_xor(ty, sign, max);
                    d = self.ir.emit_select(ty, overflow, clamped, next);
                }
                self.store_register(nth_reg(rd, RZ, reg), d);
            }
        }
    }
    pub fn ldsm(&mut self, inst: u128) {
        let transposed = match inst >> 78 & 3 {
            0 => false,
            1 => true,
            _ => panic!("LDSM.M816/M832 silicon executes this word; outputs are in the session 6 tensor fixtures"),
        };
        let ty = self.type_u32[1];
        let address = self.global_address(inst, 32, 40);
        let [group, thread] = self.lane_quad();
        let first_column = self.ir.emit_shift_left_logical(ty, thread, self.const_u32_1);
        let tiles = 1 << (inst >> 72 & 3);
        let mut row_lanes = Vec::new();
        let mut columns = Vec::new();
        for tile in 0..tiles {
            let first_row = self.cached_const_u32(8 * tile);
            for half in [self.const_u32_0, self.const_u32_1] {
                let column = self.ir.emit_iadd(ty, first_column, half);
                let (row, column) = if transposed { (column, group) } else { (group, column) };
                row_lanes.push(self.ir.emit_iadd(ty, first_row, row));
                columns.push(column);
            }
        }
        let row_addresses = self.warp_exchange(address, &row_lanes);
        let halves: Vec<u32> = row_addresses.iter().zip(columns).map(|(&[row, _], column)| {
            let offset = self.ir.emit_shift_left_logical(ty, column, self.const_u32_1);
            let at = self.ir.emit_iadd(ty, row, offset);
            let ptr = self.shared_word_at(at);
            let word = self.ir.emit_load(ty, ptr);
            self.load_narrow(word, at, 2)
        }).collect();
        let sixteen = self.cached_const_u32(16);
        for (tile, pair) in (0..).zip(halves.chunks(2)) {
            let word = self.ir.emit_bit_field_insert(ty, pair[0], pair[1], sixteen, sixteen);
            self.store_register(nth_reg((inst >> 16 & 0xff) as u32, RZ, tile), word);
        }
    }
    pub fn movm(&mut self, inst: u128) {
        assert_eq!(inst >> 75 & 0x1f, 0, "MOVM.M832 silicon executes this word; outputs are in the session 6 tensor fixtures");
        let ty = self.type_u32[1];
        let [group, thread] = self.lane_quad();
        let [three, four, sixteen] = [3, 4, 16].map(|c| self.cached_const_u32(c));
        // Element (r, c) of the tile is half c & 1 of lane 4r + c / 2, so transposed row g, columns 2t and 2t + 1 come from lanes 8t + g / 2 and 8t + 4 + g / 2.
        let row_lanes = self.ir.emit_shift_left_logical(ty, thread, three);
        let column_lane = self.ir.emit_shift_right_logical(ty, group, self.const_u32_1);
        let first = self.ir.emit_iadd(ty, row_lanes, column_lane);
        let second = self.ir.emit_iadd(ty, first, four);
        let odd = self.ir.emit_bitwise_and(ty, group, self.const_u32_1);
        let shift = self.ir.emit_shift_left_logical(ty, odd, four);
        let own = self.load_register((inst >> 24 & 0xff) as u32);
        let halves: Vec<u32> = self.warp_exchange(own, &[first, second]).into_iter().map(|[word, _]| self.ir.emit_shift_right_logical(ty, word, shift)).collect();
        let word = self.ir.emit_bit_field_insert(ty, halves[0], halves[1], sixteen, sixteen);
        self.store_register((inst >> 16 & 0xff) as u32, word);
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
        todo!("SCATTER");
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
        todo!("SPMETADATA");
    }
}
