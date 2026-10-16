#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::run_regs;
    use crate::tests::gpu_test::inst;

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
        let ur4 = 254 + 1 + 4;
        let mut seed = vec![0; ur4 + 1];
        seed[ur4] = 0x5678;
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
