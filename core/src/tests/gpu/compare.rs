#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::run_regs;

    const CONTROL: u128 = 0x004fc800000000000000000000000000;
    const FSETP: u128 = CONTROL | 0x20b | 7 << 12 | 2 << 24 | 3 << 32 | 1 << 84 | 7 << 87;
    const FSET_BF: u128 =
        CONTROL | 0x20a | 7 << 12 | 1 << 16 | 2 << 24 | 3 << 32 | 2 << 76 | 7 << 87;
    const FSEL: u128 = CONTROL | 0x208 | 7 << 12 | 1 << 16 | 2 << 24 | 3 << 32;
    const SEL: u128 = CONTROL | 0x207 | 7 << 12 | 1 << 16 | 2 << 24 | 3 << 32;
    const R2P: u128 = CONTROL | 0x804 | 7 << 12 | 2 << 24 | 5 << 32 | 1 << 76;
    const P2R: u128 = CONTROL | 0x803 | 7 << 12 | 1 << 16 | 3 << 24 | 0xff << 32 | 2 << 76;
    const PLOP3: u128 = 0x004fc80000703922000000000000781c;

    fn float_compare(op: u32, a: f32, b: f32) -> bool {
        let unordered = a.is_nan() || b.is_nan();
        match op {
            0 => false,
            1 => !unordered && a < b,
            2 => !unordered && a == b,
            3 => !unordered && a <= b,
            4 => !unordered && a > b,
            5 => !unordered && a != b,
            6 => !unordered && a >= b,
            7 => !unordered,
            8 => unordered,
            9 => unordered || a < b,
            10 => unordered || a == b,
            11 => unordered || a <= b,
            12 => unordered || a > b,
            13 => unordered || a != b,
            14 => unordered || a >= b,
            15 => true,
            _ => unreachable!(),
        }
    }

    #[test]
    fn isetp_writes_complementary_predicates() {
        let word = 0x004fc80003906270000000030200720c;
        let mut seed = vec![0; 255];
        seed[2] = u32::MAX;
        seed[3] = 1;
        let Some((regs, _)) = run_regs(&[word], &seed, &[], &[]) else { return };
        assert_eq!(regs[254] & 3, 2);
    }

    #[test]
    fn isetp_compare_ops_and_signedness() {
        let base = 0x004fc80003906270000000030200720c_u128;
        let cases = [(1, true), (2, false), (3, true), (4, false), (5, true), (6, false)];
        for (cmp, expected) in cases {
            let word = base & !(7 << 76) | cmp << 76;
            let mut seed = vec![0; 255];
            seed[2] = u32::MAX;
            seed[3] = 1;
            let Some((regs, _)) = run_regs(&[word], &seed, &[], &[]) else { return };
            assert_eq!(regs[254] & 1 != 0, expected, "cmp {cmp}");
        }
        let unsigned_lt = base & !(7 << 76) & !(1 << 73) | 1 << 76;
        let mut seed = vec![0; 255];
        seed[2] = u32::MAX;
        seed[3] = 1;
        let Some((regs, _)) = run_regs(&[unsigned_lt], &seed, &[], &[]) else { return };
        assert_eq!(regs[254] & 1, 0);
    }

    #[test]
    fn isetp_combines_and_selects_sources() {
        let base = 0x004fc80003902270000000030200720c_u128;
        let source_mask = !(0xffff_ffff_u128 << 32 | 0xfff);
        let source_words = [
            base & source_mask | 0x80c | 5 << 32,
            base & source_mask | 0xa0c,
            (base & source_mask | 0xc0c | 4 << 32) | 1 << 91,
        ];
        let ur4 = 254 + 1 + 4;
        let mut seed = vec![0; ur4 + 1];
        seed[2] = 5;
        seed[ur4] = 5;
        for (word, cbuf) in source_words.into_iter().zip([&[][..], &[5][..], &[][..]]) {
            let Some((regs, _)) = run_regs(&[word], &seed, cbuf, &[]) else { return };
            assert_eq!(regs[254] & 3, 1);
        }

        let or_p2 = base & !(3 << 74) & !(7 << 87) | 1 << 74 | 2 << 87;
        let xor_p2 = base & !(3 << 74) & !(7 << 87) | 2 << 74 | 2 << 87;
        seed[3] = 6;
        seed[254] = 1 << 2;
        let Some((regs, _)) = run_regs(&[or_p2], &seed, &[], &[]) else { return };
        assert_eq!(regs[254] & 3, 3);
        let Some((regs, _)) = run_regs(&[xor_p2], &seed, &[], &[]) else { return };
        assert_eq!(regs[254] & 3, 1);
    }

    #[test]
    fn isetp_extended_uses_low_word_result() {
        let word = 0x004fc80003906320000000030200720c;
        let mut seed = vec![0; 255];
        seed[2] = 5;
        seed[3] = 5;
        seed[254] = 1 << 2;
        let Some((regs, _)) = run_regs(&[word], &seed, &[], &[]) else { return };
        assert_eq!(regs[254] & 3, 1);
    }

    #[test]
    fn fsetp_float_compare_ops() {
        for op in 0..16 {
            for (a, b) in [(-1.0_f32, 1.0_f32), (f32::NAN, 1.0_f32)] {
                let word = FSETP | (op as u128) << 76;
                let mut seed = vec![0; 255];
                seed[2] = a.to_bits();
                seed[3] = b.to_bits();
                let Some((regs, _)) = run_regs(&[word], &seed, &[], &[]) else {
                    return;
                };
                let expected = float_compare(op, a, b) as u32;
                assert_eq!(regs[254] & 3, expected | ((!expected & 1) << 1), "op {op}");
            }
        }
    }

    #[test]
    fn fsetp_modifiers_ftz_and_predicate_combine() {
        let modified = FSETP | 1 << 76 | 1 << 73 | 1 << 63;
        let mut seed = vec![0; 255];
        seed[2] = (-2.0_f32).to_bits();
        seed[3] = (-1.0_f32).to_bits();
        let Some((regs, _)) = run_regs(&[modified], &seed, &[], &[]) else {
            return;
        };
        assert_eq!(regs[254] & 3, 2);

        let ftz = (FSETP & !(0xf << 76)) | 2 << 76 | 1 << 80;
        seed[2] = 1;
        seed[3] = 0;
        let Some((regs, _)) = run_regs(&[ftz], &seed, &[], &[]) else {
            return;
        };
        assert_eq!(regs[254] & 3, 1);

        let combined = (FSETP & !(0xf << 76) & !(7 << 87)) | 1 << 74 | 2 << 87;
        seed[2] = 1.0_f32.to_bits();
        seed[3] = 1.0_f32.to_bits();
        seed[254] = 1 << 2;
        let Some((regs, _)) = run_regs(&[combined], &seed, &[], &[]) else {
            return;
        };
        assert_eq!(regs[254] & 3, 3);
    }

    #[test]
    fn fset_writes_one_or_zero() {
        let mut seed = vec![0; 255];
        seed[2] = 1.0_f32.to_bits();
        seed[3] = 1.0_f32.to_bits();
        let Some((regs, _)) = run_regs(&[FSET_BF], &seed, &[], &[]) else {
            return;
        };
        assert_eq!(regs[1], 1.0_f32.to_bits());
        seed[3] = 2.0_f32.to_bits();
        let Some((regs, _)) = run_regs(&[FSET_BF], &seed, &[], &[]) else {
            return;
        };
        assert_eq!(regs[1], 0);
    }

    #[test]
    fn predicate_lut_and_register_moves() {
        for inputs in 0..8_u32 {
            let mut seed = vec![0; 255];
            seed[254] = inputs;
            let Some((regs, _)) = run_regs(&[PLOP3], &seed, &[], &[]) else {
                return;
            };
            let index = (inputs & 1) << 2 | inputs & 2 | inputs >> 2 & 1;
            assert_eq!(regs[254] & 1, (0xca >> index) & 1);
        }

        let mut seed = vec![0; 255];
        seed[2] = 0x500;
        seed[3] = 0xaaaa_5555;
        let Some((regs, _)) = run_regs(&[R2P, P2R], &seed, &[], &[]) else {
            return;
        };
        assert_eq!(regs[254] & 0x7f, 5);
        assert_eq!(regs[1], 0xaa85_5555);
    }

    #[test]
    fn integer_and_float_select_use_predicate() {
        let mut seed = vec![0; 255];
        seed[2] = (-2.0_f32).to_bits();
        seed[3] = 3.0_f32.to_bits();
        seed[254] = 1;
        let fsel = FSEL | 1 << 73 | 1 << 72;
        let Some((regs, _)) = run_regs(&[fsel], &seed, &[], &[]) else {
            return;
        };
        assert_eq!(regs[1], (-2.0_f32).to_bits());

        seed[2] = 2;
        seed[3] = 3;
        let Some((regs, _)) = run_regs(&[SEL], &seed, &[], &[]) else {
            return;
        };
        assert_eq!(regs[1], 2);
        seed[254] = 0;
        let Some((regs, _)) = run_regs(&[SEL], &seed, &[], &[]) else {
            return;
        };
        assert_eq!(regs[1], 3);
    }
}
