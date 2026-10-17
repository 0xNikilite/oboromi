use super::*;

impl<'a> Decoder<'a> {
    pub fn cs2r(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let sr = (((inst >> 72) & 0xff) << 0) as u32;
        let wide = (((inst >> 80) & 0x1) << 0) as u32;
        for i in 0..=wide {
            let val = self.special_register(nth_reg(sr, SRZ, i));
            self.store_register(nth_reg(rd, RZ, i), val);
        }
    }
    // CSMTEST votes on the clip space masks (CCMP F, CSM_TA/TR/MX, T, FCSM_TA/TR/MX in sm_86_instructions.txt) of the
    // vertex batch, whose semantics are not public. Only T reads true, so a driver VS never takes its `@P0 EXIT` on
    // FCSM_TR (trivially rejected) and always writes the position, which Vulkan then clips and culls itself.
    pub fn csmtest(&mut self, inst: u128) {
        let result = if inst >> 76 & 7 == 4 { self.const_u32_1 } else { self.const_u32_0 };
        self.store_setp(inst, result);
    }
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
        let bits = self.canonical_nan_bits(val);
        // RCP flushes a subnormal result to zero.
        let bits = if inst >> 74 & 0xf == 4 { self.flush_subnormal(bits) } else { bits };
        self.store_register((inst >> 16 & 0xff) as u32, bits);
    }
    pub fn nop(&mut self, _inst: u128) {
        self.ir.emit_nop();
    }
    pub fn pmtrig(&mut self, _inst: u128) {}
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
}
