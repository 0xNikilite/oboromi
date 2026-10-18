use super::*;

pub enum BitSize {
    B32 = 0b00,
    B64 = 0b01,
    B96 = 0b10,
    B128 = 0b11,
}

impl<'a> Decoder<'a> {
    // %rd := %ra + $ra_offset
    pub fn al2p(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let ra = (((inst >> 24) & 0xff) << 0) as u32;
        let ra_offset = (((inst >> 40) & 0x7ff) << 0) as u32;
        let bop = (((inst >> 74) & 0x3) << 0) as u32;
        assert!(bop == BitSize::B32 as u32);
        let base = self.load_register(ra);
        let offset = self.ir.emit_constant_typed(self.type_u32[1], ra_offset as u32);
        let dst_val = self.ir.emit_iadd(self.type_u32[1], base, offset);
        self.store_register(rd, dst_val);
    }
    // Attribute addresses as the SPH IMAP/OMAP bits order them (clc797sph.h): position 0x70, generic n 0x80 + 16n, instance and vertex id 0x2f8 and 0x2fc; front facing 0x3fc is NAK's.
    pub fn ald(&mut self, inst: u128) {
        use spirv::{BuiltIn, storage_class::INPUT};
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let ra_offset = (((inst >> 40) & 0x3ff) << 0) as u32;
        let words = (((inst >> 74) & 0x3) << 0) as u32 + 1;
        assert!(opcode(inst) == 0x321 && inst >> 24 & 0xff == RZ as u128 && inst >> 76 & 0xf == 0, "only ALD of an input at an immediate attribute address is implemented");
        assert!(matches!(self.stage, Stage::Vertex { .. }), "ALD in a {:?} module is not implemented", self.stage);
        for i in 0..words {
            let attr = ra_offset + 4 * i;
            let val = match attr {
                0x80..0x280 => {
                    let kind = ResourceKind::VertexInput { location: (attr - 0x80) / 16 };
                    let ptr = self.interface_component(kind, INPUT, attr);
                    let ty = self.interface_type(kind, 1);
                    let val = self.ir.emit_load(ty, ptr);
                    if ty == self.type_u32[1] { val } else { self.bitcast_u32(val) }
                }
                0x2f8 => self.builtin(BuiltIn::InstanceIndex),
                0x2fc => self.builtin(BuiltIn::VertexIndex),
                _ => panic!("ALD a[{attr:#x}] is not implemented"),
            };
            self.store_register(nth_reg(rd, RZ, i), val);
        }
    }
    pub fn ast(&mut self, inst: u128) {
        use spirv::storage_class::OUTPUT;
        let rb = (((inst >> 32) & 0xff) << 0) as u32;
        let ra_offset = (((inst >> 40) & 0x3ff) << 0) as u32;
        let words = (((inst >> 74) & 0x3) << 0) as u32 + 1;
        assert!(opcode(inst) == 0x322 && inst >> 24 & 0xff == RZ as u128 && inst >> 76 & 3 == 0, "only AST at an immediate attribute address is implemented");
        assert!(matches!(self.stage, Stage::Vertex { .. }), "AST in a {:?} module is not implemented", self.stage);
        for i in 0..words {
            let attr = ra_offset + 4 * i;
            let kind = match attr {
                0x70..0x80 => ResourceKind::BuiltIn(spirv::BuiltIn::Position as u32),
                0x80..0x280 => ResourceKind::Varying { location: (attr - 0x80) / 16, flat: false },
                _ => panic!("AST a[{attr:#x}] is not implemented"),
            };
            let ptr = self.interface_component(kind, OUTPUT, attr);
            let val = self.load_register(nth_reg(rb, RZ, i));
            let val = self.bitcast_f32(val);
            self.ir.emit_store(ptr, val);
        }
    }
    pub fn cctl(&mut self, _inst: u128) {
        // Vulkan buffers are coherent without guest cache control.
    }
    pub fn cctll(&mut self, inst: u128) {
        self.cctl(inst)
    }
    pub fn cctlt(&mut self, _inst: u128) {}
    pub fn errbar(&mut self, _inst: u128) {}
    pub fn getlmembase(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let value = self.cached_const_u32(0xff00_0000);
        let ty = self.type_u64();
        let value = self.ir.emit_u_convert(ty, value);
        self.store_pair(rd, value);
    }
    pub fn ipa(&mut self, inst: u128) {
        use spirv::{BuiltIn, storage_class::INPUT};
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let attr = (((inst >> 64) & 0xff) << 2) as u32;
        let constant = (((inst >> 78) & 0x3) << 0) == 1;
        assert!(opcode(inst) == 0x326 && inst >> 76 & 3 == 0 && inst >> 78 & 3 < 2 && inst >> 81 & 7 == PT as u128, "only IPA.PASS and IPA.CONSTANT at the pixel centre are implemented");
        assert!(self.stage == Stage::Fragment, "IPA in a {:?} module is not implemented", self.stage);
        let val = match attr {
            0x70..0x80 => {
                let coord = self.builtin(BuiltIn::FragCoord);
                let val = self.ir.emit_composite_extract(self.type_f32[1], coord, &[attr / 4 % 4]);
                self.bitcast_u32(val)
            }
            // The PS header's IMAP, not IPA, picks perspective or screen linear, so perspective until the header is read.
            0x80..0x280 => {
                let ptr = self.interface_component(ResourceKind::Varying { location: (attr - 0x80) / 16, flat: constant }, INPUT, attr);
                let val = self.ir.emit_load(self.type_f32[1], ptr);
                self.bitcast_u32(val)
            }
            0x3fc => {
                let front = self.builtin(BuiltIn::FrontFacing);
                let all = self.cached_const_u32(u32::MAX);
                self.ir.emit_select(self.type_u32[1], front, all, self.const_u32_0)
            }
            _ => panic!("IPA a[{attr:#x}] is not implemented"),
        };
        self.store_register(rd, val);
    }
    pub fn isberd(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffff) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _merge = (((inst >> 78) & 0x1) << 0);
        let _op = (((inst >> 79) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn isbewr(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffff) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _merge = (((inst >> 78) & 0x1) << 0);
        let _op = (((inst >> 79) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ld(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let address = self.global_address(inst, 32, 32);
        self.load_memory(ResourceKind::Global, address, sz, rd);
    }
    pub fn ldc(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let ra = (((inst >> 24) & 0xff) << 0) as u32;
        let ra_offset = (((inst >> 38) & 0xffff) << 0) as u16 as i16 as u32;
        let sc_bank = (((inst >> 54) & 0x1f) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let stride = (((inst >> 78) & 0x3) << 0) as u32;
        const LDC_UR_BANK: u32 = 0x1582;
        let bindless = opcode(inst) == LDC_UR_BANK;
        let base = self.load_register(if bindless { (inst >> 64) as u32 & 0xff } else { ra });
        let mut bank = if bindless { self.load_ureg(ra & 0x3f) } else { self.cached_const_u32(sc_bank) };
        let base = if stride >= 2 {
            let sixteen = self.cached_const_u32(16);
            let segment = self.ir.emit_shift_right_logical(self.type_u32[1], base, sixteen);
            bank = self.ir.emit_iadd(self.type_u32[1], bank, segment);
            let mask = self.cached_const_u32(0xffff);
            self.ir.emit_bitwise_and(self.type_u32[1], base, mask)
        } else {
            base
        };
        for i in 0..access_words(sz) {
            let offset = self.cached_const_u32(ra_offset.wrapping_add(4 * i));
            let addr = self.ir.emit_iadd(self.type_u32[1], base, offset);
            let word = if bindless || stride >= 2 {
                self.load_dynamic_cbuf(bank, addr)
            } else {
                self.load_cbuf(ResourceKind::Cbuf(sc_bank), addr)
            };
            let value = if sz <= 4 { self.load_narrow(word, addr, sz) } else { word };
            self.store_register(nth_reg(rd, RZ, i), value);
        }
    }
    pub fn ldg(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let address = self.global_address(inst, 32, 40);
        self.load_memory(ResourceKind::Global, address, sz, rd);
    }
    pub fn ldgdepbar(&mut self, _inst: u128) {
        // LDGSTS completes synchronously in this model.
    }
    pub fn ldl(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let address = self.global_address(inst, 32, 40);
        self.load_memory(ResourceKind::Local, address, sz, rd);
    }
    pub fn ldtram(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn membar(&mut self, inst: u128) {
        let dstfmt = (((inst >> 76) & 0x7) << 0) as u32;
        use spirv::memory_semantics::{ACQUIRE_RELEASE, UNIFORM_MEMORY, WORKGROUP_MEMORY};
        let (scope, semantics) = match dstfmt {
            0 | 1 => (spirv::scope::WORKGROUP, ACQUIRE_RELEASE | WORKGROUP_MEMORY),
            _ => (spirv::scope::DEVICE, ACQUIRE_RELEASE | UNIFORM_MEMORY | WORKGROUP_MEMORY),
        };
        let scope = self.cached_const_u32(scope);
        let semantics = self.cached_const_u32(semantics);
        self.ir.emit_memory_barrier(scope, semantics);
    }
    pub fn out(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn pixld(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _mode = (((inst >> 78) & 0x7) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn setlmembase(&mut self, _inst: u128) {}
    pub fn st(&mut self, inst: u128) {
        let rc = (((inst >> 64) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let address = self.global_address(inst, 64, 32);
        self.store_memory(ResourceKind::Global, address, sz, rc);
    }
    pub fn stg(&mut self, inst: u128) {
        let rb = (((inst >> 32) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let address = self.global_address(inst, 64, 40);
        self.store_memory(ResourceKind::Global, address, sz, rb);
    }
    pub fn stl(&mut self, inst: u128) {
        let rb = (((inst >> 32) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let address = self.global_address(inst, 64, 40);
        self.store_memory(ResourceKind::Local, address, sz, rb);
    }
}
