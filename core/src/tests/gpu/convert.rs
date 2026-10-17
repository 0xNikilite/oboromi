#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::run_regs;
    use crate::tests::gpu_test::inst::cvt::*;
    use crate::tests::gpu_test::inst::{f2f, f2i, frnd, i2f, i2i};

    fn pair(bits: u64) -> [u32; 2] {
        [bits as u32, (bits >> 32) as u32]
    }

    #[test]
    fn f2i_rounds_saturates_and_zeroes_nan() {
        let cases = [
            (S32, RN, 0.5f32, 0),
            (S32, RN, 1.5, 2),
            (S32, RN, -1.5, -2i32 as u32),
            (S32, RM, 1.5, 1),
            (S32, RP, -1.5, -1i32 as u32),
            (S32, RZ, 2147483648.0, i32::MAX as u32),
            (U32, RZ, 4294967296.0, u32::MAX),
            (S32, RZ, f32::NAN, i32::MIN as u32),
            (S32, RZ, f32::NEG_INFINITY, i32::MIN as u32),
        ];
        let seed: Vec<_> = cases.iter().map(|case| case.2.to_bits()).collect();
        let program: Vec<_> = (0..).zip(&cases).map(|(i, &(dst, rnd, ..))| f2i(16 + i, i, dst, F32, rnd)).collect();
        let Some((regs, _)) = run_regs(&program, &seed, &[], &[]) else {
            return;
        };
        let expected: Vec<_> = cases.iter().map(|case| case.3).collect();
        assert_eq!(regs[16..16 + cases.len()], expected);
    }

    #[test]
    fn f2i_f64_saturates_to_64_bits() {
        // llvmpipe truncates an f64 RoundEven.
        let llvmpipe = crate::gpu::device().is_ok_and(|d| d.is_llvmpipe());
        let cases = [
            if llvmpipe { (S64, RZ, 1.5f64, 1u64) } else { (S64, RN, 1.5, 2) },
            (S64, RZ, 9223372036854775808.0, i64::MAX as u64),
            (U64, RZ, f64::NAN, 0),
            (S64, RZ, f64::NEG_INFINITY, i64::MIN as u64),
        ];
        let seed: Vec<_> = cases.iter().flat_map(|case| pair(case.2.to_bits())).collect();
        let program: Vec<_> = (0..).zip(&cases).map(|(i, &(dst, rnd, ..))| f2i(16 + 2 * i, 2 * i, dst, F64, rnd)).collect();
        let Some((regs, _)) = run_regs(&program, &seed, &[], &[]) else {
            return;
        };
        let expected: Vec<_> = cases.iter().flat_map(|case| pair(case.3)).collect();
        assert_eq!(regs[16..16 + 2 * cases.len()], expected);
    }

    #[test]
    fn frnd_rounds_in_each_mode() {
        let cases = [(RN, 1.5f32, 2.0f32), (RM, -1.5, -2.0), (RP, -1.5, -1.0), (RZ, -1.5, -1.0)];
        let seed: Vec<_> = cases.iter().map(|case| case.1.to_bits()).collect();
        let program: Vec<_> = (0..).zip(&cases).map(|(i, &(rnd, ..))| frnd(16 + i, i, rnd)).collect();
        let Some((regs, _)) = run_regs(&program, &seed, &[], &[]) else {
            return;
        };
        let expected: Vec<_> = cases.iter().map(|case| case.2.to_bits()).collect();
        assert_eq!(regs[16..20], expected);
    }

    #[test]
    fn integer_conversions_saturate_and_widen() {
        let seed = [i32::MIN as u32, u32::MAX, -1i32 as u32, 300, -200i32 as u32, 0, u32::MAX, u32::MAX, 0, 0x8000_0000];
        let program = [
            i2f(16, 0, F32, S32),
            i2f(17, 1, F32, U32),
            i2i(18, 2, U8),
            i2i(19, 3, U8),
            i2i(20, 4, S8),
            i2f(21, 6, F32, U64),
            i2f(22, 8, F32, S64),
            i2f(24, 0, F64, S32),
        ];
        let Some((regs, _)) = run_regs(&program, &seed, &[], &[]) else {
            return;
        };
        let [lo, hi] = pair((i32::MIN as f64).to_bits());
        let expected = [
            (i32::MIN as f32).to_bits(),
            (u32::MAX as f32).to_bits(),
            0,
            u8::MAX as u32,
            i8::MIN as u32,
            (u64::MAX as f32).to_bits(),
            (i64::MIN as f32).to_bits(),
        ];
        assert_eq!(regs[16..23], expected);
        assert_eq!(regs[24..26], [lo, hi]);
    }

    #[test]
    fn f2f_round_trips_flushes_and_rounds_in_each_mode() {
        let halfway = pair((1.0f64 + 2f64.powi(-24)).to_bits());
        let negative_halfway = pair((-1.0f64 - 2f64.powi(-24)).to_bits());
        let seed = [[1.5f32.to_bits(), 1], [0, 0], halfway, negative_halfway].concat();
        let program = [
            f2f(16, 0, F64, F32, RN, false),
            f2f(18, 16, F32, F64, RN, false),
            f2f(20, 1, F64, F32, RN, true),
            f2f(22, 4, F32, F64, RN, false),
            f2f(23, 4, F32, F64, RM, false),
            f2f(24, 4, F32, F64, RP, false),
            f2f(25, 4, F32, F64, RZ, false),
            f2f(26, 6, F32, F64, RM, false),
            f2f(27, 6, F32, F64, RP, false),
        ];
        let Some((regs, _)) = run_regs(&program, &seed, &[], &[]) else {
            return;
        };
        let [one, minus_one] = [1.0f32.to_bits(), (-1.0f32).to_bits()];
        assert_eq!(regs[18], 1.5f32.to_bits());
        assert_eq!(regs[20..22], [0, 0]);
        assert_eq!(regs[22..28], [one, one, one + 1, one, minus_one + 1, minus_one]);
    }
}
