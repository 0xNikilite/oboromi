#[cfg(test)]
pub mod tests {
    use crate::tests::gpu::harness::{UREG, run_regs};
    use crate::tests::gpu_test::inst::{self, *};

    // Exact for the zeros, infinities, NaN and normal halves used here.
    pub fn h(x: f32) -> u32 {
        let bits = x.to_bits();
        let mantissa = bits >> 13 & 0x3ff;
        let magnitude = match bits >> 23 & 0xff {
            0 => 0,
            0xff => 0x7c00 | mantissa,
            exp => (exp - 112) << 10 | mantissa,
        };
        bits >> 16 & 0x8000 | magnitude
    }

    fn h2(hi: f32, lo: f32) -> u32 {
        h(hi) << 16 | h(lo)
    }

    fn assert_halves(got: &[u32], want: &[u32]) {
        let nan = |half: u32| half & 0x7c00 == 0x7c00 && half & 0x3ff != 0;
        let same = |got: u32, want: u32| got == want || nan(got) && nan(want);
        for (i, (&got, &want)) in got.iter().zip(want).enumerate() {
            let ok = same(got & 0xffff, want & 0xffff) && same(got >> 16, want >> 16);
            assert!(ok, "output {i}: got {got:#010x}, want {want:#010x}");
        }
    }

    fn run(program: &[(u128, &str)], seed: &[u32]) -> Option<Vec<u32>> {
        if !crate::gpu::device().is_ok_and(|d| d.shader_float16) {
            eprintln!("skipping: no shaderFloat16");
            return None;
        }
        let words: Vec<u128> = program.iter().map(|&(word, _)| word).collect();
        run_regs(&words, seed, &[], &[]).map(|(regs, _)| regs)
    }

    pub fn arith() -> [(u128, &'static str); 17] {
        [
            (inst::alu(HADD2, 10, 0, 1), "HADD2 R10, R0, R1;"),
            (inst::alu(HADD2, 11, 0, 1) | iswz(H0_H0, H1_H1, 0) | FP_NEG_B, "HADD2 R11, R0.H0_H0, -R1.H1_H1;"),
            (inst::alu(HADD2, 12, 2, 3), "HADD2 R12, R2, R3;"),
            (inst::alu(HADD2, 13, 2, 2) | iswz(H0_H0, H0_H0, 0), "HADD2 R13, R2.H0_H0, R2.H0_H0;"),
            (inst::alu(HADD2, 14, 255, 0) | FP_NEG_A | iswz(0, H1_H1, 0) | H_F32, "HADD2.F32 R14, -RZ, R0.H1_H1;"),
            (inst::alu(HADD2, 15, 4, 0) | iswz(F32_SWIZZLE, 0, 0) | FP_ABS_B, "HADD2 R15, R4.F32, |R0|;"),
            (inst::alu(HADD2 ^ IMM_C, 16, 0, h2(0.5, -1.0)), "HADD2 R16, R0, 0.5, -1;"),
            (inst::alu(HMUL2, 17, 0, 5) | FP_NEG_A, "HMUL2 R17, -R0, R5;"),
            (inst::alu(HMUL2, 18, 2, 2), "HMUL2 R18, R2, R2;"),
            (inst::alu(HMUL2, 19, 2, 2) | iswz(0, H0_H0, 0) | FP_FMZ, "HMUL2.FMZ R19, R2, R2.H0_H0;"),
            (inst::alu(HMUL2, 25, 7, 7) | FP_FTZ, "HMUL2.FTZ R25, R7, R7;"),
            (inst::alu3(HFMA2, 20, 0, 1, 5), "HFMA2 R20, R0, R1, R5;"),
            (inst::alu3(HFMA2, 21, 0, 5, 1) | FP_SAT, "HFMA2.SAT R21, R0, R5, R1;"),
            (inst::alu3(HFMA2, 22, 0, 1, 5) | H_RELU | H_NEG_C, "HFMA2.RELU R22, R0, R1, -R5;"),
            (inst::alu3(HFMA2 ^ IMM_B, 23, 0, h2(2.0, 0.5), 5), "HFMA2 R23, R0, 2, 0.5, R5;"),
            (inst::alu3(HFMA2 ^ IMM_C, 26, 0, h2(-1.0, 4.0), 1) | H_ABS_C, "HFMA2 R26, R0, |R1|, -1, 4;"),
            (inst::alu3(HFMA2_MMA, 24, 0, 1, 5), "HFMA2.MMA R24, R0, R1, R5;"),
        ]
    }

    pub fn compare() -> [(u128, &'static str); 10] {
        [
            (inst::alu(HSET2, 10, 0, 5) | H_BF | fcmp(1, 0, 0), "HSET2.BF.LT.AND R10, R0, R5, PT;"),
            (inst::alu(HSET2, 11, 0, 5) | fcmp(6, 0, 0), "HSET2.GE.AND R11, R0, R5, PT;"),
            (inst::alu(HSET2, 12, 3, 3) | H_BF | fcmp(13, 0, 0), "HSET2.BF.NEU.AND R12, R3, R3, PT;"),
            (inst::alu(HSETP2, 0, 0, 5) | fcmp(1, 0, 1), "HSETP2.LT.AND P0, P1, R0, R5, PT;"),
            (inst::alu(HSETP2, 0, 0, 5) | fcmp(1, 2, 3) | H_AND, "HSETP2.LT.H_AND.AND P2, P3, R0, R5, PT;"),
            (inst::alu(HSETP2, 0, 1, 5) | fcmp(4, 4, 5) | H_AND | iswz(H0_H0, 0, 0), "HSETP2.GT.H_AND.AND P4, P5, R1.H0_H0, R5, PT;"),
            (inst::alu(HMNMX2, 13, 0, 5) | pp(7, false), "HMNMX2 R13, R0, R5, PT;"),
            (inst::alu(HMNMX2, 14, 0, 5) | pp(7, true), "HMNMX2 R14, R0, R5, !PT;"),
            (inst::alu(HMNMX2, 15, 3, 0) | pp(7, false), "HMNMX2 R15, R3, R0, PT;"),
            (inst::alu(HMNMX2, 16, 3, 0) | pp(7, false) | H_NAN, "HMNMX2.NAN R16, R3, R0, PT;"),
        ]
    }

    pub fn convert() -> [(u128, &'static str); 11] {
        [
            (inst::alu3(F2FP, 20, 6, 7, 255), "F2FP.PACK_AB R20, R6, R7;"),
            (inst::alu3(F2FP, 21, 8, 9, 255), "F2FP.PACK_AB R21, R8, R9;"),
            (inst::alu3(F2FP, 22, 255, 7, 0) | MERGE_C | MERGE_H1, "F2FP.MERGE_C R22, R7, R0.H1;"),
            (inst::alu(I2FP_F32_U32, 23, 0, 11) | PACK_S32, "I2FP.F32.S32 R23, R11;"),
            (inst::alu(I2FP_F32_U32, 24, 0, 14), "I2FP.F32.U32 R24, R14;"),
            (inst::alu3(F2IP, 25, 6, 15, 10), "F2IP.U8.F32 R25, R6, R15, R10;"),
            (inst::alu3(F2IP, 26, 16, 6, 10) | pack_fmt(1, 1) | F2IP_TRUNC, "F2IP.S8.F32.TRUNC R26, R16, R6, R10.H1;"),
            (inst::alu3(I2IP, 27, 11, 12, 10) | pack_fmt(1, 0), "I2IP.S8.S32.SAT R27, R11, R12, R10;"),
            (inst::alu3(I2IP, 28, 13, 14, 255) | pack_fmt(2, 0), "I2IP.U16.S32.SAT R28, R13, R14, RZ;"),
            (inst::alu3(I2IP, 29, 11, 12, 10) | pack_fmt(7, 2) | I2IP_NOSAT, "I2IP.S4.S32 R29, R11, R12, R10.24;"),
            (inst::alu3(I2IP, 30, 11, 12, 10) | pack_fmt(7, 2) | I2IP_SATRELU, "I2IP.S4.S32.SATRELU R30, R11, R12, R10.24;"),
        ]
    }

    #[test]
    fn half2_arithmetic_rounds_each_lane() {
        let tiny = h2(2f32.powi(-7), 2f32.powi(-8));
        let seed = [
            h2(-2.0, 1.5), h2(0.25, 64.0), h2(f32::INFINITY, -0.0), h2(f32::NAN, 3.0),
            2.5f32.to_bits(), h2(0.75, 0.5), 0, tiny,
        ];
        let Some(regs) = run(&arith(), &seed) else {
            return;
        };
        let want = [
            h2(-1.75, 65.5),
            h2(1.25, 1.25),
            h2(f32::NAN, 3.0),
            h2(-0.0, -0.0),
            (-2.0f32).to_bits(),
            h2(4.5, 4.0),
            h2(-1.5, 0.5),
            h2(1.5, -0.75),
            h2(f32::INFINITY, 0.0),
            h2(0.0, 0.0),
            h2(0.25, 96.5),
            h2(0.0, 1.0),
            h2(0.0, 95.5),
            h2(-3.25, 1.25),
            h2(0.25, 96.5),
            h2(2f32.powi(-14), 0.0),
            h2(-1.5, 100.0),
        ];
        assert_halves(&regs[10..27], &want);
    }

    // -74 * 3.9375 is the midpoint -291.375 between two halves, so the subnormal addend decides the rounding and an f32 fma drops it.
    #[test]
    fn hfma2_rounds_the_exact_result_once() {
        let Some(regs) = run(&[(inst::alu3(HFMA2, 3, 0, 1, 2), "")], &[h(-74.0), h(3.9375), 0x20]) else {
            return;
        };
        assert_eq!(regs[3], h(-291.25));
    }

    #[test]
    fn half2_compares_test_each_lane() {
        let seed = [h2(-2.0, 1.5), h2(0.25, 64.0), 0, h2(f32::NAN, 3.0), 0, h2(0.75, 0.5)];
        let Some(regs) = run(&compare(), &seed) else {
            return;
        };
        assert_eq!(regs[10..13], [h2(1.0, 0.0), 0xffff, h2(1.0, 0.0)]);
        assert_eq!(regs[254] & 0x3f, 0b01_1010);
        assert_halves(&regs[13..17], &[h2(-2.0, 0.5), h2(0.75, 1.5), h2(-2.0, 1.5), h2(f32::NAN, 1.5)]);
    }

    #[test]
    fn packed_conversions_round_saturate_and_pack() {
        let mut seed = vec![0; 17];
        seed[0] = h2(-2.0, 1.5);
        seed[6..17].copy_from_slice(&[
            (-2.0f32).to_bits(),
            (1.0 + 2f32.powi(-11) + 2f32.powi(-20)).to_bits(),
            65520f32.to_bits(),
            f32::NAN.to_bits(),
            0x1234_abcd,
            -200i32 as u32,
            5,
            70000,
            u32::MAX,
            300f32.to_bits(),
            2.5f32.to_bits(),
        ]);
        let Some(regs) = run(&convert(), &seed) else {
            return;
        };
        let one_ulp_up = 0x3c01;
        assert_eq!(regs[20], h(-2.0) << 16 | one_ulp_up);
        assert_halves(&regs[21..22], &[h2(f32::INFINITY, f32::NAN)]);
        let want = [
            h(-2.0) << 16 | one_ulp_up,
            (-200f32).to_bits(),
            4294967296f32.to_bits(),
            0xabcd_00ff,
            0x1234_02fe,
            0xabcd_8005,
            0xffff_0000,
            0x34ab_cd85,
            0x34ab_cd05,
        ];
        assert_eq!(regs[22..31], want);
    }

    #[test]
    fn packed_conversions_cover_rz_clamps_and_uniform_data() {
        let mut seed = vec![0; UREG + 11];
        seed[6] = (-2.0f32).to_bits();
        seed[7] = (1.0f32 + 2f32.powi(-11) + 2f32.powi(-20)).to_bits();
        seed[11] = 16_777_219;
        seed[UREG + 6] = 2.0f32.to_bits();
        seed[UREG + 7] = 3.0f32.to_bits();
        let program = [W2J_F2FP_RZ_RELU_SATFINITE, W2J_I2FP_RZ, W2J_UF2FP];
        let Some(regs) = run(&program.map(|word| (word, "")), &seed) else {
            return;
        };
        assert_eq!([regs[20], regs[23], regs[UREG + 10]], [h(1.0), 16_777_218f32.to_bits(), h2(2.0, 3.0)]);
    }
}
