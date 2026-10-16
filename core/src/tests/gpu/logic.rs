#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::run_regs;
    use crate::tests::gpu_test::inst::*;

    const RZ: u32 = 255;
    const URZ: u32 = 63;
    const PRED: usize = 254;
    const UR0: usize = 255;
    const UPRED: usize = UR0 + 63;

    fn run(program: &[u128], seed: &[(usize, u32)]) -> Option<Vec<u32>> {
        let mut words = vec![0; UPRED + 1];
        for &(slot, value) in seed {
            words[slot] = value;
        }
        run_regs(program, &words, &[], &[]).map(|(regs, _)| regs)
    }

    #[test]
    fn lop3_selects_sources_and_combines_predicate_output() {
        let program = [
            alu3(LOP3 | lop3_fields(0xca, 7, 7, true), 10, 1, 2, 3),
            alu3(
                LOP3 ^ IMM_B | lop3_fields(0xc0, 7, 7, true),
                11,
                1,
                0xffff,
                3,
            ),
            alu3(
                LOP3 ^ IMM_B | lop3_fields(0xc0, 0, 7, true),
                RZ,
                1,
                0x8000_0000,
                RZ,
            ),
            alu3(
                LOP3 ^ IMM_B | lop3_fields(0xc0, 1, 4, false),
                RZ,
                1,
                0xff,
                RZ,
            ),
            alu3(
                LOP3 ^ IMM_B | PAND | lop3_fields(0xc0, 2, 4, true),
                RZ,
                1,
                0x8000_0000,
                RZ,
            ),
        ];
        let seed = [
            (1, 0xff00_ff00),
            (2, 0x0f0f_0f0f),
            (3, 0x3333_3333),
            (PRED, 1 << 4 | 1 << 2),
        ];
        let Some(regs) = run(&program, &seed) else {
            return;
        };
        assert_eq!(regs[10..12], [0x0f33_0f33, 0xff00]);
        assert_eq!(regs[PRED], 1 << 4 | 1 << 1 | 1);
    }

    #[test]
    fn shf_funnels_high_words_wraps_and_shifts_64_bit() {
        let program = [
            alu3(SHF | SHF_U32 | SHF_HI | WRAP, 10, 0, 2, 1),
            alu3(SHF | SHF_U32 | SHF_HI, 11, 0, 2, 1),
            alu3(SHF ^ IMM_B | SHF_R | SHF_S32 | SHF_HI, 12, RZ, 31, 1),
            alu3(SHF ^ IMM_B | SHF_R | SHF_U32 | SHF_HI, 13, RZ, 31, 1),
            alu3(SHF ^ IMM_B | SHF_R | SHF_U64, 14, 0, 64, 1),
            alu3(SHF ^ IMM_B | SHF_R | SHF_HI, 15, 0, 64, 1),
            alu3(SHF ^ IMM_B | SHF_U64 | SHF_HI, 16, 0, 36, 1),
            alu3(SHF ^ IMM_C | SHF_R | SHF_U32, 17, 0, 0xabcd, 2),
        ];
        let seed = [(0, 0x1234_5678), (1, 0x8000_0001), (2, 33)];
        let Some(regs) = run(&program, &seed) else {
            return;
        };
        assert_eq!(
            regs[10..18],
            [
                2,
                0x1234_5678,
                u32::MAX,
                1,
                0,
                u32::MAX,
                0x2345_6780,
                0xabcd
            ]
        );
    }

    #[test]
    fn prmt_picks_bytes_in_every_mode() {
        let prmt =
            |mode: u32, rd: u32, sel: u32| alu3(PRMT ^ IMM_B | prmt_mode(mode), rd, 0, sel, 1);
        let program = [
            prmt(0, 10, 0x3210),
            prmt(0, 11, 0x7654),
            prmt(0, 12, 0x5098),
            prmt(1, 13, 1),
            prmt(2, 14, 0),
            prmt(3, 15, 2),
            prmt(4, 16, 1),
            prmt(5, 17, 2),
            prmt(6, 18, 1),
            alu3(PRMT | prmt_mode(3), 19, 0, 2, 1),
        ];
        let seed = [(0, 0x0382_0180), (1, 0x0786_0584), (2, 0x11)];
        let Some(regs) = run(&program, &seed) else {
            return;
        };
        let expected = [
            0x0382_0180,
            0x0786_0584,
            0x0580_00ff,
            0x8403_8201,
            0x0586_0780,
            0x8282_8282,
            0x0382_0101,
            0x8282_0180,
            0x0382_0382,
            0x0101_0101,
        ];
        assert_eq!(regs[10..20], expected);
    }

    #[test]
    fn popc_counts_set_bits_of_inverted_registers_only() {
        let program = [
            alu(POPC, 10, 0, 0),
            alu(POPC | INVERT_B, 11, 0, 0),
            alu(POPC, 12, 0, 1),
            alu(POPC, 13, 0, RZ),
            alu(POPC ^ IMM_B, 14, 0, 0x8000_000f),
        ];
        let Some(regs) = run(&program, &[(0, 0x8000_0001), (1, u32::MAX)]) else {
            return;
        };
        assert_eq!(regs[10..15], [2, 30, 32, 0, 5]);
    }

    #[test]
    fn flo_finds_leading_one_or_sign_bit() {
        let program = [
            alu(FLO, 10, 0, 0),
            alu(FLO, 11, 0, 1),
            alu(FLO, 12, 0, 2),
            alu(FLO | FLO_SH, 13, 0, 1),
            alu(FLO | FLO_SH, 14, 0, 0),
            alu(FLO | S32, 15, 0, 3),
            alu(FLO | S32, 16, 0, 2),
            alu(FLO | INVERT_B, 17, 0, 3),
            alu(FLO | S32 | FLO_SH, 18, 0, 4),
        ];
        let seed = [(1, 1), (2, 0x8000_0000), (3, u32::MAX), (4, 0x7fff_ffff)];
        let Some(regs) = run(&program, &seed) else {
            return;
        };
        assert_eq!(
            regs[10..19],
            [u32::MAX, 0, 31, 31, u32::MAX, u32::MAX, 30, u32::MAX, 1]
        );
    }

    #[test]
    fn brev_reverses_bits() {
        let program = [
            alu(BREV, 10, 0, 0),
            alu(BREV, 11, 0, 1),
            alu(BREV ^ IMM_B, 12, 0, 3),
        ];
        let Some(regs) = run(&program, &[(0, 1), (1, 0x1234_5678)]) else {
            return;
        };
        assert_eq!(regs[10..13], [0x8000_0000, 0x1e6a_2c48, 0xc000_0000]);
    }

    #[test]
    fn bmsk_clamps_or_wraps_position_and_width() {
        let program = [
            alu(BMSK, 10, 0, 1),
            alu(BMSK, 11, 2, 3),
            alu(BMSK, 12, 4, 5),
            alu(BMSK, 13, 0, 6),
            alu(BMSK, 14, 6, 1),
            alu(BMSK, 15, 0, 7),
            alu(BMSK | WRAP, 16, 0, 6),
            alu(BMSK | WRAP, 17, 7, 1),
            alu(BMSK ^ IMM_B, 18, 2, 4),
        ];
        let seed = [(1, 1), (2, 4), (3, 8), (4, 31), (5, 2), (6, 32), (7, 33)];
        let Some(regs) = run(&program, &seed) else {
            return;
        };
        assert_eq!(
            regs[10..19],
            [1, 0xff0, 0x8000_0000, u32::MAX, 0, u32::MAX, 0, 2, 0xf0]
        );
    }

    #[test]
    fn sgxt_extends_from_clamped_or_wrapped_width() {
        let program = [
            alu(SGXT ^ IMM_B | S32, 10, 0, 8),
            alu(SGXT ^ IMM_B, 11, 1, 8),
            alu(SGXT ^ IMM_B | S32, 12, 2, 0),
            alu(SGXT ^ IMM_B | S32, 13, 2, 32),
            alu(SGXT ^ IMM_B | S32, 14, 2, 33),
            alu(SGXT ^ IMM_B | S32 | WRAP, 15, 3, 33),
            alu(SGXT | S32, 16, 0, 4),
        ];
        let seed = [(0, 0x80), (1, 0x180), (2, 0x1234_5678), (3, 1), (4, 8)];
        let Some(regs) = run(&program, &seed) else {
            return;
        };
        assert_eq!(
            regs[10..17],
            [
                0xffff_ff80,
                0x80,
                0,
                0x1234_5678,
                0x1234_5678,
                u32::MAX,
                0xffff_ff80
            ]
        );
    }

    #[test]
    fn uniform_twins_read_and_write_uniform_registers() {
        let program = [
            alu3(ULOP3 | lop3_fields(0xc0, 7, 7, true), 10, 1, 3, URZ),
            alu3(
                ULOP3 ^ IMM_B | lop3_fields(0xc0, 1, 7, true),
                URZ,
                2,
                1,
                URZ,
            ),
            alu3(USHF ^ IMM_B | SHF_R | SHF_U32 | SHF_HI, 11, URZ, 31, 2),
            alu3(UPRMT ^ IMM_B, 12, 1, 0x0123, URZ),
            alu(UPOPC, 13, 0, 2),
            alu(UFLO, 14, 0, 2),
            alu(UBREV, 15, 0, 1),
            alu(UBMSK ^ IMM_B, 16, 4, 8),
            alu(USGXT ^ IMM_B | S32, 17, 3, 16),
        ];
        let seed = [
            (UR0 + 1, 0x1234_5678),
            (UR0 + 2, 0x8000_0001),
            (UR0 + 3, 0xff00_ff00),
            (UR0 + 4, 4),
        ];
        let Some(regs) = run(&program, &seed) else {
            return;
        };
        let expected = [
            0x1200_5600,
            1,
            0x7856_3412,
            2,
            31,
            0x1e6a_2c48,
            0xff0,
            0xffff_ff00,
        ];
        assert_eq!(regs[UR0 + 10..UR0 + 18], expected);
        assert_eq!(regs[UPRED], 1 << 1);
    }
}
