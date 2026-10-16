use crate::tests::gpu::harness::translate_module;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub message: String,
    pub duration: Duration,
}

// helpers to build sm86 instructions
#[doc(hidden)]
pub mod inst {
    const IADD3_IMM: u128 = 0x810;
    const IADD3_RRR: u128 = 0x210;
    const MOV_RI: u128 = 0x802;
    const MOV_RR: u128 = 0x202;
    const MOV_RC: u128 = 0xa02;
    const MOV_RU: u128 = 0xc02 | 1 << 91;
    const KILL: u128 = 0x95b;
    const GUARD: u128 = 0xf << 12;
    const S2R: u128 = 0x919;
    const CS2R: u128 = 0x805;
    const S2UR: u128 = 0x9c3;
    const PT_GUARD: u128 = 7 << 12;
    const RC_RZ: u128 = 0xff << 64;
    const PIXMASK_ALL: u128 = 0xf << 72;
    const IADD3_NO_CARRY: u128 = 0x3fff << 77;
    const PP_PT: u128 = 7 << 87;
    const NO_SCOREBOARD: u128 = 0x3f << 110;
    const FADD_RR: u128 = 0x221;
    const FADD_RI: u128 = 0x421;
    const FMUL_RR: u128 = 0x220;
    const FMUL_RI: u128 = 0x820;
    const FMUL_NOSCALE: u128 = 4 << 84;
    const FFMA_RRR: u128 = 0x223;
    const FFMA_RRI: u128 = 0x423;
    const FFMA_RIR: u128 = 0x823;
    const FMNMX_RR: u128 = 0x209;
    const MUFU_R: u128 = 0x308;
    pub const FP_ABS_B: u128 = 1 << 62;
    pub const FP_NEG_B: u128 = 1 << 63;
    pub const FP_NEG_A: u128 = 1 << 72;
    pub const FP_ABS_A: u128 = 1 << 73;
    pub const FP_NEG_C: u128 = 1 << 75;
    pub const FP_FMZ: u128 = 1 << 76;
    pub const FP_SAT: u128 = 1 << 77;
    pub const FP_FTZ: u128 = 1 << 80;
    pub const MUFU_COS: u32 = 0;
    pub const MUFU_SIN: u32 = 1;
    pub const MUFU_EX2: u32 = 2;
    pub const MUFU_LG2: u32 = 3;
    pub const MUFU_RCP: u32 = 4;
    pub const MUFU_RSQ: u32 = 5;
    pub const MUFU_SQRT: u32 = 8;
    pub const MUFU_TANH: u32 = 9;
    const LDG: u128 = 0x381;
    const STG: u128 = 0x386;
    const LDC: u128 = 0xb82;
    const ULDC: u128 = 0xab9;
    const ADDR_64: u128 = 1 << 72;
    const SZ_32: u128 = 4 << 73;
    const SZ_64: u128 = 5 << 73;
    const LDG_PU_PT: u128 = 7 << 81;
    const COP_EN: u128 = 1 << 84;
    const NO_WRITE_SCOREBOARD: u128 = 7 << 110;
    const BRA: u128 = 0x947;
    const BRA_UREG: u128 = 0x947 | 1 << 91;
    const BRA_UPRED: u128 = 0x547 | 1 << 91;
    const BRX: u128 = 0x949;
    const JMX: u128 = 0x94c;
    const BSSY: u128 = 0x945;
    const BSYNC: u128 = 0x941;
    const CALL_REL: u128 = 0x944;
    const RET: u128 = 0x950;
    const EXIT: u128 = 0x94d;
    const COND_U: u128 = 1 << 32;
    const COND_DIV: u128 = 2 << 32;
    const COND_CONV: u128 = 3 << 32;
    const URB_INVERT: u128 = 1 << 30;
    const URZ: u32 = 63;
    const F2F: u128 = 0x310;
    const F2I: u128 = 0x305;
    const F2I_64: u128 = 0x311;
    const FRND: u128 = 0x307;
    const I2F: u128 = 0x306;
    const I2F_64: u128 = 0x312;
    const I2I: u128 = 0x238;
    const RET_NODEC: u128 = 1 << 86;

    pub fn iadd_imm(rd: u32, ra: u32, imm32: u32) -> u128 {
        let mut inst = IADD3_IMM | PT_GUARD | IADD3_NO_CARRY | RC_RZ;
        inst |= ((rd as u128) & 0xff) << 16;
        inst |= ((ra as u128) & 0xff) << 24;
        inst |= ((imm32 as u128) & 0xffffffff) << 32;
        inst
    }

    pub fn iadd_reg(rd: u32, ra: u32, rb: u32) -> u128 {
        iadd3_reg(rd, ra, rb, 0xff)
    }

    pub fn iadd3_reg(rd: u32, ra: u32, rb: u32, rc: u32) -> u128 {
        let mut inst = IADD3_RRR | PT_GUARD | IADD3_NO_CARRY;
        inst |= ((rd as u128) & 0xff) << 16;
        inst |= ((ra as u128) & 0xff) << 24;
        inst |= ((rb as u128) & 0xff) << 32;
        inst |= ((rc as u128) & 0xff) << 64;
        inst
    }

    pub fn mov_imm(rd: u32, imm32: u32) -> u128 {
        let mut inst = MOV_RI | PT_GUARD | PIXMASK_ALL;
        inst |= ((rd as u128) & 0xff) << 16;
        inst |= ((imm32 as u128) & 0xffffffff) << 32;
        inst
    }

    pub fn mov_cbuf(rd: u32, bank: u32, off: u32) -> u128 {
        let mut inst = MOV_RC | PT_GUARD | PIXMASK_ALL;
        inst |= ((rd as u128) & 0xff) << 16;
        inst |= ((off as u128 >> 2) & 0x3fff) << 40;
        inst |= ((bank as u128) & 0x1f) << 54;
        inst
    }

    pub fn mov_ureg(rd: u32, urb: u32) -> u128 {
        let mut inst = MOV_RU | PT_GUARD | PIXMASK_ALL;
        inst |= ((rd as u128) & 0xff) << 16;
        inst |= ((urb as u128) & 0x3f) << 32;
        inst
    }

    pub fn mov_reg(rd: u32, rb: u32) -> u128 {
        let mut inst = MOV_RR | PT_GUARD | PIXMASK_ALL;
        inst |= ((rd as u128) & 0xff) << 16;
        inst |= ((rb as u128) & 0xff) << 32;
        inst
    }

    pub fn ldg(rd: u32, ra: u32, off: u32) -> u128 {
        let mut inst = LDG | PT_GUARD | ADDR_64 | SZ_32 | LDG_PU_PT | COP_EN;
        inst |= ((rd as u128) & 0xff) << 16;
        inst |= ((ra as u128) & 0xff) << 24;
        inst |= ((off as u128) & 0xff_ffff) << 40;
        inst
    }

    pub fn stg(ra: u32, off: u32, rb: u32) -> u128 {
        let mut inst = STG | PT_GUARD | ADDR_64 | SZ_32 | COP_EN | NO_WRITE_SCOREBOARD;
        inst |= ((ra as u128) & 0xff) << 24;
        inst |= ((rb as u128) & 0xff) << 32;
        inst |= ((off as u128) & 0xff_ffff) << 40;
        inst
    }

    pub fn ldc(rd: u32, bank: u32, ra: u32, off: u32) -> u128 {
        let mut inst = LDC | PT_GUARD | SZ_32;
        inst |= ((rd as u128) & 0xff) << 16;
        inst |= ((ra as u128) & 0xff) << 24;
        inst |= ((off as u128) & 0xffff) << 38;
        inst |= ((bank as u128) & 0x1f) << 54;
        inst
    }

    pub fn uldc64(urd: u32, bank: u32, off: u32) -> u128 {
        let mut inst = ULDC | PT_GUARD | SZ_64;
        inst |= ((urd as u128) & 0x3f) << 16;
        inst |= ((off as u128) & 0xffff) << 38;
        inst |= ((bank as u128) & 0x1f) << 54;
        inst
    }

    pub fn s2r(rd: u32, sr: u32) -> u128 {
        S2R | PT_GUARD | ((rd as u128) & 0xff) << 16 | ((sr as u128) & 0xff) << 72
    }

    pub fn cs2r(rd: u32, sr: u32, wide: bool) -> u128 {
        CS2R | PT_GUARD | ((rd as u128) & 0xff) << 16 | ((sr as u128) & 0xff) << 72 | (wide as u128) << 80
    }

    pub fn s2ur(urd: u32, sr: u32) -> u128 {
        S2UR | PT_GUARD | ((urd as u128) & 0x3f) << 16 | ((sr as u128) & 0xff) << 72
    }

    pub fn kill(pred: u32, invert: bool) -> u128 {
        let mut inst = KILL | PP_PT | NO_SCOREBOARD;
        inst |= ((pred as u128) & 0x7) << 12;
        if invert {
            inst |= 1 << 15;
        }
        inst
    }

    fn control(op: u128, ra: u32, offset: i64) -> u128 {
        op | PT_GUARD | PP_PT | NO_SCOREBOARD | (ra as u128 & 0xff) << 24 | ((offset as u128 >> 2) & 0xffff_ffff_ffff) << 34
    }

    pub fn bra(offset: i64) -> u128 {
        control(BRA, 0, offset)
    }

    pub fn bra_upred(upq: u32, invert: bool, offset: i64) -> u128 {
        control(BRA_UPRED | COND_U, upq | (invert as u32) << 3, offset)
    }

    pub fn bra_div(offset: i64) -> u128 {
        control(BRA_UREG | COND_DIV | URB_INVERT, URZ, offset)
    }

    pub fn bra_conv(offset: i64) -> u128 {
        control(BRA_UREG | COND_CONV | URB_INVERT, URZ, offset)
    }

    pub fn brx(ra: u32, offset: i64) -> u128 {
        control(BRX, ra, offset)
    }

    pub fn jmx(ra: u32, offset: i64) -> u128 {
        control(JMX, ra, offset)
    }

    pub fn bssy(bar: u32, offset: i64) -> u128 {
        control(BSSY, 0, offset) | (bar as u128) << 16
    }

    pub fn bsync(bar: u32) -> u128 {
        control(BSYNC, 0, 0) | (bar as u128) << 16
    }

    pub fn call_rel(offset: i64) -> u128 {
        control(CALL_REL, 0, offset)
    }

    pub fn ret_rel(ra: u32, offset: i64) -> u128 {
        control(RET | RET_NODEC, ra, offset)
    }

    pub fn exit() -> u128 {
        control(EXIT, 0, 0)
    }

    pub fn alu(op: u128, rd: u32, ra: u32, b: u32) -> u128 {
        op | PT_GUARD | (rd as u128) << 16 | (ra as u128) << 24 | (b as u128) << 32
    }

    pub fn fadd(rd: u32, ra: u32, rb: u32) -> u128 {
        alu(FADD_RR, rd, ra, rb)
    }

    pub fn fadd_imm(rd: u32, ra: u32, imm: f32) -> u128 {
        alu(FADD_RI, rd, ra, imm.to_bits()) | RC_RZ
    }

    pub fn fmul(rd: u32, ra: u32, rb: u32) -> u128 {
        alu(FMUL_RR, rd, ra, rb) | FMUL_NOSCALE
    }

    pub fn fmul_imm(rd: u32, ra: u32, imm: f32) -> u128 {
        alu(FMUL_RI, rd, ra, imm.to_bits()) | FMUL_NOSCALE
    }

    pub fn ffma(rd: u32, ra: u32, rb: u32, rc: u32) -> u128 {
        alu(FFMA_RRR, rd, ra, rb) | (rc as u128) << 64
    }

    pub fn ffma_imm_b(rd: u32, ra: u32, imm: f32, rc: u32) -> u128 {
        alu(FFMA_RIR, rd, ra, imm.to_bits()) | (rc as u128) << 64
    }

    pub fn ffma_imm_c(rd: u32, ra: u32, rb: u32, imm: f32) -> u128 {
        alu(FFMA_RRI, rd, ra, imm.to_bits()) | (rb as u128) << 64
    }

    pub fn fmnmx(rd: u32, ra: u32, rb: u32, pp: u32, pp_not: bool) -> u128 {
        alu(FMNMX_RR, rd, ra, rb) | (pp as u128) << 87 | (pp_not as u128) << 90
    }

    pub fn mufu(rd: u32, op: u32, rb: u32) -> u128 {
        alu(MUFU_R, rd, 0, rb) | (op as u128) << 74
    }

    pub mod cvt {
        pub const F32: u32 = 2;
        pub const F64: u32 = 3;
        pub const U8: u32 = 0;
        pub const S8: u32 = 1;
        pub const U32: u32 = 4;
        pub const S32: u32 = 5;
        pub const U64: u32 = 6;
        pub const S64: u32 = 7;
        pub const RN: u32 = 0;
        pub const RM: u32 = 1;
        pub const RP: u32 = 2;
        pub const RZ: u32 = 3;
    }

    pub fn f2f(rd: u32, rb: u32, dst: u32, src: u32, rnd: u32, ftz: bool) -> u128 {
        alu(F2F, rd, 0, rb) | (dst as u128) << 75 | (src as u128) << 84 | (rnd as u128) << 78 | (ftz as u128) << 80
    }

    pub fn f2i(rd: u32, rb: u32, dst: u32, src: u32, rnd: u32) -> u128 {
        let op = if src == cvt::F64 { F2I_64 } else { F2I };
        alu(op, rd, 0, rb) | (dst as u128 & 1) << 72 | (dst as u128 >> 1) << 75 | (src as u128) << 84 | (rnd as u128) << 78
    }

    pub fn frnd(rd: u32, rb: u32, rnd: u32) -> u128 {
        alu(FRND, rd, 0, rb) | (cvt::F32 as u128) << 75 | (cvt::F32 as u128) << 84 | (rnd as u128) << 78
    }

    pub fn i2f(rd: u32, rb: u32, dst: u32, src: u32) -> u128 {
        let op = if dst == cvt::F64 || src >= cvt::U64 { I2F_64 } else { I2F };
        alu(op, rd, 0, rb) | (dst as u128) << 75 | (src as u128 & 1) << 74 | (src as u128 >> 1) << 84
    }

    pub fn i2i(rd: u32, rb: u32, dst: u32) -> u128 {
        alu(I2I, rd, 0, rb) | (dst as u128) << 76
    }

    pub fn guard(inst: u128, pred: u32, invert: bool) -> u128 {
        inst & !GUARD | (pred as u128 & 0x7) << 12 | (invert as u128) << 15
    }

    const UNIFORM: u128 = 1 << 91;
    const PU_PT: u128 = 7 << 81;
    pub const LOP3: u128 = 0x212;
    pub const ULOP3: u128 = 0x292 | UNIFORM;
    pub const SHF: u128 = 0x219;
    pub const USHF: u128 = 0x299 | UNIFORM;
    pub const PRMT: u128 = 0x216;
    pub const UPRMT: u128 = 0x296 | UNIFORM;
    pub const POPC: u128 = 0x309;
    pub const UPOPC: u128 = 0x2bf | UNIFORM;
    pub const FLO: u128 = 0x300 | PU_PT;
    pub const UFLO: u128 = 0x2bd | UNIFORM | PU_PT;
    pub const BREV: u128 = 0x301;
    pub const UBREV: u128 = 0x2be | UNIFORM;
    pub const BMSK: u128 = 0x21b;
    pub const UBMSK: u128 = 0x29b | UNIFORM;
    pub const SGXT: u128 = 0x21a;
    pub const USGXT: u128 = 0x29a | UNIFORM;
    // XOR into a register-B opcode: form 4 takes B as imm32, form 2 takes C as imm32 and B from the Rc slot.
    pub const IMM_B: u128 = 5 << 9;
    pub const IMM_C: u128 = 3 << 9;
    pub const INVERT_B: u128 = 1 << 63;
    pub const PAND: u128 = 1 << 80;
    pub const S32: u128 = 1 << 73;
    pub const FLO_SH: u128 = 1 << 74;
    pub const WRAP: u128 = 1 << 75;
    pub const SHF_U64: u128 = 1 << 73;
    pub const SHF_S32: u128 = 2 << 73;
    pub const SHF_U32: u128 = 3 << 73;
    pub const SHF_R: u128 = 1 << 76;
    pub const SHF_HI: u128 = 1 << 80;

    pub fn alu3(op: u128, rd: u32, ra: u32, b: u32, rc: u32) -> u128 {
        alu(op, rd, ra, b) | (rc as u128) << 64
    }

    pub fn lop3_fields(lut: u8, pu: u32, pp: u32, pp_not: bool) -> u128 {
        (lut as u128) << 72 | (pu as u128) << 81 | (pp as u128) << 87 | (pp_not as u128) << 90
    }

    pub fn prmt_mode(mode: u32) -> u128 {
        (mode as u128) << 72
    }

    pub const UIADD3: u128 = 0x290 | UNIFORM;
    pub const UIADD3_64: u128 = 0x297 | UNIFORM;
    pub const UIMAD: u128 = 0x2a4 | UNIFORM;
    pub const UIMAD_WIDE: u128 = 0x2a5 | UNIFORM;
    pub const UISETP: u128 = 0x28c | UNIFORM;
    pub const ULEA: u128 = 0x291 | UNIFORM;
    pub const USEL: u128 = 0x287 | UNIFORM;
    pub const UMOV: u128 = 0xc82 | UNIFORM;
    pub const UMOV_IMM: u128 = 0x882;
    pub const UP2UR_IMM: u128 = 0x883 | UNIFORM;
    pub const UR2UP_IMM: u128 = 0x884 | UNIFORM;
    pub const UPLOP3: u128 = 0x89c;
    pub const R2UR: u128 = 0x3c2 | PU_PT;
    pub const NEG_A: u128 = 1 << 72;
    pub const EXTENDED: u128 = 1 << 74;
    pub const ICMP_LT: u128 = 1 << 76;

    pub fn preds(pu: u32, pv: u32, pp: u32, pq: u32) -> u128 {
        (pu as u128) << 81 | (pv as u128) << 84 | (pp as u128) << 87 | (pq as u128) << 77
    }

    pub fn plop3_fields(lut0: u8, lut1: u8, pr: u32) -> u128 {
        (lut0 as u128 >> 3) << 72 | (lut0 as u128 & 7) << 64 | (lut1 as u128) << 16 | (pr as u128) << 68
    }

    pub fn lea_shift(scale: u32) -> u128 {
        (scale as u128) << 75
    }

    pub fn byte_select(byte: u32) -> u128 {
        (byte as u128) << 76
    }
}

fn run_translation_test(name: &str, instructions: &[u128]) -> TestResult {
    let start = Instant::now();
    println!("Running test: {} ({} instructions)", name, instructions.len());
    let result = catch_unwind(AssertUnwindSafe(|| {
        translate_module(instructions, false, None).validate()
    }));

    let duration = start.elapsed();
    match result {
        Ok(()) => TestResult {
            name: name.to_string(),
            passed: true,
            message: "PASS".to_string(),
            duration,
        },
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else {
                "unknown panic".to_string()
            };
            TestResult {
                name: name.to_string(),
                passed: false,
                message: format!("FAIL: {}", msg),
                duration,
            }
        }
    }
}

pub fn run_gpu_tests() -> Vec<String> {
    let mut results = Vec::new();
    let start_time = Instant::now();

    results.push("Starting GPU/SM86 Decoder Tests...".to_string());
    println!("Starting GPU/SM86 Decoder Tests...");

    let tests = vec![
        run_translation_test("IADD Immediate", &[inst::iadd_imm(1, 2, 42)]),
        run_translation_test("IADD Register", &[inst::iadd_reg(1, 2, 3)]),
        run_translation_test("IADD3 Register", &[inst::iadd3_reg(1, 2, 3, 0xFF)]),
        run_translation_test("MOV immediate", &[inst::mov_imm(1, 0xCAFEBABE)]),
        run_translation_test("MOV register", &[inst::mov_reg(1, 2)]),
        run_translation_test("KILL PT", &[inst::kill(7, false)]),
    ];

    let mut passed = 0;
    for t in &tests {
        let icon = if t.passed { "Y" } else { "N" };
        let line = format!(
            "{} {} - {} ({:?})",
            icon, t.name, t.message, t.duration
        );
        println!("{}", line);
        results.push(line);
        if t.passed {
            passed += 1;
        }
    }

    let failed = tests.len() - passed;
    let total_time = start_time.elapsed();
    let summary = format!(
        "Total: {} ({}/{} passed) time {:?}",
        tests.len(),
        passed,
        failed,
        total_time
    );
    println!("{}", summary);
    results.push(summary);

    results
}

#[test]
fn fixtures_match_nvdisasm() {
    let Some(nvdisasm) = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .chain(std::fs::read_dir("/usr/local").into_iter().flatten().flatten().map(|e| e.path().join("bin")))
        .map(|dir| dir.join("nvdisasm"))
        .find(|path| path.is_file())
    else {
        eprintln!("nvdisasm not found, skipping");
        return;
    };
    const W1D_FSETP: [u128; 16] = [
        0x004fc80003900000000000030200720b,
        0x004fc80003901000000000030200720b,
        0x004fc80003902000000000030200720b,
        0x004fc80003903000000000030200720b,
        0x004fc80003904000000000030200720b,
        0x004fc80003905000000000030200720b,
        0x004fc80003906000000000030200720b,
        0x004fc80003907000000000030200720b,
        0x004fc80003908000000000030200720b,
        0x004fc80003909000000000030200720b,
        0x004fc8000390a000000000030200720b,
        0x004fc8000390b000000000030200720b,
        0x004fc8000390c000000000030200720b,
        0x004fc8000390d000000000030200720b,
        0x004fc8000390e000000000030200720b,
        0x004fc8000390f000000000030200720b,
    ];
    const W1D_FSETP_MOD: u128 = 0x004fc80003901200800000030200720b;
    const W1D_FSETP_FTZ: u128 = 0x004fc80003912000000000030200720b;
    const W1D_FSETP_COMBINE: u128 = 0x004fc80001100400000000030200720b;
    const W1D_FSET: u128 = 0x004fc80003802000000000030201720a;
    const W1D_FSEL: u128 = 0x004fc800000003000000000302017208;
    const W1D_SEL: u128 = 0x004fc800000000000000000302017207;
    const W1D_R2P: u128 = 0x004fc800000010000000000502007804;
    const W1D_P2R: u128 = 0x004fc80000002000000000ff03017803;
    const W1D_PLOP3: u128 = 0x004fc80000703922000000000000781c;
    const W1D_IABS: u128 = 0x004fc800000000000000000200017213;
    const W1D_IMNMX_U32: u128 = 0x004fc800000000000000000302017217;
    const W1D_IMNMX_S32: u128 = 0x004fc800000002000000000302017217;
    const W1D_IADD3_CARRY: u128 = 0x004fe2000791e0050000000004077210;
    const W1D_IADD3_OBSERVER: u128 = 0x008fc600007e24ff000000ffff097210;
    const W1D_IADD3_X: u128 = 0x004fc80003fee4090000000700077210;
    let bin = std::env::temp_dir().join(format!("oboromi-fixture-{}.bin", std::process::id()));
    let fixtures: &[(u128, &str)] = &[
        (inst::iadd_imm(1, 2, 42), "IADD3 R1, R2, 0x2a, RZ;"),
        (inst::iadd_reg(1, 2, 3), "IADD3 R1, R2, R3, RZ;"),
        (inst::iadd3_reg(1, 2, 3, 0xFF), "IADD3 R1, R2, R3, RZ;"),
        (inst::mov_imm(1, 0xCAFEBABE), "MOV R1, 0xcafebabe;"),
        (inst::mov_reg(1, 2), "MOV R1, R2;"),
        (inst::mov_cbuf(5, 0, 0x10), "MOV R5, c[0x0][0x10];"),
        (inst::mov_ureg(6, 4), "MOV R6, UR4;"),
        (inst::kill(7, false), "KILL;"),
        (inst::guard(inst::mov_imm(1, 5), 0, false), "@P0 MOV R1, 0x5;"),
        (inst::guard(inst::bra(16), 0, true), "@!P0 BRA 0x20;"),
        (inst::bra(-80), "BRA -0x40;"),
        (inst::exit(), "EXIT;"),
        (inst::bssy(0, 0x40), "BSSY B0, 0x50;"),
        (inst::bsync(0), "BSYNC B0;"),
        (inst::call_rel(0x20), "CALL.REL 0x30;"),
        (inst::ret_rel(20, -0x60), "RET.REL.NODEC R20 -0x50;"),
        (inst::brx(4, -0x10), "BRX R4 -0x10;"),
        (inst::jmx(4, 0), "JMX R4;"),
        (inst::bra_upred(0, true, 0x10), "BRA.U !UP0, 0x20;"),
        (inst::bra_div(0x10), "BRA.DIV ~URZ, 0x20;"),
        (inst::bra_conv(0x10), "BRA.CONV ~URZ, 0x20;"),
        (inst::fadd(1, 2, 3), "FADD R1, R2, R3;"),
        (inst::fadd(1, 2, 3) | inst::FP_ABS_A | inst::FP_NEG_A | inst::FP_NEG_B, "FADD R1, -|R2|, -R3;"),
        (inst::fadd(1, 2, 3) | inst::FP_ABS_B | inst::FP_SAT | inst::FP_FTZ, "FADD.FTZ.SAT R1, R2, |R3|;"),
        (inst::fadd_imm(1, 2, 0.5), "FADD R1, R2, 0.5;"),
        (inst::fmul(1, 2, 3), "FMUL R1, R2, R3;"),
        (inst::fmul(1, 2, 3) | inst::FP_FMZ, "FMUL.FMZ R1, R2, R3;"),
        (inst::fmul(1, 2, 3) | inst::FP_FTZ | inst::FP_SAT, "FMUL.FTZ.SAT R1, R2, R3;"),
        (inst::fmul_imm(1, 2, 3.0), "FMUL R1, R2, 3;"),
        (inst::ffma(1, 2, 3, 4), "FFMA R1, R2, R3, R4;"),
        (inst::ffma(1, 2, 3, 4) | inst::FP_NEG_C | inst::FP_FMZ, "FFMA.FMZ R1, R2, R3, -R4;"),
        (inst::ffma_imm_b(1, 2, 2.0, 4), "FFMA R1, R2, 2, R4;"),
        (inst::ffma_imm_c(1, 2, 3, 0.5), "FFMA R1, R2, R3, 0.5;"),
        (inst::fmnmx(1, 2, 3, 7, false), "FMNMX R1, R2, R3, PT;"),
        (inst::fmnmx(1, 2, 3, 0, true) | inst::FP_NEG_A | inst::FP_ABS_B, "FMNMX R1, -R2, |R3|, !P0;"),
        (inst::mufu(1, inst::MUFU_RCP, 3), "MUFU.RCP R1, R3 ;"),
        (inst::mufu(1, inst::MUFU_TANH, 3), "MUFU.TANH R1, R3 ;"),
        (0x004fc800078e02090000000700077224, "IMAD R7, R0, R7, R9 ;"),
        (0x004fc800078e18ff0000000302017211, "LEA R1, R2, R3, 0x3 ;"),
        (0x004fc80003906270000000030200720c, "ISETP.GE.AND P0, P1, R2, R3, PT ;"),
        (0x004fc800078e00060000000302047225, "IMAD.WIDE.U32 R4, R2, R3, R6 ;"),
        (0x004fc800078e02060000000302017227, "IMAD.HI R1, R2, R3, R6 ;"),
        (0x004fc800000e06060000000302017224, "IMAD.X R1, R2, R3, R6, P0 ;"),
        (0x004fc800000e0e060000000302017224, "IMAD.X R1, R2, R3, ~R6, P0 ;"),
        (0x004fc800078e02060000000502017824, "IMAD R1, R2, 0x5, R6 ;"),
        (0x004fc800078e02060000000002017a24, "IMAD R1, R2, c[0x0][0x0], R6 ;"),
        (0x004fc8000f8e02060000000402017c24, "IMAD R1, R2, UR4, R6 ;"),
        (0x004fc800078e02030000000502017424, "IMAD R1, R2, R3, 0x5 ;"),
        (0x004fc80003902270000000050200780c, "ISETP.EQ.AND P0, P1, R2, 0x5, PT ;"),
        (0x004fc800039022700000000002007a0c, "ISETP.EQ.AND P0, P1, R2, c[0x0][0x0], PT ;"),
        (0x004fc8000b9022700000000402007c0c, "ISETP.EQ.AND P0, P1, R2, UR4, PT ;"),
        (0x004fc80003906320000000030200720c, "ISETP.GE.AND.EX P0, P1, R2, R3, PT, P2 ;"),
        (W1D_FSETP[0], "FSETP.F.AND P0, P1, R2, R3, PT ;"),
        (W1D_FSETP[1], "FSETP.LT.AND P0, P1, R2, R3, PT ;"),
        (W1D_FSETP[2], "FSETP.EQ.AND P0, P1, R2, R3, PT ;"),
        (W1D_FSETP[3], "FSETP.LE.AND P0, P1, R2, R3, PT ;"),
        (W1D_FSETP[4], "FSETP.GT.AND P0, P1, R2, R3, PT ;"),
        (W1D_FSETP[5], "FSETP.NE.AND P0, P1, R2, R3, PT ;"),
        (W1D_FSETP[6], "FSETP.GE.AND P0, P1, R2, R3, PT ;"),
        (W1D_FSETP[7], "FSETP.NUM.AND P0, P1, R2, R3, PT ;"),
        (W1D_FSETP[8], "FSETP.NAN.AND P0, P1, R2, R3, PT ;"),
        (W1D_FSETP[9], "FSETP.LTU.AND P0, P1, R2, R3, PT ;"),
        (W1D_FSETP[10], "FSETP.EQU.AND P0, P1, R2, R3, PT ;"),
        (W1D_FSETP[11], "FSETP.LEU.AND P0, P1, R2, R3, PT ;"),
        (W1D_FSETP[12], "FSETP.GTU.AND P0, P1, R2, R3, PT ;"),
        (W1D_FSETP[13], "FSETP.NEU.AND P0, P1, R2, R3, PT ;"),
        (W1D_FSETP[14], "FSETP.GEU.AND P0, P1, R2, R3, PT ;"),
        (W1D_FSETP[15], "FSETP.T.AND P0, P1, R2, R3, PT ;"),
        (W1D_FSETP_MOD, "FSETP.LT.AND P0, P1, |R2|, -R3, PT ;"),
        (W1D_FSETP_FTZ, "FSETP.EQ.FTZ.AND P0, P1, R2, R3, PT ;"),
        (W1D_FSETP_COMBINE, "FSETP.F.OR P0, P1, R2, R3, P2 ;"),
        (W1D_FSET, "FSET.BF.EQ.AND R1, R2, R3, PT ;"),
        (W1D_FSEL, "FSEL R1, -|R2|, R3, P0 ;"),
        (W1D_SEL, "SEL R1, R2, R3, P0 ;"),
        (W1D_R2P, "R2P PR, R2.B1, 0x5 ;"),
        (W1D_P2R, "P2R.B2 R1, PR, R3, 0xff ;"),
        (W1D_PLOP3, "PLOP3.LUT P0, PT, P0, P1, P2, 0xca, 0x0 ;"),
        (W1D_IABS, "IABS R1, R2 ;"),
        (W1D_IMNMX_U32, "IMNMX.U32 R1, R2, R3, P0 ;"),
        (W1D_IMNMX_S32, "IMNMX R1, R2, R3, P0 ;"),
        (W1D_IADD3_CARRY, "IADD3 R7, P0, P1, R4, R0, R5 ;"),
        (W1D_IADD3_OBSERVER, "IADD3.X R9, RZ, RZ, RZ, P0, P1 ;"),
        (W1D_IADD3_X, "IADD3.X R7, R0, R7, R9, PT, PT ;"),
        (inst::ldg(4, 2, 8), "LDG.E R4, [R2+0x8] ;"),
        (inst::alu3(inst::LOP3 | inst::lop3_fields(0xca, 7, 7, true), 10, 1, 2, 3), "LOP3.LUT R10, R1, R2, R3, 0xca, !PT;"),
        (inst::alu3(inst::LOP3 ^ inst::IMM_B | inst::PAND | inst::lop3_fields(0xc0, 2, 4, true), 255, 1, 0x8000_0000, 255), "LOP3.LUT.PAND P2, RZ, R1, 0x80000000, RZ, 0xc0, !P4;"),
        (inst::alu3(inst::SHF | inst::SHF_U32 | inst::SHF_HI | inst::WRAP, 10, 0, 2, 1), "SHF.L.W.U32.HI R10, R0, R2, R1;"),
        (inst::alu3(inst::SHF ^ inst::IMM_B | inst::SHF_R | inst::SHF_S32 | inst::SHF_HI, 12, 255, 31, 1), "SHF.R.S32.HI R12, RZ, 0x1f, R1;"),
        (inst::alu3(inst::SHF ^ inst::IMM_B | inst::SHF_R | inst::SHF_U64, 14, 0, 64, 1), "SHF.R.U64 R14, R0, 0x40, R1;"),
        (inst::alu3(inst::SHF ^ inst::IMM_B | inst::SHF_R | inst::SHF_HI, 15, 0, 64, 1), "SHF.R.S64.HI R15, R0, 0x40, R1;"),
        (inst::alu3(inst::SHF ^ inst::IMM_C | inst::SHF_R | inst::SHF_U32, 17, 0, 0xabcd, 2), "SHF.R.U32 R17, R0, R2, 0xabcd;"),
        (inst::alu3(inst::PRMT ^ inst::IMM_B, 12, 0, 0x5098, 1), "PRMT R12, R0, 0x5098, R1;"),
        (inst::alu3(inst::PRMT ^ inst::IMM_B | inst::prmt_mode(1), 13, 0, 1, 1), "PRMT.F4E R13, R0, 0x1, R1;"),
        (inst::alu3(inst::PRMT ^ inst::IMM_B | inst::prmt_mode(2), 14, 0, 0, 1), "PRMT.B4E R14, R0, 0x0, R1;"),
        (inst::alu3(inst::PRMT | inst::prmt_mode(3), 19, 0, 2, 1), "PRMT.RC8 R19, R0, R2, R1;"),
        (inst::alu3(inst::PRMT ^ inst::IMM_B | inst::prmt_mode(4), 16, 0, 1, 1), "PRMT.ECL R16, R0, 0x1, R1;"),
        (inst::alu3(inst::PRMT ^ inst::IMM_B | inst::prmt_mode(5), 17, 0, 2, 1), "PRMT.ECR R17, R0, 0x2, R1;"),
        (inst::alu3(inst::PRMT ^ inst::IMM_B | inst::prmt_mode(6), 18, 0, 1, 1), "PRMT.RC16 R18, R0, 0x1, R1;"),
        (inst::alu(inst::POPC | inst::INVERT_B, 11, 0, 0), "POPC R11, ~R0 ;"),
        (inst::alu(inst::POPC ^ inst::IMM_B, 14, 0, 0x8000_000f), "POPC R14, 0x8000000f ;"),
        (inst::alu(inst::FLO | inst::FLO_SH, 13, 0, 1), "FLO.U32.SH R13, R1 ;"),
        (inst::alu(inst::FLO | inst::S32 | inst::FLO_SH, 18, 0, 4), "FLO.SH R18, R4 ;"),
        (inst::alu(inst::FLO | inst::INVERT_B, 17, 0, 3), "FLO.U32 R17, ~R3 ;"),
        (inst::alu(inst::BREV ^ inst::IMM_B, 12, 0, 3), "BREV R12, 0x3 ;"),
        (inst::alu(inst::BMSK | inst::WRAP, 17, 7, 1), "BMSK.W R17, R7, R1;"),
        (inst::alu(inst::BMSK ^ inst::IMM_B, 18, 2, 4), "BMSK R18, R2, 0x4;"),
        (inst::alu(inst::SGXT | inst::S32, 16, 0, 4), "SGXT R16, R0, R4;"),
        (inst::alu(inst::SGXT ^ inst::IMM_B | inst::S32 | inst::WRAP, 15, 3, 33), "SGXT.W R15, R3, 0x21;"),
        (inst::alu(inst::SGXT ^ inst::IMM_B, 11, 1, 8), "SGXT.U32 R11, R1, 0x8;"),
        (inst::alu3(inst::ULOP3 | inst::lop3_fields(0xc0, 7, 7, true), 10, 1, 3, 63), "ULOP3.LUT UR10, UR1, UR3, URZ, 0xc0, !UPT;"),
        (inst::alu3(inst::ULOP3 ^ inst::IMM_B | inst::lop3_fields(0xc0, 1, 7, true), 63, 2, 1, 63), "ULOP3.LUT UP1, URZ, UR2, 0x1, URZ, 0xc0, !UPT;"),
        (inst::alu3(inst::USHF ^ inst::IMM_B | inst::SHF_R | inst::SHF_U32 | inst::SHF_HI, 11, 63, 31, 2), "USHF.R.U32.HI UR11, URZ, 0x1f, UR2;"),
        (inst::alu3(inst::UPRMT ^ inst::IMM_B, 12, 1, 0x0123, 63), "UPRMT UR12, UR1, 0x123, URZ;"),
        (inst::alu(inst::UPOPC, 13, 0, 2), "UPOPC UR13, UR2;"),
        (inst::alu(inst::UFLO, 14, 0, 2), "UFLO.U32 UR14, UR2;"),
        (inst::alu(inst::UBREV, 15, 0, 1), "UBREV UR15, UR1;"),
        (inst::alu(inst::UBMSK ^ inst::IMM_B, 16, 4, 8), "UBMSK UR16, UR4, 0x8;"),
        (inst::alu(inst::USGXT ^ inst::IMM_B | inst::S32, 17, 3, 16), "USGXT UR17, UR3, 0x10;"),
        (inst::guard(inst::alu3(inst::UIADD3 ^ inst::IMM_B | inst::preds(7, 7, 7, 7), 6, 63, 1, 63), 0, true), "@!UP0 UIADD3 UR6, URZ, 0x1, URZ;"),
        (inst::alu3(inst::UIADD3 | inst::preds(0, 1, 7, 7), 12, 0, 7, 9), "UIADD3 UR12, UP0, UP1, UR0, UR7, UR9;"),
        (inst::alu3(inst::UIADD3 | inst::NEG_A | inst::preds(7, 7, 7, 7), 10, 0, 7, 9), "UIADD3 UR10, -UR0, UR7, UR9;"),
        (inst::alu3(inst::UIADD3 | inst::EXTENDED | inst::preds(7, 7, 7, 7), 11, 0, 7, 9), "UIADD3.X UR11, UR0, UR7, UR9, UPT, UPT;"),
        (inst::alu3(inst::UIMAD | inst::S32, 7, 0, 7, 9), "UIMAD UR7, UR0, UR7, UR9;"),
        (inst::alu(inst::UMOV_IMM, 1, 0, 0xdead_beef), "UMOV UR1, 0xdeadbeef;"),
        (inst::alu(inst::UMOV, 2, 0, 1), "UMOV UR2, UR1;"),
        (inst::alu(inst::R2UR, 3, 5, 0), "R2UR UR3, R5 ;"),
        (inst::alu(inst::USEL ^ inst::IMM_B | inst::preds(0, 0, 1, 0), 4, 3, 7), "USEL UR4, UR3, 0x7, UP1;"),
        (inst::alu3(inst::ULEA | inst::lea_shift(3) | inst::preds(7, 0, 0, 0), 6, 3, 1, 63), "ULEA UR6, UR3, UR1, 0x3;"),
        (inst::alu3(inst::UIMAD_WIDE ^ inst::IMM_B | inst::preds(7, 0, 0, 0), 8, 3, 0x10, 10), "UIMAD.WIDE.U32 UR8, UR3, 0x10, UR10;"),
        (inst::alu3(inst::UIADD3_64 ^ inst::IMM_B | inst::preds(7, 7, 7, 7), 12, 10, u32::MAX, 63), "UIADD3.64 UR12, UR10, -0x1, URZ;"),
        (inst::alu(inst::UISETP | inst::ICMP_LT | inst::preds(0, 1, 7, 0), 0, 5, 6), "UISETP.LT.U32.AND UP0, UP1, UR5, UR6, UPT;"),
        (inst::alu(inst::UISETP | inst::ICMP_LT | inst::S32 | inst::preds(2, 3, 7, 0), 0, 6, 5), "UISETP.LT.AND UP2, UP3, UR6, UR5, UPT;"),
        (inst::alu(inst::UPLOP3 | inst::preds(4, 5, 0, 1) | inst::plop3_fields(0xe8, 0x96, 2), 0, 0, 0), "UPLOP3.LUT UP4, UP5, UP0, UP1, UP2, 0xe8, 0x96;"),
        (inst::alu(inst::UP2UR_IMM | inst::byte_select(1), 7, 8, 0xff), "UP2UR.B1 UR7, UPR, UR8, 0xff;"),
        (inst::alu(inst::UR2UP_IMM | inst::byte_select(2), 0, 9, 0xf), "UR2UP UPR, UR9.B2, 0xf;"),
        (inst::stg(2, 0x10, 5), "STG.E [R2+0x10], R5 ;"),
        (inst::ldc(6, 0, 3, 4), "LDC R6, c[0x0][R3+0x4] ;"),
        (inst::uldc64(4, 0, 0x10), "ULDC.64 UR4, c[0x0][0x10];"),
        (inst::s2r(1, 33), "S2R R1, SR_TID.X ;"),
        (inst::cs2r(4, 80, false), "CS2R.32 R4, SR_CLOCKLO;"),
        (inst::cs2r(4, 80, true), "CS2R R4, SR_CLOCKLO;"),
        (inst::s2ur(5, 37), "S2UR UR5, SR_CTAID.X ;"),
        (inst::f2f(0, 2, inst::cvt::F64, inst::cvt::F32, inst::cvt::RN, false), "F2F.F64.F32 R0, R2 ;"),
        (inst::f2f(13, 14, inst::cvt::F32, inst::cvt::F64, inst::cvt::RP, false), "F2F.F32.F64.RP R13, R14 ;"),
        (inst::f2i(1, 3, inst::cvt::S32, inst::cvt::F32, inst::cvt::RN), "F2I R1, R3 ;"),
        (inst::f2i(10, 12, inst::cvt::S64, inst::cvt::F64, inst::cvt::RN), "F2I.S64.F64 R10, R12 ;"),
        (inst::frnd(4, 5, inst::cvt::RN), "FRND R4, R5 ;"),
        (inst::i2f(6, 7, inst::cvt::F32, inst::cvt::S32), "I2F R6, R7 ;"),
        (inst::i2f(11, 12, inst::cvt::F32, inst::cvt::U64), "I2F.U64 R11, R12 ;"),
        (inst::i2i(8, 9, inst::cvt::U8), "I2I.U8.S32.SAT R8, R9;"),
    ];
    for &(word, expected) in fixtures {
        std::fs::write(&bin, word.to_le_bytes()).unwrap();
        let out = std::process::Command::new(&nvdisasm).args(["--binary", "SM86"]).arg(&bin).output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        let disasm = text.lines().find_map(|line| line.split("/*0000*/").nth(1)).unwrap_or_default();
        assert_eq!(disasm.trim(), expected, "{word:#x}");
    }
    std::fs::remove_file(bin).unwrap();
}

#[test]
fn mov_immediate_passes_spirv_val() {
    use crate::gpu::spirv::capability::SHADER;
    use crate::tests::gpu::harness::spirv_val;
    let module = translate_module(&[inst::mov_imm(1, 0xCAFEBABE)], false, None);
    assert_eq!(module.words()[5..8], [2 << 16 | 17, SHADER, 3 << 16 | 14]);
    spirv_val(&module);
}
