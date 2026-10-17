use super::*;

impl<'a> Decoder<'a> {
    // No-ops: CSMTEST(external), IDE(host), QSPC(explicit), RTT(unsupported).
    pub fn cs2r(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let sr = (((inst >> 72) & 0xff) << 0) as u32;
        let wide = (((inst >> 80) & 0x1) << 0) as u32;
        for i in 0..=wide {
            let val = self.special_register(nth_reg(sr, SRZ, i));
            self.store_register(nth_reg(rd, RZ, i), val);
        }
    }
    pub fn csmtest(&mut self, _inst: u128) {}
    pub fn ide(&mut self, _inst: u128) {}
    pub fn mufu(&mut self, inst: u128) {
        use spirv::glsl::{COS, EXP2, INVERSE_SQRT, LOG2, SIN, SQRT, TANH};
        let ty = self.type_f32[1];
        let x = self.fp_alu_src(inst, false);
        let val = match inst >> 74 & 0xf {
            // Since SM70 SIN and COS take the angle in turns, nvcc scales by 1/(2*pi) first.
            op @ (0 | 1) => {
                let tau = self.ir.emit_constant_typed(ty, std::f32::consts::TAU);
                let radians = self.ir.emit_fmul(ty, x, tau);
                self.glsl(if op == 0 { COS } else { SIN }, ty, &[radians])
            }
            2 => self.glsl(EXP2, ty, &[x]),
            3 => self.glsl(LOG2, ty, &[x]),
            4 => {
                let one = self.ir.emit_constant_typed(ty, 1.0f32);
                self.ir.emit_fdiv(ty, one, x)
            }
            5 => self.glsl(INVERSE_SQRT, ty, &[x]),
            8 => self.glsl(SQRT, ty, &[x]),
            9 => self.glsl(TANH, ty, &[x]),
            op => panic!("MUFU op {op} (RCP64H, RSQ64H) is not implemented"),
        };
        let bits = self.bitcast_u32(val);
        // RCP flushes a subnormal result to zero.
        let bits = if inst >> 74 & 0xf == 4 { self.flush_subnormal(bits) } else { bits };
        self.store_fp_register((inst >> 16 & 0xff) as u32, bits);
    }
    pub fn nop(&mut self, _inst: u128) {
        self.ir.emit_nop();
    }
    pub fn pmtrig(&mut self, inst: u128) {
        if matches!(self.timing, Timing::Off) {
            return;
        }
        let buffer = self.hint_buffer();
        let ptr = self.ssbo_word(buffer, self.const_u32_0, (inst >> 32 & 0xffff) as u32);
        let [scope, relaxed] = [spirv::scope::DEVICE, spirv::memory_semantics::RELAXED].map(|v| self.cached_const_u32(v));
        self.ir.emit_atomic_iadd(self.type_u32[1], ptr, scope, relaxed, self.const_u32_1);
    }

    pub(super) fn hint_buffer(&mut self) -> u32 {
        self.resource(ResourceKind::Hints).unwrap_or_else(|| self.declare_ssbo(ResourceKind::Hints, 0))
    }
    pub fn qspc(&mut self, _inst: u128) {}
    pub fn rtt(&mut self, _inst: u128) {}
    pub fn s2r(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let sr = (((inst >> 72) & 0xff) << 0) as u32;
        let val = self.special_register(sr);
        self.store_register(rd, val);
    }
    pub fn setctaid(&mut self, _inst: u128) {
        panic!("SETCTAID cannot change the compute harness workgroup id");
    }
    pub fn ttucctl(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ttugo(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ttuld(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ttumacrofuse(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _sb = (((inst >> 40) & 0x1f) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ttuopen(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ttust(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
}
