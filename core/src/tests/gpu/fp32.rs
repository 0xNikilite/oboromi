#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::run_regs;
    use crate::tests::gpu_test::inst::{self, *};

    fn bits(values: &[f32]) -> Vec<u32> {
        values.iter().map(|v| v.to_bits()).collect()
    }

    fn assert_f32(got: &[u32], want: &[f32]) {
        for (i, (&got, want)) in got.iter().zip(want).enumerate() {
            let ok = got == want.to_bits() || want.is_nan() && f32::from_bits(got).is_nan();
            assert!(ok, "output {i}: got {got:#x} ({}), want {want}", f32::from_bits(got));
        }
    }

    #[test]
    fn fadd_fmul_ffma_round_to_nearest_like_ieee() {
        let x = [1.5, -2.25, 0.1, 3.0e38, -0.0, f32::INFINITY, f32::NAN, 7.0, f32::NEG_INFINITY];
        let program = [
            inst::fadd(20, 0, 1),
            inst::fadd_imm(21, 2, 0.2),
            inst::fadd(22, 3, 3),
            inst::fadd(23, 4, 4),
            inst::fadd(24, 5, 8),
            inst::fadd(25, 1, 0) | FP_ABS_A | FP_NEG_B,
            inst::fadd(26, 6, 0),
            inst::fmul(27, 0, 1),
            inst::fmul_imm(28, 2, 3.0),
            inst::fmul(29, 4, 0),
            inst::fmul(30, 5, 4),
            inst::ffma(31, 5, 4, 0),
            inst::ffma_imm_b(32, 0, 2.0, 1),
            inst::ffma_imm_c(33, 0, 1, 0.5),
            inst::ffma(34, 2, 2, 7) | FP_NEG_A | FP_NEG_C,
        ];
        let Some((regs, _)) = run_regs(&program, &bits(&x), &[], &[]) else {
            return;
        };
        let want = [
            x[0] + x[1],
            x[2] + 0.2,
            x[3] + x[3],
            x[4] + x[4],
            x[5] + x[8],
            x[1].abs() - x[0],
            x[6] + x[0],
            x[0] * x[1],
            x[2] * 3.0,
            x[4] * x[0],
            x[5] * x[4],
            x[5].mul_add(x[4], x[0]),
            x[0].mul_add(2.0, x[1]),
            x[0].mul_add(x[1], 0.5),
            (-x[2]).mul_add(x[2], -x[7]),
        ];
        assert_f32(&regs[20..35], &want);
    }

    #[test]
    fn ftz_fmz_and_sat_flush_zero_and_clamp() {
        let x = [f32::from_bits(1), 2f32.powi(100), 2f32.powi(-100), 2f32.powi(-30), 0.0, f32::INFINITY, -f32::from_bits(1), 2.0, 0.5, f32::NAN];
        let program = [
            inst::fmul(10, 0, 1) | FP_FTZ,
            inst::fmul(11, 2, 3) | FP_FTZ,
            inst::fadd(12, 6, 6) | FP_FTZ,
            inst::fmul(13, 4, 5) | FP_FMZ,
            inst::ffma(14, 5, 4, 1) | FP_FMZ,
            inst::fadd(15, 7, 8) | FP_SAT,
            inst::fadd(16, 8, 7) | FP_SAT | FP_NEG_B,
            inst::fmul(17, 8, 8) | FP_SAT,
            inst::ffma(18, 9, 8, 8) | FP_SAT,
        ];
        let Some((regs, _)) = run_regs(&program, &bits(&x), &[], &[]) else {
            return;
        };
        assert_f32(&regs[10..19], &[0.0, 0.0, -0.0, 0.0, x[1], 1.0, 0.0, 0.25, 0.0]);
    }

    #[test]
    fn fmnmx_takes_min_on_true_predicate_and_skips_nan() {
        let mut seed = bits(&[1.0, -2.0, f32::NAN]);
        seed.resize(255, 0);
        seed[254] = 1;
        let program = [
            inst::fmnmx(10, 0, 1, 7, false),
            inst::fmnmx(11, 0, 1, 7, true),
            inst::fmnmx(12, 0, 2, 0, false),
            inst::fmnmx(13, 2, 1, 0, true),
            inst::fmnmx(14, 0, 1, 7, false) | FP_NEG_A | FP_ABS_B,
        ];
        let Some((regs, _)) = run_regs(&program, &seed, &[], &[]) else {
            return;
        };
        assert_f32(&regs[10..15], &[-2.0, 1.0, 1.0, -2.0, -1.0]);
    }

    #[test]
    fn fadd_of_inf_and_its_negation_is_nan() {
        let program = [inst::fadd(1, 0, 0) | FP_NEG_B];
        let Some((regs, _)) = run_regs(&program, &bits(&[f32::INFINITY]), &[], &[]) else {
            return;
        };
        if !crate::gpu::device().unwrap().signed_zero_inf_nan_preserve {
            return eprintln!("skipping: device lacks SignedZeroInfNanPreserve");
        }
        assert_f32(&regs[1..2], &[f32::NAN]);
    }

    // MUFU approximates on hardware and Vulkan only bounds GLSL.std.450 loosely, so this checks 4 ULP.
    #[test]
    fn mufu_ops_match_std_within_4_ulp() {
        let x = [4.0f32, 16.0, 3.0, 8.0, 0.125, 0.5];
        let program = [
            inst::mufu(10, MUFU_RCP, 0),
            inst::mufu(11, MUFU_RSQ, 0),
            inst::mufu(12, MUFU_SQRT, 1),
            inst::mufu(13, MUFU_EX2, 2),
            inst::mufu(14, MUFU_LG2, 3),
            inst::mufu(15, MUFU_SIN, 4),
            inst::mufu(16, MUFU_COS, 4),
            inst::mufu(17, MUFU_TANH, 5),
        ];
        let Some((regs, _)) = run_regs(&program, &bits(&x), &[], &[]) else {
            return;
        };
        let turn = x[4] * std::f32::consts::TAU;
        let want = [x[0].recip(), x[0].sqrt().recip(), x[1].sqrt(), x[2].exp2(), x[3].log2(), turn.sin(), turn.cos(), x[5].tanh()];
        for (i, (&got, want)) in regs[10..18].iter().zip(want).enumerate() {
            assert!(got.abs_diff(want.to_bits()) <= 4, "output {i}: got {}, want {want}", f32::from_bits(got));
        }
    }
}
