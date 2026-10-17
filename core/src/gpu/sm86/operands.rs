use super::*;

impl<'a> Decoder<'a> {
    // Bits 11:9 say what bits 32..64 hold: Rb (1), imm32 (2, 4), c[bank][off] (3, 5), URb with bit 91 set (6, 7).
    // Forms 2, 3 and 7 feed that operand to C and move B to Rc at 64..72.
    pub(super) fn alu_src(&mut self, inst: u128) -> u32 {
        let field = (inst >> 32) as u32;
        match ((inst >> 9) & 7, (inst >> 91) & 1) {
            (1, 0) => self.load_register(field & 0xff),
            (2 | 4, 0) => self.cached_const_u32(field),
            (3 | 5, 0) => {
                let offset = self.cached_const_u32((field >> 8 & 0x3fff) << 2);
                self.load_cbuf(ResourceKind::Cbuf(field >> 22 & 0x1f), offset)
            }
            (6 | 7, 1) => self.load_ureg(field & 0x3f),
            form => todo!("ALU source form {form:?}"),
        }
    }

    pub(super) fn alu_reg(&mut self, inst: u128, bit: u32) -> u32 {
        let index = (inst >> bit) as u32;
        if is_uniform(inst) { self.load_ureg(index & 0x3f) } else { self.load_register(index & 0xff) }
    }

    pub(super) fn store_alu(&mut self, inst: u128, val: u32) {
        let rd = (inst >> 16) as u32;
        if is_uniform(inst) { self.store_ureg(rd & 0x3f, val) } else { self.store_register(rd & 0xff, val) }
    }

    pub(super) fn src_b(&mut self, inst: u128) -> u32 {
        match (is_uniform(inst), inst >> 9 & 7) {
            (true, 1) => self.alu_reg(inst, 32),
            (true, _) => self.cached_const_u32((inst >> 32) as u32),
            _ => self.alu_src(inst),
        }
    }

    pub(super) fn srcs_bc(&mut self, inst: u128) -> (u32, u32) {
        let (src, rc) = (self.src_b(inst), self.alu_reg(inst, 64));
        if matches!(inst >> 9 & 7, 2 | 3 | 7) { (rc, src) } else { (src, rc) }
    }

    pub(super) fn pred_bit(&mut self, uniform: bool, pred: u32, invert: bool) -> u32 {
        let bit = if pred == PT {
            self.const_u32_1
        } else {
            let preds = if uniform { self.load_upredicate() } else { self.load_predicate() };
            let offset = self.cached_const_u32(pred);
            self.ir.emit_bit_field_u_extract(self.type_u32[1], preds, offset, self.const_u32_1)
        };
        if invert { self.ir.emit_bitwise_xor(self.type_u32[1], bit, self.const_u32_1) } else { bit }
    }

    pub(super) fn pp_bit(&mut self, inst: u128) -> u32 {
        self.pred_bit(is_uniform(inst), (inst >> 87 & 7) as u32, inst >> 90 & 1 != 0)
    }

    pub(super) fn store_pred_bit(&mut self, uniform: bool, pred: u32, bit: u32) {
        if pred == PT {
            return;
        }
        let preds = if uniform { self.load_upredicate() } else { self.load_predicate() };
        let offset = self.cached_const_u32(pred);
        let preds = self.ir.emit_bit_field_insert(self.type_u32[1], preds, bit, offset, self.const_u32_1);
        if uniform { self.store_upredicate(preds) } else { self.store_predicate(preds) }
    }

    pub(super) fn alu_src_pair(&mut self, inst: u128) -> u32 {
        let field = (inst >> 32) as u32;
        let words = match ((inst >> 9) & 7, (inst >> 91) & 1) {
            (1, 0) => return self.load_pair(field & 0xff),
            (2 | 4, 0) => [self.const_u32_0, self.cached_const_u32(field)],
            (3 | 5, 0) => {
                let offset = (field >> 8 & 0x3fff) << 2;
                let bank = ResourceKind::Cbuf(field >> 22 & 0x1f);
                let lo_offset = self.cached_const_u32(offset);
                let hi_offset = self.cached_const_u32(offset + 4);
                [self.load_cbuf(bank, lo_offset), self.load_cbuf(bank, hi_offset)]
            }
            (6 | 7, 1) => return self.load_ureg_pair(field & 0x3f),
            form => todo!("64-bit ALU source form {form:?}"),
        };
        self.join_u64(words[0], words[1])
    }

    // Int64 is declared on first use so 32-bit shaders still run on devices without shaderInt64.
    pub(super) fn type_u64(&mut self) -> u32 {
        if self.type_u64 == 0 {
            self.ir.emit_capability(spirv::capability::INT64);
            self.type_u64 = self.ir.emit_type_int(64, 0);
        }
        self.type_u64
    }

    pub(super) fn type_s64(&mut self) -> u32 {
        if self.type_s64 == 0 {
            self.ir.emit_capability(spirv::capability::INT64);
            self.type_s64 = self.ir.emit_type_int(64, 1);
        }
        self.type_s64
    }

    pub(super) fn type_f64(&mut self) -> u32 {
        if self.type_f64 == 0 {
            self.ir.emit_capability(spirv::capability::FLOAT64);
            self.type_f64 = self.ir.emit_type_float(64);
        }
        self.type_f64
    }

    // Float16 is declared on first use so fp32 shaders still run on devices without shaderFloat16.
    pub(super) fn type_f16(&mut self) -> u32 {
        if self.type_f16 == 0 {
            self.ir.emit_capability(spirv::capability::FLOAT16);
            self.type_f16 = self.ir.emit_type_float(16);
        }
        self.type_f16
    }

    pub(super) fn type_f16x2(&mut self) -> u32 {
        if self.type_f16x2 == 0 {
            let half = self.type_f16();
            self.type_f16x2 = self.ir.emit_type_vector(half, 2);
        }
        self.type_f16x2
    }

    // Component 0 is bits 15:0 (H0), so bitcast_u32 is the inverse.
    pub(super) fn unpack_half2(&mut self, word: u32) -> u32 {
        let ty = self.type_f16x2();
        self.ir.emit_bitcast(ty, word)
    }

    // ISWZ: 0 = H1_H0, 2 = H0_H0, 3 = H1_H1; 1 (F32) reads a whole f32 and is not a swizzle.
    pub(super) fn half2_swizzle(&mut self, half2: u32, iswz: u32) -> u32 {
        let lanes = match iswz {
            0 => return half2,
            2 => [0, 0],
            3 => [1, 1],
            _ => panic!("ISWZ {iswz} is not a half2 swizzle"),
        };
        let ty = self.type_f16x2();
        self.ir.emit_vector_shuffle(ty, half2, half2, &lanes)
    }

    // Each half with a zero exponent keeps only its sign.
    fn flush_half2_subnormal(&mut self, word: u32) -> u32 {
        let ty = self.type_u32[1];
        let mut keep = self.const_u32_0;
        for half in [0xffff, 0xffff_0000] {
            let [exponent, half, sign] = [half & 0x7c00_7c00, half, half & 0x8000_8000].map(|c| self.cached_const_u32(c));
            let exp = self.ir.emit_bitwise_and(ty, word, exponent);
            let subnormal = self.ir.emit_iequal(self.type_bool[1], exp, self.const_u32_0);
            let lane = self.ir.emit_select(ty, subnormal, sign, half);
            keep = self.ir.emit_bitwise_or(ty, keep, lane);
        }
        self.ir.emit_bitwise_and(ty, word, keep)
    }

    // Lanes come out as exact f32 so the fp32 helpers apply; ISWZ 1 (F32) rounds the whole register to f16 for both lanes.
    pub(super) fn half2_lanes(&mut self, word: u32, iswz: u32, abs: bool, neg: bool, ftz: bool) -> [u32; 2] {
        let (half, half2_ty) = (self.type_f16(), self.type_f16x2());
        let (word, iswz) = if iswz == 1 {
            let val = self.bitcast_f32(word);
            let val = self.ir.emit_f_convert(half, val);
            let half2 = self.ir.emit_composite_construct(half2_ty, &[val, val]);
            (self.bitcast_u32(half2), 0)
        } else {
            (word, iswz)
        };
        let word = if ftz { self.flush_half2_subnormal(word) } else { word };
        let half2 = self.unpack_half2(word);
        let half2 = self.half2_swizzle(half2, iswz);
        [0, 1].map(|i| {
            let lane = self.ir.emit_composite_extract(half, half2, &[i]);
            let lane = self.ir.emit_f_convert(self.type_f32[1], lane);
            self.fp_src(self.type_f32[1], lane, abs, neg)
        })
    }

    pub(super) fn half2_reg(&mut self, inst: u128, reg_lsb: u32, iswz_lsb: u32, abs_bit: u32, neg_bit: u32, ftz: bool) -> [u32; 2] {
        let word = self.load_register((inst >> reg_lsb & 0xff) as u32);
        self.half2_lanes(word, (inst >> iswz_lsb & 3) as u32, inst >> abs_bit & 1 != 0, inst >> neg_bit & 1 != 0, ftz)
    }

    // An immediate (form 2 or 4) is two raw halves filling bits 32..64, so it has no swizzle (61:60), |x| (62) or -x (63).
    pub(super) fn half2_alu_src(&mut self, inst: u128, ftz: bool) -> [u32; 2] {
        let word = self.alu_src(inst);
        if matches!(inst >> 9 & 7, 2 | 4) {
            return self.half2_lanes(word, 0, false, false, ftz);
        }
        self.half2_lanes(word, (inst >> 60 & 3) as u32, inst >> 62 & 1 != 0, inst >> 63 & 1 != 0, ftz)
    }

    pub(super) fn f16_alu_src(&mut self, inst: u128, ftz: bool) -> u32 {
        let word = self.alu_src(inst);
        let imm = matches!(inst >> 9 & 7, 2 | 4);
        let word = self.high_half_if(word, !imm && inst >> 60 & 1 != 0);
        self.half2_lanes(word, 0, !imm && inst >> 62 & 1 != 0, !imm && inst >> 63 & 1 != 0, ftz)[0]
    }

    pub(super) fn pack_half2(&mut self, lanes: [u32; 2]) -> u32 {
        self.pack_half2_round(lanes, false)
    }

    pub(super) fn pack_half2_round(&mut self, lanes: [u32; 2], rtz: bool) -> u32 {
        let half = self.type_f16();
        let halves = lanes.map(|lane| self.ir.emit_f_convert(half, lane));
        let ty = self.type_f16x2();
        let half2 = self.ir.emit_composite_construct(ty, &halves);
        let mut packed = self.bitcast_u32(half2);
        let [width, canonical_nan] = [16, 0x7fff].map(|c| self.cached_const_u32(c));
        for i in 0..2 {
            let offset = self.cached_const_u32(16 * i as u32);
            if rtz {
                let back = self.ir.emit_f_convert(self.type_f32[1], halves[i]);
                let source = self.fp_src(self.type_f32[1], lanes[i], true, false);
                let back = self.fp_src(self.type_f32[1], back, true, false);
                let above = self.ir.emit_ford_greater_than(self.type_bool[1], back, source);
                let bits = self.ir.emit_bit_field_u_extract(self.type_u32[1], packed, offset, width);
                let down = self.ir.emit_isub(self.type_u32[1], bits, self.const_u32_1);
                let bits = self.ir.emit_select(self.type_u32[1], above, down, bits);
                packed = self.ir.emit_bit_field_insert(self.type_u32[1], packed, bits, offset, width);
            }
            let nan = self.ir.emit_f_is_nan(self.type_bool[1], lanes[i]);
            let canonical = self.ir.emit_bit_field_insert(self.type_u32[1], packed, canonical_nan, offset, width);
            packed = self.ir.emit_select(self.type_u32[1], nan, canonical, packed);
        }
        packed
    }

    pub(super) fn pack_f16_round(&mut self, val: u32, src_f64: bool, rnd: u32, ftz: bool) -> u32 {
        let half = self.type_f16();
        let nearest = self.ir.emit_f_convert(half, val);
        let zero_half = self.ir.emit_constant_typed(half, 0.0f32);
        let half2_ty = self.type_f16x2();
        let half2 = self.ir.emit_composite_construct(half2_ty, &[nearest, zero_half]);
        let mut bits = self.bitcast_u32(half2);
        if rnd != 0 {
            let src_ty = if src_f64 { self.type_f64() } else { self.type_f32[1] };
            let back = self.ir.emit_f_convert(src_ty, nearest);
            let less = self.ir.emit_ford_less_than(self.type_bool[1], back, val);
            let greater = self.ir.emit_ford_greater_than(self.type_bool[1], back, val);
            let zero = if src_f64 { self.ir.emit_constant_typed(src_ty, 0.0f64) } else { self.ir.emit_constant_typed(src_ty, 0.0f32) };
            let negative = self.ir.emit_ford_less_than(self.type_bool[1], val, zero);
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
            if let Some(cond) = use_up { bits = self.ir.emit_select(self.type_u32[1], cond, up, bits); }
            if let Some(cond) = use_down { bits = self.ir.emit_select(self.type_u32[1], cond, down, bits); }
        }
        let nan = self.ir.emit_f_is_nan(self.type_bool[1], val);
        let canonical = self.cached_const_u32(0x7fff);
        bits = self.ir.emit_select(self.type_u32[1], nan, canonical, bits);
        if ftz {
            let [exponent, sign] = [0x7c00, 0x8000].map(|c| self.cached_const_u32(c));
            let exp = self.ir.emit_bitwise_and(self.type_u32[1], bits, exponent);
            let subnormal = self.ir.emit_iequal(self.type_bool[1], exp, self.const_u32_0);
            let zero = self.ir.emit_bitwise_and(self.type_u32[1], bits, sign);
            bits = self.ir.emit_select(self.type_u32[1], subnormal, zero, bits);
        }
        bits
    }

    // .F32 writes lane 0 as one f32 instead of packing both lanes; .SAT maps NaN to +0 as in fp_arith_result.
    pub(super) fn store_half2(&mut self, inst: u128, lanes: [u32; 2], f32_out: bool, sat: bool, ftz: bool) {
        let lanes = if sat {
            let [zero, one] = [0.0f32, 1.0].map(|c| self.ir.emit_constant_typed(self.type_f32[1], c));
            lanes.map(|lane| self.glsl(spirv::glsl::N_CLAMP, self.type_f32[1], &[lane, zero, one]))
        } else {
            lanes
        };
        let bits = if f32_out { self.canonical_nan_bits(lanes[0]) } else { self.pack_half2(lanes) };
        let bits = match (ftz, f32_out) {
            (false, _) => bits,
            (true, true) => self.flush_subnormal(bits),
            (true, false) => self.flush_half2_subnormal(bits),
        };
        self.store_register((inst >> 16 & 0xff) as u32, bits);
    }

    pub(super) fn bool_op(&mut self, op: u128, a: u32, b: u32) -> u32 {
        let ty = self.type_u32[1];
        match op {
            0 => self.ir.emit_bitwise_and(ty, a, b),
            1 => self.ir.emit_bitwise_or(ty, a, b),
            2 => self.ir.emit_bitwise_xor(ty, a, b),
            _ => panic!("boolean op {op} is invalid"),
        }
    }

    pub(super) fn high_half_if(&mut self, word: u32, high: bool) -> u32 {
        if !high {
            return word;
        }
        let shift = self.cached_const_u32(16);
        self.ir.emit_shift_right_logical(self.type_u32[1], word, shift)
    }

    // OpBitcast puts vector component 0 in the low bits, so (Rn, Rn+1) is (lo, hi).
    pub(super) fn join_u64(&mut self, lo: u32, hi: u32) -> u32 {
        let pair = self.ir.emit_composite_construct(self.type_u32[2], &[lo, hi]);
        let ty = self.type_u64();
        self.ir.emit_bitcast(ty, pair)
    }

    pub(super) fn load_pair(&mut self, reg: u32) -> u32 {
        let [lo, hi] = [0, 1].map(|i| self.load_register(nth_reg(reg, RZ, i)));
        self.join_u64(lo, hi)
    }

    pub(super) fn load_ureg_pair(&mut self, ur: u32) -> u32 {
        let [lo, hi] = [0, 1].map(|i| self.load_ureg(nth_reg(ur, URZ, i)));
        self.join_u64(lo, hi)
    }

    pub(super) fn store_pair(&mut self, reg: u32, val: u32) {
        if reg == RZ {
            return;
        }
        let pair = self.ir.emit_bitcast(self.type_u32[2], val);
        for i in 0..2 {
            let word = self.ir.emit_composite_extract(self.type_u32[1], pair, &[i]);
            self.store_register(reg + i, word);
        }
    }

    pub(super) fn builtin(&mut self, builtin: spirv::BuiltIn) -> u32 {
        use spirv::{BuiltIn, decoration, storage_class::INPUT};
        let compute_only = matches!(builtin, BuiltIn::LocalInvocationIndex | BuiltIn::LocalInvocationId | BuiltIn::WorkgroupId);
        assert!(compute_only == matches!(self.stage, Stage::Compute { .. }), "{builtin:?} is not an input of a {:?} module", self.stage);
        let ty = match builtin {
            BuiltIn::LocalInvocationIndex | BuiltIn::VertexIndex | BuiltIn::InstanceIndex => self.type_u32[1],
            BuiltIn::InvocationId | BuiltIn::PrimitiveId | BuiltIn::PatchVertices | BuiltIn::SampleId => self.type_u32[1],
            BuiltIn::FragCoord => self.type_f32[4],
            BuiltIn::TessCoord => self.type_f32[3],
            BuiltIn::FrontFacing => self.type_bool[1],
            _ => self.type_u32[3],
        };
        let builtin = builtin as u32;
        let var = match self.resource(ResourceKind::BuiltIn(builtin)) {
            Some(var) => var,
            None => {
                let ptr = self.ir.emit_type_pointer(INPUT, ty);
                let var = self.ir.emit_variable(ptr, INPUT);
                self.ir.emit_decorate(var, decoration::BUILTIN, &[builtin]);
                if ty == self.type_u32[1] && matches!(self.stage, Stage::Fragment { .. }) {
                    self.ir.emit_decorate(var, decoration::FLAT, &[]);
                }
                self.resources.push(Resource { kind: ResourceKind::BuiltIn(builtin), var });
                var
            }
        };
        self.ir.emit_load(ty, var)
    }

    // The vertex fetch converts each attribute to its format's type, so an integer attribute reaches ALD as an integer.
    pub(super) fn interface_type(&self, kind: ResourceKind, width: usize) -> u32 {
        match (kind, self.stage) {
            (ResourceKind::VertexInput { location }, Stage::Vertex { uint, .. }) if uint >> location & 1 != 0 => self.type_u32[width],
            (ResourceKind::VertexInput { location }, Stage::Vertex { sint, .. }) if sint >> location & 1 != 0 => self.type_s32[width],
            _ => self.type_f32[width],
        }
    }

    pub(super) fn interface_var(&mut self, kind: ResourceKind, storage: u32) -> u32 {
        use spirv::{BuiltIn, decoration::{BUILTIN, FLAT, LOCATION, NO_PERSPECTIVE, PATCH}};
        if let Some(var) = self.resource(kind) {
            return var;
        }
        let tess_level = |builtin| builtin == BuiltIn::TessLevelOuter as u32 || builtin == BuiltIn::TessLevelInner as u32;
        let (ty, len) = match kind {
            ResourceKind::BuiltIn(builtin) if builtin == BuiltIn::TessLevelOuter as u32 => (self.type_f32[1], 4),
            ResourceKind::BuiltIn(builtin) if builtin == BuiltIn::TessLevelInner as u32 => (self.type_f32[1], 2),
            ResourceKind::BuiltIn(builtin) if builtin == BuiltIn::SampleMask as u32 => (self.type_u32[1], 1),
            ResourceKind::BuiltIn(builtin) if builtin == BuiltIn::Layer as u32 || builtin == BuiltIn::ViewportIndex as u32 => (self.type_u32[1], 0),
            ResourceKind::PerVertex { output, .. } => {
                let vertices = match self.stage {
                    Stage::Geometry { input, .. } => geometry_input(input).1,
                    Stage::TessControl { vertices } if output => vertices,
                    // maxTessellationPatchSize, which every Vulkan device supports at least as 32.
                    _ => 32,
                };
                (self.type_f32[4], vertices)
            }
            _ => (self.interface_type(kind, 4), 0),
        };
        let ty = if len == 0 { ty } else {
            let len = self.cached_const_u32(len);
            self.ir.emit_type_array(ty, len)
        };
        let ptr = self.ir.emit_type_pointer(storage, ty);
        let var = self.ir.emit_variable(ptr, storage);
        match kind {
            ResourceKind::BuiltIn(builtin) => self.ir.emit_decorate(var, BUILTIN, &[builtin]),
            ResourceKind::PerVertex { attr: 0x70, .. } => self.ir.emit_decorate(var, BUILTIN, &[BuiltIn::Position as u32]),
            ResourceKind::PerVertex { attr, .. } => self.ir.emit_decorate(var, LOCATION, &[(attr - 0x80) / 16]),
            ResourceKind::VertexInput { location } | ResourceKind::Varying { location, .. } | ResourceKind::FragmentOutput { location } | ResourceKind::Patch { location } => {
                self.ir.emit_decorate(var, LOCATION, &[location])
            }
            _ => unreachable!("{kind:?} is not a stage interface"),
        }
        match (kind, self.stage) {
            (ResourceKind::Varying { flat: true, .. }, _) => self.ir.emit_decorate(var, FLAT, &[]),
            (ResourceKind::Varying { location, .. }, Stage::Fragment { screen_linear, .. }) if screen_linear >> location & 1 != 0 => {
                self.ir.emit_decorate(var, NO_PERSPECTIVE, &[])
            }
            (ResourceKind::Patch { .. }, _) => self.ir.emit_decorate(var, PATCH, &[]),
            (ResourceKind::BuiltIn(builtin), _) if tess_level(builtin) => self.ir.emit_decorate(var, PATCH, &[]),
            (ResourceKind::BuiltIn(builtin), Stage::Fragment { .. }) if builtin == BuiltIn::SampleMask as u32 => self.ir.emit_decorate(var, FLAT, &[]),
            _ => {}
        }
        self.resources.push(Resource { kind, var });
        var
    }

    pub(super) fn frag_depth(&mut self) -> u32 {
        use spirv::{BuiltIn::FragDepth, decoration::BUILTIN, storage_class::OUTPUT};
        let kind = ResourceKind::BuiltIn(FragDepth as u32);
        if let Some(var) = self.resource(kind) {
            return var;
        }
        let ptr = self.ir.emit_type_pointer(OUTPUT, self.type_f32[1]);
        let var = self.ir.emit_variable(ptr, OUTPUT);
        self.ir.emit_decorate(var, BUILTIN, &[FragDepth as u32]);
        self.resources.push(Resource { kind, var });
        var
    }

    pub(super) fn interface_component(&mut self, kind: ResourceKind, storage: u32, attr: u32) -> u32 {
        self.vertex_component(kind, storage, &[], attr)
    }

    pub(super) fn vertex_component(&mut self, kind: ResourceKind, storage: u32, vertex: &[u32], attr: u32) -> u32 {
        let var = self.interface_var(kind, storage);
        let ptr = self.ir.emit_type_pointer(storage, self.interface_type(kind, 1));
        let component = self.cached_const_u32(attr / 4 % 4);
        self.ir.emit_access_chain(ptr, var, &[vertex, &[component]].concat())
    }

    pub(super) fn special_register(&mut self, index: u32) -> u32 {
        use spirv::BuiltIn;
        // SR_VIRTID holds the warp id at bit 8, SR_TID packs x, y and z at bits 0, 16 and 26. TCS SASS indexes per-vertex outputs by
        // lane and TES SASS reads TessCoord at a[SR_LANEID] (NAK nak_nir_lower_vtg_io.c, from_nir.rs), so a patch's invocations take
        // lanes from 0; SR_PRIM_TYPE holds the patch size at bit 8 (NAK nak_nir.c). ISBERD takes vertex v of primitive i in the warp at
        // i * n + v from SR_INVOCATION_INFO, n at bit 16 and i at bit 0 (driver GS, TCS and TES SASS), and here i is 0.
        match index {
            0 | 17 if matches!(self.stage, Stage::TessControl { .. }) => self.builtin(BuiltIn::InvocationId),
            17 if matches!(self.stage, Stage::Geometry { .. }) => self.builtin(BuiltIn::InvocationId),
            0 if matches!(self.stage, Stage::TessEval { .. }) => self.const_u32_0,
            16 | 29 if matches!(self.stage, Stage::TessControl { .. } | Stage::TessEval { .. }) || index == 29 && matches!(self.stage, Stage::Geometry { .. }) => {
                let vertices = match self.stage {
                    Stage::Geometry { input, .. } => self.cached_const_u32(geometry_input(input).1),
                    _ => self.builtin(BuiltIn::PatchVertices),
                };
                let shift = self.cached_const_u32(if index == 16 { 8 } else { 16 });
                self.ir.emit_shift_left_logical(self.type_u32[1], vertices, shift)
            }
            0 => {
                let invocation = self.builtin(BuiltIn::LocalInvocationIndex);
                let mask = self.cached_const_u32(31);
                self.ir.emit_bitwise_and(self.type_u32[1], invocation, mask)
            }
            3 => {
                let invocation = self.builtin(BuiltIn::LocalInvocationIndex);
                let [five, eight] = [5, 8].map(|i| self.cached_const_u32(i));
                let warp = self.ir.emit_shift_right_logical(self.type_u32[1], invocation, five);
                self.ir.emit_shift_left_logical(self.type_u32[1], warp, eight)
            }
            32 => {
                let tid = self.builtin(BuiltIn::LocalInvocationId);
                let x = self.ir.emit_composite_extract(self.type_u32[1], tid, &[0]);
                let y = self.ir.emit_composite_extract(self.type_u32[1], tid, &[1]);
                let z = self.ir.emit_composite_extract(self.type_u32[1], tid, &[2]);
                let [o16, c10, o26, c6] = [16, 10, 26, 6].map(|i| self.cached_const_u32(i));
                let xy = self.ir.emit_bit_field_insert(self.type_u32[1], x, y, o16, c10);
                self.ir.emit_bit_field_insert(self.type_u32[1], xy, z, o26, c6)
            }
            33..=35 => {
                let tid = self.builtin(BuiltIn::LocalInvocationId);
                self.ir.emit_composite_extract(self.type_u32[1], tid, &[index - 33])
            }
            37..=39 => {
                let ctaid = self.builtin(BuiltIn::WorkgroupId);
                self.ir.emit_composite_extract(self.type_u32[1], ctaid, &[index - 37])
            }
            56..=60 => {
                let ty = self.type_u32[1];
                let lane = self.special_register(0);
                let eq = self.ir.emit_shift_left_logical(ty, self.const_u32_1, lane);
                let lt = self.ir.emit_isub(ty, eq, self.const_u32_1);
                let le = self.ir.emit_bitwise_or(ty, lt, eq);
                match index {
                    56 => eq,
                    57 => lt,
                    58 => le,
                    59 => self.ir.emit_not(ty, le),
                    _ => self.ir.emit_not(ty, lt),
                }
            }
            1 | 2 | 4..=31 | 36 | 40..=55 | 61..=133 | SRZ => self.const_u32_0,
            _ => panic!("unknown special register {index}"),
        }
    }

    pub(super) fn bitcast_f32(&mut self, val: u32) -> u32 {
        self.ir.emit_bitcast(self.type_f32[1], val)
    }

    pub(super) fn bitcast_u32(&mut self, val: u32) -> u32 {
        self.ir.emit_bitcast(self.type_u32[1], val)
    }

    pub(super) fn bitcast_f64(&mut self, val: u32) -> u32 {
        let ty = self.type_f64();
        self.ir.emit_bitcast(ty, val)
    }

    pub(super) fn glsl(&mut self, op: u32, ty: u32, args: &[u32]) -> u32 {
        if self.glsl_ext == 0 {
            self.glsl_ext = self.ir.emit_ext_inst_import("GLSL.std.450");
        }
        self.ir.emit_ext_inst(ty, self.glsl_ext, op, args)
    }

    pub(super) fn fp_src(&mut self, ty: u32, val: u32, abs: bool, neg: bool) -> u32 {
        let val = if abs { self.glsl(spirv::glsl::F_ABS, ty, &[val]) } else { val };
        if neg { self.ir.emit_fnegate(ty, val) } else { val }
    }

    pub(super) fn fp64_alu_src(&mut self, inst: u128) -> u32 {
        let ty = self.type_f64();
        let bits = self.alu_src_pair(inst);
        let val = self.ir.emit_bitcast(ty, bits);
        let imm = matches!(inst >> 9 & 7, 2 | 4);
        self.fp_src(ty, val, !imm && inst >> 62 & 1 != 0, !imm && inst >> 63 & 1 != 0)
    }

    pub(super) fn fp64_reg(&mut self, inst: u128, reg_lsb: u32, abs_bit: u32, neg_bit: u32) -> u32 {
        let bits = self.load_pair((inst >> reg_lsb & 0xff) as u32);
        let val = self.bitcast_f64(bits);
        let ty = self.type_f64();
        self.fp_src(ty, val, inst >> abs_bit & 1 != 0, inst >> neg_bit & 1 != 0)
    }

    // FTZ keeps the sign: a subnormal becomes the zero of the same sign.
    pub(super) fn flush_subnormal(&mut self, bits: u32) -> u32 {
        let [exponent, sign] = [0x7f80_0000, 0x8000_0000].map(|c| self.cached_const_u32(c));
        let exp = self.ir.emit_bitwise_and(self.type_u32[1], bits, exponent);
        let subnormal = self.ir.emit_iequal(self.type_bool[1], exp, self.const_u32_0);
        let zero = self.ir.emit_bitwise_and(self.type_u32[1], bits, sign);
        self.ir.emit_select(self.type_u32[1], subnormal, zero, bits)
    }

    pub(super) fn fp_operand(&mut self, bits: u32, abs: bool, neg: bool, ftz: bool) -> u32 {
        let bits = if ftz { self.flush_subnormal(bits) } else { bits };
        let val = self.bitcast_f32(bits);
        self.fp_src(self.type_f32[1], val, abs, neg)
    }

    pub(super) fn fp_reg(&mut self, inst: u128, reg_lsb: u32, abs_bit: u32, neg_bit: u32, ftz: bool) -> u32 {
        let bits = self.load_register((inst >> reg_lsb & 0xff) as u32);
        self.fp_operand(bits, inst >> abs_bit & 1 != 0, inst >> neg_bit & 1 != 0, ftz)
    }

    // An immediate fills all of bits 32..64, so it has no |x| (62) or -x (63) bit.
    pub(super) fn fp_alu_src(&mut self, inst: u128, ftz: bool) -> u32 {
        let bits = self.alu_src(inst);
        let imm = matches!(inst >> 9 & 7, 2 | 4);
        self.fp_operand(bits, !imm && inst >> 62 & 1 != 0, !imm && inst >> 63 & 1 != 0, ftz)
    }

    pub(super) fn canonical_nan_bits(&mut self, val: u32) -> u32 {
        let nan = self.ir.emit_f_is_nan(self.type_bool[1], val);
        let canonical = self.cached_const_u32(0x7fff_ffff);
        let bits = self.bitcast_u32(val);
        self.ir.emit_select(self.type_u32[1], nan, canonical, bits)
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::run_body;

    #[test]
    fn half2_lanes_round_trip_through_a_register() {
        if !crate::gpu::device().is_ok_and(|d| d.shader_float16) {
            return eprintln!("skipping: no shaderFloat16");
        }
        let program = |d: &mut super::Decoder| {
            let half2 = d.load_register(0);
            let half2 = d.unpack_half2(half2);
            let ty = d.type_f16x2();
            let sum = d.ir.emit_fadd(ty, half2, half2);
            let packed = d.bitcast_u32(sum);
            d.store_register(1, packed);
            let hi = d.half2_swizzle(sum, 3);
            let hi = d.bitcast_u32(hi);
            d.store_register(2, hi);
        };
        let Some((lanes, _)) = run_body(program, &[vec![0xC000_3E00]], &[], &[]) else {
            return;
        };
        assert_eq!(lanes[0][1..3], [0xC400_4200, 0xC400_C400]);
    }
}
