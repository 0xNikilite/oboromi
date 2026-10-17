#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::{PRED, UREG, run, run_regs, spirv_val, translate_module};
    use crate::tests::gpu_test::inst::{self, *};

    const RZ: u32 = 255;

    const IADD3_CARRY: u128 = 0x004fe2000791e0050000000004077210;
    const IADD3_NEG_C: u128 = 0x004fe2000791e8050000000004077210;
    const IADD3_CARRY_OBSERVER: u128 = 0x008fc600007e24ff000000ffff097210;
    const IADD3_X: u128 = 0x004fc80003fee4090000000700077210;
    const IABS: u128 = 0x004fc800000000000000000200017213;
    const IMNMX_U32: u128 = 0x004fc800000000000000000302017217;
    const IDP4A_S8: u128 = 0x004fc800000006040000000302017226;
    const IDP2A_LO_S16_S8: u128 = 0x004fc800000016040000000302017226;
    const IDP2A_HI_S16_S8: u128 = 0x004fc800000036040000000302017226;
    const IADD3_CBUF: u128 = 0x004fc80007ffe0030000000002017a10;
    const IADD3_UREG: u128 = 0x004fc8000fffe0030000000402057c10;
    const VABSDIFF_S32: u128 = 0x004fc800000e02040000000302017214;
    const VABSDIFF4_S8: u128 = 0x004fc800000e00040000000302017215;
    const VABSDIFF4_S8_ACC: u128 = 0x004fc800000e08040000000302017215;
    const CLMAD_LO: u128 = 0x004fc80000000008000000060402722c;
    const LEA_HI: u128 = 0x004fc800078f18040000000302017211;
    const LEA_HI_SX32: u128 = 0x004fc800078f1aff0000000302017211;
    const LEA_X: u128 = 0x004fc800038e1cff0000000302017211;
    const LEA_CARRY: u128 = 0x004fc800078000ff0000000302017211;

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
    fn mov_cx_validates_dynamic_constant_bank() {
        let module = translate_module(&[inst::mov_cbuf_ureg(7, 4, 0x10)], true, None);
        spirv_val(&module);
    }

    #[test]
    fn lea_shifts_then_adds() {
        let word = 0x004fc800078e18ff0000000302017211;
        let Some((regs, _)) = run_regs(&[word], &[0, 0, 3, 5], &[], &[]) else { return };
        assert_eq!(regs[1], 29);
    }

    #[test]
    fn lea_high_sign_extension_and_carry_input() {
        let mut seed = vec![0; 5];
        seed[2] = 0x4000_0000;
        seed[3] = 5;
        seed[4] = 2;
        let Some((regs, _)) = run_regs(&[LEA_HI], &seed, &[], &[]) else {
            return;
        };
        assert_eq!(regs[1], 0x17);
        let Some((regs, _)) = run_regs(&[LEA_HI_SX32], &seed, &[], &[]) else {
            return;
        };
        assert_eq!(regs[1], 7);
        seed[2] = 3;
        let Some((regs, _)) = run_regs(&[LEA_X], &seed, &[], &[]) else {
            return;
        };
        assert_eq!(regs[1], 30);
        seed.resize(255, 0);
        seed[2] = u32::MAX;
        seed[3] = 1;
        let Some((regs, _)) = run_regs(&[LEA_CARRY, IADD3_CARRY_OBSERVER], &seed, &[], &[])
        else {
            return;
        };
        assert_eq!([regs[1], regs[9]], [0, 1]);
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

    #[test]
    fn idp_and_vabsdiff_cover_packed_and_scalar_forms() {
        let mut seed = vec![0; 10];
        seed[2] = 0xfc03_fe01;
        seed[3] = 0xf8f9_0605;
        seed[4] = 10;
        let Some((regs, _)) = run_regs(&[IDP4A_S8], &seed, &[], &[]) else {
            return;
        };
        assert_eq!(regs[1], 14);

        seed[2] = 0xfffe_0001;
        seed[3] = 0x0403_0201;
        seed[4] = 10;
        let Some((regs, _)) = run_regs(&[IDP2A_LO_S16_S8, IDP2A_HI_S16_S8], &seed, &[], &[])
        else {
            return;
        };
        assert_eq!(regs[1], 5);

        seed[2] = 0x8000_0000;
        seed[3] = 1;
        seed[4] = 2;
        let Some((regs, _)) = run_regs(&[VABSDIFF_S32], &seed, &[], &[]) else {
            return;
        };
        assert_eq!(regs[1], 0x8000_0003);

        seed[2] = 0xff1e_1401;
        seed[3] = 0x0028_0a05;
        seed[4] = 7;
        let Some((regs, _)) = run_regs(&[VABSDIFF4_S8, VABSDIFF4_S8_ACC], &seed, &[], &[])
        else {
            return;
        };
        assert_eq!(regs[1], 286);
    }

    #[test]
    fn clmad_carryless_multiplies_and_accumulates() {
        let mut seed = vec![0; 10];
        seed[4] = 3;
        seed[6] = 5;
        seed[8] = 1;
        let Some((regs, _)) = run_regs(&[CLMAD_LO], &seed, &[], &[]) else {
            return;
        };
        assert_eq!(regs[2..4], [14, 0]);
    }

    #[test]
    fn iadd3_reads_cbuf_and_uniform_sources() {
        let ur4 = 254 + 1 + 4;
        let mut seed = vec![0; ur4 + 1];
        seed[2] = 3;
        seed[3] = 5;
        seed[ur4] = 7;
        let Some((regs, _)) = run_regs(&[IADD3_CBUF, IADD3_UREG], &seed, &[11], &[]) else {
            return;
        };
        assert_eq!([regs[1], regs[5]], [19, 15]);
    }

    #[test]
    fn lop3_selects_sources_and_combines_predicate_output() {
        let program = [
            alu3(LOP3 | lop3_fields(0xca, 7, 7, true), 10, 1, 2, 3),
            alu3(LOP3 ^ IMM_B | lop3_fields(0xc0, 7, 7, true), 11, 1, 0xffff, 3),
            alu3(LOP3 ^ IMM_B | lop3_fields(0xc0, 0, 7, true), RZ, 1, 0x8000_0000, RZ),
            alu3(LOP3 ^ IMM_B | lop3_fields(0xc0, 1, 4, false), RZ, 1, 0xff, RZ),
            alu3(LOP3 ^ IMM_B | PAND | lop3_fields(0xc0, 2, 4, true), RZ, 1, 0x8000_0000, RZ),
        ];
        let seed = [(1, 0xff00_ff00), (2, 0x0f0f_0f0f), (3, 0x3333_3333), (PRED, 1 << 4 | 1 << 2)];
        let Some(regs) = run(&program, &seed) else { return };
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
        let Some(regs) = run(&program, &seed) else { return };
        assert_eq!(regs[10..18], [2, 0x1234_5678, u32::MAX, 1, 0, u32::MAX, 0x2345_6780, 0xabcd]);
    }

    #[test]
    fn prmt_picks_bytes_in_every_mode() {
        let prmt = |mode: u32, rd: u32, sel: u32| alu3(PRMT ^ IMM_B | prmt_mode(mode), rd, 0, sel, 1);
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
        let Some(regs) = run(&program, &seed) else { return };
        let expected = [
            0x0382_0180, 0x0786_0584, 0x0580_00ff, 0x8403_8201, 0x0586_0780,
            0x8282_8282, 0x0382_0101, 0x8282_0180, 0x0382_0382, 0x0101_0101,
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
        let Some(regs) = run(&program, &[(0, 0x8000_0001), (1, u32::MAX)]) else { return };
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
        let Some(regs) = run(&program, &seed) else { return };
        assert_eq!(regs[10..19], [u32::MAX, 0, 31, 31, u32::MAX, u32::MAX, 30, u32::MAX, 1]);
    }

    #[test]
    fn brev_reverses_bits() {
        let program = [alu(BREV, 10, 0, 0), alu(BREV, 11, 0, 1), alu(BREV ^ IMM_B, 12, 0, 3)];
        let Some(regs) = run(&program, &[(0, 1), (1, 0x1234_5678)]) else { return };
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
        let Some(regs) = run(&program, &seed) else { return };
        assert_eq!(regs[10..19], [1, 0xff0, 0x8000_0000, u32::MAX, 0, u32::MAX, 0, 2, 0xf0]);
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
        let Some(regs) = run(&program, &seed) else { return };
        assert_eq!(regs[10..17], [0xffff_ff80, 0x80, 0, 0x1234_5678, 0x1234_5678, u32::MAX, 0xffff_ff80]);
    }
}
