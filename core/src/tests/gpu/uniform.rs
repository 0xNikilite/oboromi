#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::{UPRED, UREG, run_regs};
    use crate::tests::gpu_test::inst::*;

    fn run(program: &[u128], seed: &[(usize, u32)]) -> Option<Vec<u32>> {
        let mut words = vec![0; UPRED + 1];
        for &(slot, value) in seed {
            words[slot] = value;
        }
        run_regs(program, &words, &[], &[]).map(|(regs, _)| regs)
    }

    #[test]
    fn uiadd3_matches_iadd3_silicon() {
        let program = [
            alu3(UIADD3 | preds(0, 1, 7, 7), 12, 0, 7, 9),
            alu3(UIADD3 | NEG_A | preds(7, 7, 7, 7), 10, 0, 7, 9),
            alu3(UIADD3 | EXTENDED | preds(7, 7, 7, 7), 11, 0, 7, 9),
        ];
        let cases = [
            (0, [0, 0, 0, 2]),
            (1, [3, 0, 1, 5]),
            (0x7fff_ffff, [0x7fff_fffd, 1, 0x7fff_ffff, 0x7fff_ffff]),
            (0x8000_0000, [0x8000_0000, 1, 0x8000_0000, 0x8000_0002]),
            (u32::MAX, [u32::MAX - 2, 2, u32::MAX, u32::MAX]),
        ];
        for (v, [sum, carries, negated, extended]) in cases {
            let seed = [(UREG, v), (UREG + 7, v), (UREG + 9, v)];
            let Some(regs) = run(&program, &seed) else { return };
            assert_eq!(regs[UREG + 10..UREG + 13], [negated, extended, sum]);
            assert_eq!((regs[UPRED] & 1) + (regs[UPRED] >> 1 & 1), carries);
        }
    }

    #[test]
    fn uimad_matches_imad_silicon() {
        let cases = [
            ([0, 0, 0], 0),
            ([1, 1, 1], 2),
            ([0x7fff_ffff, 0x7fff_ffff, 0x7fff_ffff], 0x8000_0000),
            ([0x8000_0000, 0x8000_0000, 0x8000_0000], 0x8000_0000),
            ([u32::MAX, u32::MAX, u32::MAX], 0),
        ];
        for ([a, b, c], expected) in cases {
            let seed = [(UREG, a), (UREG + 7, b), (UREG + 9, c)];
            let Some(regs) = run(&[alu3(UIMAD | S32, 7, 0, 7, 9)], &seed) else { return };
            assert_eq!(regs[UREG + 7], expected);
        }
    }

    #[test]
    fn uniform_moves_lea_and_wide_arithmetic() {
        let program = [
            alu(UMOV_IMM, 1, 0, 0xdead_beef),
            alu(UMOV, 2, 0, 1),
            alu(R2UR, 3, 5, 0),
            alu(USEL ^ IMM_B | preds(0, 0, 1, 0), 4, 3, 7),
            alu(USEL ^ IMM_B | preds(0, 0, 2, 0), 5, 3, 7),
            alu3(ULEA | lea_shift(3) | preds(7, 0, 0, 0), 6, 3, 1, 63),
            alu3(UIMAD_WIDE ^ IMM_B | preds(7, 0, 0, 0), 8, 3, 0x10, 10),
            alu3(UIADD3_64 ^ IMM_B | preds(7, 7, 7, 7), 12, 10, u32::MAX, 63),
        ];
        let seed = [(5, 0x1234), (UREG + 10, u32::MAX), (UREG + 11, 1), (UPRED, 0b10)];
        let Some(regs) = run(&program, &seed) else { return };
        assert_eq!(
            regs[UREG + 1..UREG + 14],
            [0xdead_beef, 0xdead_beef, 0x1234, 0x1234, 7, 0xdeae_508f, 0, 0x0001_233f, 2, u32::MAX, 1, u32::MAX - 1, 1]
        );
    }

    #[test]
    fn uniform_predicate_ops() {
        let program = [
            alu(UISETP | ICMP_LT | preds(0, 1, 7, 0), 0, 5, 6),
            alu(UISETP | ICMP_LT | S32 | preds(2, 3, 7, 0), 0, 6, 5),
            alu(UPLOP3 | preds(4, 5, 0, 1) | plop3_fields(0xe8, 0x96, 2), 0, 0, 0),
            alu(UP2UR_IMM | byte_select(1), 7, 8, 0xff),
            alu(UR2UP_IMM | byte_select(2), 0, 9, 0xf),
        ];
        let seed = [(UREG + 5, 1), (UREG + 6, u32::MAX), (UREG + 8, 0xaaaa_aaaa), (UREG + 9, 0x000a_0000)];
        let Some(regs) = run(&program, &seed) else { return };
        assert_eq!([regs[UREG + 7], regs[UPRED]], [0xaaaa_95aa, 0x1a]);
    }
}
