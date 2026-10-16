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
    const PT_GUARD: u128 = 7 << 12;
    const RC_RZ: u128 = 0xff << 64;
    const PIXMASK_ALL: u128 = 0xf << 72;
    const IADD3_NO_CARRY: u128 = 0x3fff << 77;
    const KILL_PP_PT: u128 = 7 << 87;
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
    pub const FP_ABS_C: u128 = 1 << 74;
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

    // ldg.e rd, [ra + off]
    pub fn ldg(rd: u32, ra: u32, off: u32) -> u128 {
        let mut inst = LDG | PT_GUARD | ADDR_64 | SZ_32 | LDG_PU_PT | COP_EN;
        inst |= ((rd as u128) & 0xff) << 16;
        inst |= ((ra as u128) & 0xff) << 24;
        inst |= ((off as u128) & 0xff_ffff) << 40;
        inst
    }

    // stg.e [ra + off], rb
    pub fn stg(ra: u32, off: u32, rb: u32) -> u128 {
        let mut inst = STG | PT_GUARD | ADDR_64 | SZ_32 | COP_EN | NO_WRITE_SCOREBOARD;
        inst |= ((ra as u128) & 0xff) << 24;
        inst |= ((rb as u128) & 0xff) << 32;
        inst |= ((off as u128) & 0xff_ffff) << 40;
        inst
    }

    // ldc rd, c[bank][ra + off]
    pub fn ldc(rd: u32, bank: u32, ra: u32, off: u32) -> u128 {
        let mut inst = LDC | PT_GUARD | SZ_32;
        inst |= ((rd as u128) & 0xff) << 16;
        inst |= ((ra as u128) & 0xff) << 24;
        inst |= ((off as u128) & 0xffff) << 38;
        inst |= ((bank as u128) & 0x1f) << 54;
        inst
    }

    // uldc.64 urd, c[bank][off]
    pub fn uldc64(urd: u32, bank: u32, off: u32) -> u128 {
        let mut inst = ULDC | PT_GUARD | SZ_64;
        inst |= ((urd as u128) & 0x3f) << 16;
        inst |= ((off as u128) & 0xffff) << 38;
        inst |= ((bank as u128) & 0x1f) << 54;
        inst
    }

    pub fn kill(pred: u32, invert: bool) -> u128 {
        let mut inst = KILL | KILL_PP_PT | NO_SCOREBOARD;
        inst |= ((pred as u128) & 0x7) << 12;
        if invert {
            inst |= 1 << 15;
        }
        inst
    }

    // bra (opcode=0x947), offset in bytes from the next instruction
    pub fn bra(offset: i64) -> u128 {
        0x7947 | 0x7 << 87 | 0x3f << 110 | ((offset as u128 >> 2) & 0xffff_ffff_ffff) << 34
    }

    // exit (opcode=0x94d)
    pub fn exit() -> u128 {
        0x794d | 0x7 << 87 | 0x3f << 110
    }

    fn alu(op: u128, rd: u32, ra: u32, b: u32) -> u128 {
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

    pub fn guard(inst: u128, pred: u32, invert: bool) -> u128 {
        inst & !(0xf << 12) | (pred as u128 & 0x7) << 12 | (invert as u128) << 15
    }
}

fn run_translation_test(name: &str, instructions: &[u128]) -> TestResult {
    let start = Instant::now();
    println!("Running test: {} ({} instructions)", name, instructions.len());
    let result = catch_unwind(AssertUnwindSafe(|| {
        translate_module(instructions, false).validate()
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
        (inst::ldg(4, 2, 8), "LDG.E R4, [R2+0x8] ;"),
        (inst::stg(2, 0x10, 5), "STG.E [R2+0x10], R5 ;"),
        (inst::ldc(6, 0, 3, 4), "LDC R6, c[0x0][R3+0x4] ;"),
        (inst::uldc64(4, 0, 0x10), "ULDC.64 UR4, c[0x0][0x10];"),
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
    let module = translate_module(&[inst::mov_imm(1, 0xCAFEBABE)], false);
    assert_eq!(module.words()[5..8], [2 << 16 | 17, SHADER, 3 << 16 | 14]);
    spirv_val(&module);
}
