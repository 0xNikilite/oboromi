#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::run_regs;

    #[test]
    fn isetp_writes_complementary_predicates() {
        let word = 0x004fc80003906270000000030200720c;
        let mut seed = vec![0; 255];
        seed[2] = u32::MAX;
        seed[3] = 1;
        let Some(regs) = run_regs(&[word], &seed, &[]) else { return };
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
            let Some(regs) = run_regs(&[word], &seed, &[]) else { return };
            assert_eq!(regs[254] & 1 != 0, expected, "cmp {cmp}");
        }
        let unsigned_lt = base & !(7 << 76) & !(1 << 73) | 1 << 76;
        let mut seed = vec![0; 255];
        seed[2] = u32::MAX;
        seed[3] = 1;
        let Some(regs) = run_regs(&[unsigned_lt], &seed, &[]) else { return };
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
            let Some(regs) = run_regs(&[word], &seed, cbuf) else { return };
            assert_eq!(regs[254] & 3, 1);
        }

        let or_p2 = base & !(3 << 74) & !(7 << 87) | 1 << 74 | 2 << 87;
        let xor_p2 = base & !(3 << 74) & !(7 << 87) | 2 << 74 | 2 << 87;
        seed[3] = 6;
        seed[254] = 1 << 2;
        let Some(regs) = run_regs(&[or_p2], &seed, &[]) else { return };
        assert_eq!(regs[254] & 3, 3);
        let Some(regs) = run_regs(&[xor_p2], &seed, &[]) else { return };
        assert_eq!(regs[254] & 3, 1);
    }

    #[test]
    fn isetp_extended_uses_low_word_result() {
        let word = 0x004fc80003906320000000030200720c;
        let mut seed = vec![0; 255];
        seed[2] = 5;
        seed[3] = 5;
        seed[254] = 1 << 2;
        let Some(regs) = run_regs(&[word], &seed, &[]) else { return };
        assert_eq!(regs[254] & 3, 1);
    }
}
