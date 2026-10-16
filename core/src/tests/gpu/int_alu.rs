#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::{UREG, run_regs};
    use crate::tests::gpu_test::inst;

    const IADD3_CARRY: u128 = 0x004fe2000791e0050000000004077210;
    const IADD3_NEG_C: u128 = 0x004fe2000791e8050000000004077210;
    const IADD3_CARRY_OBSERVER: u128 = 0x008fc600007e24ff000000ffff097210;
    const IADD3_X: u128 = 0x004fc80003fee4090000000700077210;
    const IABS: u128 = 0x004fc800000000000000000200017213;
    const IMNMX_U32: u128 = 0x004fc800000000000000000302017217;

    #[test]
    fn translated_mov_and_iadd3_run_on_gpu() {
        let program = [inst::mov_imm(0, 42), inst::iadd_reg(1, 2, 3)];
        let Some((regs, _)) = run_regs(&program, &[0, 0, 40, 2], &[], &[]) else {
            return;
        };
        assert_eq!(regs[..3], [42, 42, 40]);
    }

    #[test]
    fn mov_reads_cbuf_and_uniform_register() {
        let mut seed = vec![0; UREG + 5];
        seed[UREG + 4] = 0x5678;
        let program = [inst::mov_cbuf(5, 0, 0x10), inst::mov_ureg(6, 4)];
        let Some((regs, _)) = run_regs(&program, &seed, &[0, 0, 0, 0, 0x1234], &[]) else {
            return;
        };
        assert_eq!(regs[5..7], [0x1234, 0x5678]);
    }

    #[test]
    fn imad_matches_silicon() {
        let word = 0x004fc800078e02090000000700077224;
        let cases = [
            ([0, 0, 0, 0], [0, 0]),
            ([1, 1, 0, 1], [2, 1]),
            ([0x7fffffff, 0x7fffffff, 0, 0x7fffffff], [0x80000000, 0x7fffffff]),
            ([0x80000000, 0x80000000, 0, 0x80000000], [0x80000000, 0x80000000]),
            ([0xffffffff, 0xffffffff, 0, 0xffffffff], [0, 0xffffffff]),
        ];
        for ([a, b, _, c], expected) in cases {
            let mut seed = vec![0; 10];
            seed[0] = a;
            seed[7] = b;
            seed[9] = c;
            let Some((regs, _)) = run_regs(&[word], &seed, &[], &[]) else { return };
            assert_eq!([regs[7], regs[9]], expected);
        }
    }

    #[test]
    fn lea_shifts_then_adds() {
        let word = 0x004fc800078e18ff0000000302017211;
        let Some((regs, _)) = run_regs(&[word], &[0, 0, 3, 5], &[], &[]) else { return };
        assert_eq!(regs[1], 29);
    }

    #[test]
    fn iadd3_carry_inputs_and_outputs_match_silicon() {
        let cases = [
            ([0, 0, 0], [0, 0]),
            ([1, 1, 1], [3, 0]),
            ([0x7fff_ffff; 3], [0x7fff_fffd, 1]),
            ([0x8000_0000; 3], [0x8000_0000, 1]),
            ([u32::MAX; 3], [0xffff_fffd, 2]),
        ];
        for ([a, b, c], expected) in cases {
            let mut seed = vec![0; 10];
            seed[0] = b;
            seed[4] = a;
            seed[5] = c;
            let Some((regs, _)) = run_regs(&[IADD3_CARRY, IADD3_CARRY_OBSERVER], &seed, &[], &[])
            else {
                return;
            };
            assert_eq!([regs[7], regs[9]], expected);
        }

        let mut seed = vec![0; 10];
        seed[4] = 1;
        let Some((regs, _)) = run_regs(&[IADD3_NEG_C, IADD3_CARRY_OBSERVER], &seed, &[], &[]) else {
            return;
        };
        assert_eq!([regs[7], regs[9]], [1, 1]);

        let mut seed = vec![0; 10];
        seed[0] = 1;
        seed[7] = 1;
        seed[9] = 1;
        let Some((regs, _)) = run_regs(&[IADD3_X], &seed, &[], &[]) else {
            return;
        };
        assert_eq!(regs[7], 5);
    }

    #[test]
    fn iabs_and_imnmx_cover_signedness_and_predicate() {
        for (value, expected) in [(0, 0), (1, 1), (u32::MAX, 1), (0x8000_0000, 0x8000_0000)] {
            let mut seed = vec![0; 255];
            seed[2] = value;
            let Some((regs, _)) = run_regs(&[IABS], &seed, &[], &[]) else {
                return;
            };
            assert_eq!(regs[1], expected);
        }

        let mut seed = vec![0; 255];
        seed[2] = u32::MAX;
        seed[3] = 1;
        seed[254] = 1;
        let Some((regs, _)) = run_regs(&[IMNMX_U32], &seed, &[], &[]) else {
            return;
        };
        assert_eq!(regs[1], 1);
        let signed = IMNMX_U32 | 1 << 73;
        let Some((regs, _)) = run_regs(&[signed], &seed, &[], &[]) else {
            return;
        };
        assert_eq!(regs[1], u32::MAX);
        seed[254] = 0;
        let Some((regs, _)) = run_regs(&[signed], &seed, &[], &[]) else {
            return;
        };
        assert_eq!(regs[1], 1);
    }

    #[test]
    fn imad_source_forms_and_modifiers() {
        let words = [
            0x004fc800078e02060000000502017824,
            0x004fc800078e02060000000002017a24,
            0x004fc8000f8e02060000000402017c24,
            0x004fc800078e02030000000502017424,
        ];
        let ur4 = 254 + 1 + 4;
        let mut seed = vec![0; ur4 + 1];
        seed[2] = 3;
        seed[3] = 5;
        seed[4] = 7;
        seed[6] = 11;
        seed[ur4] = 7;
        let Some((regs, _)) = run_regs(&words, &seed, &[7], &[]) else { return };
        assert_eq!(regs[1], 20);

        let wide = 0x004fc800078e00060000000302047225;
        let mut seed = vec![0; 8];
        seed[2] = u32::MAX;
        seed[3] = 2;
        seed[6] = 1;
        let Some((regs, _)) = run_regs(&[wide], &seed, &[], &[]) else { return };
        assert_eq!(regs[4..6], [u32::MAX, 1]);

        let high = 0x004fc800078e02060000000302017227;
        seed[2] = u32::MAX - 1;
        seed[3] = 3;
        let Some((regs, _)) = run_regs(&[high], &seed, &[], &[]) else { return };
        assert_eq!(regs[1], 0);

        let x = 0x004fc800000e06060000000302017224;
        seed.resize(255, 0);
        seed[2] = 2;
        seed[3] = 3;
        seed[6] = 4;
        seed[254] = 1;
        let Some((regs, _)) = run_regs(&[x], &seed, &[], &[]) else { return };
        assert_eq!(regs[1], 11);
        let invert_c = 0x004fc800000e0e060000000302017224;
        let Some((regs, _)) = run_regs(&[invert_c], &seed, &[], &[]) else { return };
        assert_eq!(regs[1], 2);
    }
}
