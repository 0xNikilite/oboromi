#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::{UREG, run_regs, spirv_val, translate_module};
    use crate::tests::gpu_test::inst;

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

}
