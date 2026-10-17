use super::*;

impl<'a> Decoder<'a> {
    // UP2UR and UR2UP move UP0..UP6 only, masked by imm32 (form 4) or URb.
    fn up_mask(&mut self, inst: u128) -> u32 {
        let mask = if inst >> 9 & 7 == 4 { self.cached_const_u32((inst >> 32) as u32) } else { self.load_ureg((inst >> 32 & 0x3f) as u32) };
        let predicates = self.cached_const_u32(P0_TO_P6);
        self.ir.emit_bitwise_and(self.type_u32[1], mask, predicates)
    }

    pub fn s2ur(&mut self, inst: u128) {
        let urd = (((inst >> 16) & 0x3f) << 0) as u32;
        let sr = (((inst >> 72) & 0xff) << 0) as u32;
        let val = self.special_register(sr);
        self.store_ureg(urd, val);
    }
    pub fn ubmsk(&mut self, inst: u128) {
        self.bmsk(inst)
    }
    pub fn ubrev(&mut self, inst: u128) {
        self.brev(inst)
    }
    pub fn uclea(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _sb = (((inst >> 32) & 0xffff) << 0);
        let _sz = (((inst >> 73) & 0xf) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!("UCLEA operand fields are public, but NAK and Eden provide no operation semantics");
    }
    pub fn uf2fp(&mut self, inst: u128) {
        let fmt = (inst >> 84 & 3) << 2 | (inst >> 75 & 3);
        assert!(fmt == 9, "UF2FP BF16, E8M7 and E6M9 formats need a bfloat16 type");
        let rnd = inst >> 79 & 7;
        assert!(matches!(rnd, 0 | 3), "UF2FP.RS stochastic rounding cannot be expressed in SPIR-V");
        let (b, c) = self.srcs_bc(inst);
        let b = self.bitcast_f32(b);
        let word = if inst >> 78 & 1 == 0 {
            let a = self.load_ureg((inst >> 24 & 0x3f) as u32);
            let a = self.bitcast_f32(a);
            self.pack_half2_round([b, a], rnd == 3)
        } else {
            let packed = self.pack_half2_round([b, b], rnd == 3);
            let c = self.high_half_if(c, inst >> 72 & 1 != 0);
            let sixteen = self.cached_const_u32(16);
            self.ir.emit_bit_field_insert(self.type_u32[1], packed, c, sixteen, sixteen)
        };
        self.store_ureg((inst >> 16 & 0x3f) as u32, word);
    }
    pub fn uflo(&mut self, inst: u128) {
        self.flo(inst)
    }
    pub fn uiadd3(&mut self, inst: u128) {
        self.iadd3(inst)
    }
    pub fn uiadd3_64(&mut self, inst: u128) {
        self.uiadd3(inst)
    }
    pub fn uimad(&mut self, inst: u128) {
        self.imad(inst)
    }
    pub fn uisetp(&mut self, inst: u128) {
        self.isetp(inst)
    }
    pub fn uldc(&mut self, inst: u128) {
        let urd = (((inst >> 16) & 0x3f) << 0) as u32;
        let ra_offset = (((inst >> 38) & 0xffff) << 0) as u32;
        let sc_bank = (((inst >> 54) & 0x1f) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        const ULDC: u32 = 0xab9;
        const ULDC_UR_BANK: u32 = 0x1ab9;
        const ULDC_UR_OFFSET: u32 = 0x1abb;
        const ULD: u32 = 0x18b8;
        let op = opcode(inst);
        let ura = (inst >> 24 & 0x3f) as u32;
        let offset = if op == ULD { (inst >> 38) as u32 } else { ra_offset };
        for i in 0..access_words(sz) {
            let offset = self.cached_const_u32(offset.wrapping_add(4 * i));
            let base = if matches!(op, ULD | ULDC_UR_OFFSET) { self.load_ureg(ura) } else { self.const_u32_0 };
            let address = self.ir.emit_iadd(self.type_u32[1], base, offset);
            let word = match op {
                ULDC | ULDC_UR_OFFSET => self.load_cbuf(ResourceKind::Cbuf(sc_bank), address),
                ULDC_UR_BANK => {
                    let bank = self.load_ureg(ura);
                    self.load_dynamic_cbuf(bank, address)
                }
                ULD => {
                    let ptr = self.global_word_at(address);
                    self.ir.emit_load(self.type_u32[1], ptr)
                }
                _ => unreachable!(),
            };
            let value = if sz <= 4 { self.load_narrow(word, address, sz) } else { word };
            self.store_ureg(nth_reg(urd, URZ, i), value);
        }
    }
    pub fn ulea(&mut self, inst: u128) {
        self.lea(inst)
    }
    pub fn ulop3(&mut self, inst: u128) {
        self.lop3(inst)
    }
    pub fn umov(&mut self, inst: u128) {
        let value = if inst >> 9 & 7 == 4 {
            self.cached_const_u32((inst >> 32) as u32)
        } else {
            self.load_ureg((inst >> 32 & 0x3f) as u32)
        };
        self.store_ureg((inst >> 16 & 0x3f) as u32, value);
    }
    pub fn up2ur(&mut self, inst: u128) {
        let ty = self.type_u32[1];
        let shift = self.cached_const_u32((inst >> 76 & 3) as u32 * 8);
        let mask = self.up_mask(inst);
        let selected = self.load_upredicate();
        let selected = self.ir.emit_bitwise_and(ty, selected, mask);
        let selected = self.ir.emit_shift_left_logical(ty, selected, shift);
        let mask = self.ir.emit_shift_left_logical(ty, mask, shift);
        let mask = self.ir.emit_not(ty, mask);
        let base = self.load_ureg((inst >> 24 & 0x3f) as u32);
        let kept = self.ir.emit_bitwise_and(ty, base, mask);
        let result = self.ir.emit_bitwise_or(ty, kept, selected);
        self.store_ureg((inst >> 16 & 0x3f) as u32, result);
    }
    pub fn uplop3(&mut self, inst: u128) {
        assert!(inst as u32 & 0xfff == UPLOP3, "UPLOP3 with register sources");
        self.plop3(inst)
    }
    pub fn upopc(&mut self, inst: u128) {
        self.popc(inst)
    }
    pub fn uprmt(&mut self, inst: u128) {
        self.prmt(inst)
    }
    pub fn ur2up(&mut self, inst: u128) {
        let ty = self.type_u32[1];
        let shift = self.cached_const_u32((inst >> 76 & 3) as u32 * 8);
        let mask = self.up_mask(inst);
        let value = self.load_ureg((inst >> 24 & 0x3f) as u32);
        let value = self.ir.emit_shift_right_logical(ty, value, shift);
        let value = self.ir.emit_bitwise_and(ty, value, mask);
        let mask = self.ir.emit_not(ty, mask);
        let kept = self.load_upredicate();
        let kept = self.ir.emit_bitwise_and(ty, kept, mask);
        let result = self.ir.emit_bitwise_or(ty, kept, value);
        self.store_upredicate(result);
    }
    pub fn usel(&mut self, inst: u128) {
        self.sel(inst)
    }
    pub fn usgxt(&mut self, inst: u128) {
        self.sgxt(inst)
    }
    pub fn ushf(&mut self, inst: u128) {
        self.shf(inst)
    }
}
