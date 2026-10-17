#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::run_regs;

    const PT: u128 = 7 << 12;
    const DADD_R: u128 = 0x229 | PT;
    const DADD_I: u128 = 0x429 | PT;
    const DADD_C: u128 = 0x629 | PT;
    const DMUL_R: u128 = 0x228 | PT;
    const DMUL_I: u128 = 0x828 | PT;
    const DFMA_R: u128 = 0x22b | PT;
    const DFMA_I_B: u128 = 0x82b | PT;
    const DFMA_I_C: u128 = 0x42b | PT;
    const DFMA_C_B: u128 = 0xa2b | PT;
    const DFMA_C_C: u128 = 0x62b | PT;
    const DSETP_R: u128 = 0x22a | PT;
    const DSETP_I: u128 = 0x42a | PT;
    const DSETP_C: u128 = 0x62a | PT;

    const ABS_A: u128 = 1 << 73;
    const NEG_A: u128 = 1 << 72;
    const ABS_B: u128 = 1 << 62;
    const ABS_C: u128 = 1 << 74;
    const NEG_C: u128 = 1 << 75;

    const fn dadd_r(rd: u32, ra: u32, rc: u32) -> u128 {
        DADD_R | (rd as u128) << 16 | (ra as u128) << 24 | (rc as u128) << 64
    }

    const fn binary(base: u128, rd: u32, ra: u32, src: u32) -> u128 {
        base | (rd as u128) << 16 | (ra as u128) << 24 | (src as u128) << 32
    }

    const fn ternary(base: u128, rd: u32, ra: u32, src: u32, rc: u32) -> u128 {
        binary(base, rd, ra, src) | (rc as u128) << 64
    }

    const fn dsetp(
        base: u128,
        pu: u32,
        pv: u32,
        ra: u32,
        src: u32,
        test: u32,
        bop: u32,
        pp: u32,
    ) -> u128 {
        base | (ra as u128) << 24
            | (src as u128) << 32
            | (bop as u128) << 74
            | (test as u128) << 76
            | (pu as u128) << 81
            | (pv as u128) << 84
            | (pp as u128) << 87
    }

    const WORD_DADD_REG: u128 = dadd_r(32, 0, 2);
    const WORD_DADD_IMM: u128 = binary(DADD_I, 34, 0, 0x4000_0000);
    const WORD_DADD_CBUF: u128 = binary(DADD_C, 36, 0, 0);
    const WORD_DADD_NEG_ZERO: u128 = dadd_r(38, 6, 6);
    const WORD_DADD_INF: u128 = dadd_r(40, 8, 10);
    const WORD_DADD_NAN: u128 = dadd_r(42, 12, 0);
    const WORD_DADD_MOD: u128 = dadd_r(58, 0, 2) | ABS_A | NEG_A | ABS_C | NEG_C;
    const WORD_DMUL_MOD: u128 = binary(DMUL_R, 44, 0, 2) | NEG_A | ABS_B;
    const WORD_DMUL_IMM: u128 = binary(DMUL_I, 46, 0, 0x4000_0000);
    const WORD_DFMA_MOD: u128 = ternary(DFMA_R, 48, 0, 2, 4) | ABS_A | NEG_A | ABS_B | NEG_C;
    const WORD_DFMA_IMM_B: u128 = ternary(DFMA_I_B, 50, 0, 0x4000_0000, 4);
    const WORD_DFMA_IMM_C: u128 = ternary(DFMA_I_C, 52, 0, 0x4000_0000, 2);
    const WORD_DFMA_CBUF_B: u128 = ternary(DFMA_C_B, 54, 0, 0, 4);
    const WORD_DFMA_CBUF_C: u128 = ternary(DFMA_C_C, 56, 0, 0, 2);
    const WORD_DFMA_FUSED: u128 = ternary(DFMA_R, 60, 0, 2, 4);
    const WORD_DSETP_LT: u128 = dsetp(DSETP_R, 0, 1, 0, 2, 1, 0, 7);
    const WORD_DSETP_EQU: u128 = dsetp(DSETP_I, 3, 4, 4, 0x4000_0000, 10, 0, 7);
    const WORD_DSETP_NUM: u128 = dsetp(DSETP_C, 5, 6, 4, 0, 7, 0, 7);
    const WORD_DSETP_OR: u128 = dsetp(DSETP_R, 0, 1, 0, 2, 4, 1, 2);
    const WORD_DSETP_XOR: u128 = dsetp(DSETP_R, 5, 6, 0, 0, 2, 2, 2);
    const WORD_DSETP_LTU: u128 = dsetp(DSETP_R, 0, 7, 4, 0, 9, 0, 7);
    const WORD_DSETP_EQU_REG: u128 = dsetp(DSETP_R, 1, 7, 4, 0, 10, 0, 7);
    const WORD_DSETP_LEU: u128 = dsetp(DSETP_R, 2, 7, 4, 0, 11, 0, 7);
    const WORD_DSETP_GTU: u128 = dsetp(DSETP_R, 3, 7, 4, 0, 12, 0, 7);
    const WORD_DSETP_NEU: u128 = dsetp(DSETP_R, 4, 7, 4, 0, 13, 0, 7);
    const WORD_DSETP_GEU: u128 = dsetp(DSETP_R, 5, 7, 4, 0, 14, 0, 7);

    fn words(values: &[f64]) -> Vec<u32> {
        values
            .iter()
            .flat_map(|value| {
                let bits = value.to_bits();
                [bits as u32, (bits >> 32) as u32]
            })
            .collect()
    }

    fn assert_f64(regs: &[u32], reg: usize, expected: f64) {
        let actual = regs[reg] as u64 | (regs[reg + 1] as u64) << 32;
        assert_eq!(
            actual,
            expected.to_bits(),
            "R{reg}: got {}",
            f64::from_bits(actual)
        );
    }

    fn using_lavapipe() -> bool {
        ["VK_DRIVER_FILES", "VK_ICD_FILENAMES"]
            .iter()
            .filter_map(|name| std::env::var(name).ok())
            .any(|path| path.contains("lvp_icd"))
    }

    #[test]
    fn arithmetic_handles_sources_modifiers_and_ieee_edges() {
        let nan = f64::from_bits(0x7ff8_0000_0000_1234);
        let values = [1.5, -2.25, 0.5, -0.0, f64::INFINITY, f64::NEG_INFINITY, nan];
        let program = [
            WORD_DADD_REG,
            WORD_DADD_IMM,
            WORD_DADD_CBUF,
            WORD_DADD_NEG_ZERO,
            WORD_DADD_INF,
            WORD_DADD_NAN,
            WORD_DMUL_MOD,
            WORD_DMUL_IMM,
            WORD_DFMA_MOD,
            WORD_DFMA_IMM_B,
            WORD_DFMA_IMM_C,
            WORD_DFMA_CBUF_B,
            WORD_DFMA_CBUF_C,
            WORD_DADD_MOD,
        ];
        let Some((regs, _)) = run_regs(&program, &words(&values), &words(&[4.0]), &[]) else {
            return;
        };
        let [a, b, c] = [values[0], values[1], values[2]];
        for (reg, expected) in [
            (32, a + b),
            (34, a + 2.0),
            (36, a + 4.0),
            (38, -0.0 + -0.0),
            (42, nan + a),
            (44, (-a.abs()) * b.abs()),
            (46, a * 2.0),
            (48, (-a.abs()).mul_add(b.abs(), -c)),
            (50, a.mul_add(2.0, c)),
            (52, a.mul_add(b, 2.0)),
            (54, a.mul_add(4.0, c)),
            (56, a.mul_add(b, 4.0)),
            (58, -a.abs() - b.abs()),
        ] {
            assert_f64(&regs, reg, expected);
        }
        let invalid = regs[40] as u64 | (regs[41] as u64) << 32;
        assert!(f64::from_bits(invalid).is_nan());
    }

    #[test]
    fn dfma_rounds_once() {
        if using_lavapipe() {
            return;
        }
        let a = 1.0 + f64::EPSILON;
        let b = 1.0 + f64::EPSILON;
        let c = -(1.0 + 2.0 * f64::EPSILON);
        let Some((regs, _)) = run_regs(&[WORD_DFMA_FUSED], &words(&[a, b, c]), &[], &[]) else {
            return;
        };
        assert_ne!(a.mul_add(b, c), a * b + c);
        assert_f64(&regs, 60, a.mul_add(b, c));
    }

    #[test]
    fn dsetp_handles_unordered_comparisons_and_predicate_combine() {
        let mut seed = words(&[1.0, 2.0, f64::NAN]);
        seed.resize(255, 0);
        seed[254] = 1 << 2;
        let unordered = [
            WORD_DSETP_LTU,
            WORD_DSETP_EQU_REG,
            WORD_DSETP_LEU,
            WORD_DSETP_GTU,
            WORD_DSETP_NEU,
            WORD_DSETP_GEU,
        ];
        let Some((regs, _)) = run_regs(&unordered, &seed, &[], &[]) else {
            return;
        };
        assert_eq!(regs[254] & 0x3f, 0x3f);
        let program = [
            WORD_DSETP_LT,
            WORD_DSETP_EQU,
            WORD_DSETP_NUM,
            WORD_DSETP_OR,
            WORD_DSETP_XOR,
        ];
        let Some((regs, _)) = run_regs(&program, &seed, &words(&[2.0]), &[]) else {
            return;
        };
        assert_eq!(regs[254] & 0x7f, 0b0100_1111);
    }

}
