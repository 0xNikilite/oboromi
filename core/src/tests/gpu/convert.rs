#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::run_regs;

    const PT: u128 = 7 << 12;
    const F2F_R: u128 = 0x310 | PT;
    const F2I_R: u128 = 0x305 | PT;
    const FRND_R: u128 = 0x307 | PT;
    const I2F_R: u128 = 0x306 | PT;
    const I2I_R: u128 = 0x238 | PT;

    fn f2f(rd: u32, rb: u32, fmt: u32, rnd: u32, ftz: bool) -> u128 {
        F2F_R
            | (rd as u128) << 16
            | (rb as u128) << 32
            | ((fmt & 7) as u128) << 75
            | ((fmt >> 3) as u128) << 84
            | (rnd as u128) << 78
            | (ftz as u128) << 80
    }

    fn f2i(rd: u32, rb: u32, dst: u32, rnd: u32, ftz: bool) -> u128 {
        F2I_R
            | (rd as u128) << 16
            | (rb as u128) << 32
            | ((dst & 1) as u128) << 72
            | ((dst >> 1) as u128) << 75
            | 2 << 84
            | (rnd as u128) << 78
            | (ftz as u128) << 80
    }

    fn f2i64(rd: u32, rb: u32, dst: u32, rnd: u32) -> u128 {
        (0x311 | PT)
            | (rd as u128) << 16
            | (rb as u128) << 32
            | ((dst & 1) as u128) << 72
            | ((dst >> 1) as u128) << 75
            | 3 << 84
            | (rnd as u128) << 78
    }

    fn frnd(rd: u32, rb: u32, rnd: u32, ftz: bool) -> u128 {
        let fmt = 10u128;
        FRND_R
            | (rd as u128) << 16
            | (rb as u128) << 32
            | (fmt & 3) << 75
            | (fmt >> 2) << 84
            | (rnd as u128) << 78
            | (ftz as u128) << 80
    }

    fn i2f(rd: u32, rb: u32, src: u32, rnd: u32) -> u128 {
        I2F_R
            | (rd as u128) << 16
            | (rb as u128) << 32
            | 2 << 75
            | ((src & 1) as u128) << 74
            | ((src >> 1) as u128) << 84
            | (rnd as u128) << 78
    }

    fn i2f_wide(rd: u32, rb: u32, dst: u32, src: u32) -> u128 {
        (0x312 | PT)
            | (rd as u128) << 16
            | (rb as u128) << 32
            | (dst as u128) << 75
            | ((src & 1) as u128) << 74
            | ((src >> 1) as u128) << 84
    }

    fn i2i(rd: u32, rb: u32, dst: u32) -> u128 {
        I2I_R | (rd as u128) << 16 | (rb as u128) << 32 | (dst as u128) << 76
    }

    #[test]
    fn conversion_words_match_nvdisasm() {
        let fixtures = [
            (f2f(0, 2, 19, 0, false), "F2F.F64.F32 R0, R2 ;"),
            (f2f(13, 14, 26, 2, false), "F2F.F32.F64.RP R13, R14 ;"),
            (f2i(1, 3, 5, 0, false), "F2I R1, R3 ;"),
            (f2i64(10, 12, 7, 0), "F2I.S64.F64 R10, R12 ;"),
            (frnd(4, 5, 0, false), "FRND R4, R5 ;"),
            (i2f(6, 7, 5, 0), "I2F R6, R7 ;"),
            (i2f_wide(11, 12, 2, 6), "I2F.U64 R11, R12 ;"),
            (i2i(8, 9, 0), "I2I.U8.S32.SAT R8, R9;"),
        ];
        let path = std::env::temp_dir().join(format!("oboromi-convert-{}.bin", std::process::id()));
        for (word, expected) in fixtures {
            std::fs::write(&path, word.to_le_bytes()).unwrap();
            let output = std::process::Command::new("/usr/local/cuda-13.1/bin/nvdisasm")
                .args(["--binary", "SM86"])
                .arg(&path)
                .output()
                .unwrap();
            let text = String::from_utf8_lossy(&output.stdout);
            let disasm = text
                .lines()
                .find_map(|line| line.split("/*0000*/").nth(1))
                .unwrap();
            assert_eq!(disasm.trim(), expected, "{word:#034x}");
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn f2i_rounds_and_clamps() {
        let values = [
            0.5f32,
            1.5,
            -1.5,
            2147483648.0,
            4294967296.0,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            0.0,
            -0.0,
            f32::from_bits(1),
        ];
        let seed: Vec<_> = values.iter().map(|x| x.to_bits()).collect();
        let program = [
            f2i(16, 0, 5, 0, false),
            f2i(17, 1, 5, 0, false),
            f2i(18, 2, 5, 0, false),
            f2i(19, 3, 5, 3, false),
            f2i(20, 4, 4, 3, false),
            f2i(21, 5, 5, 3, false),
            f2i(22, 6, 5, 3, false),
            f2i(23, 7, 5, 3, false),
            f2i(24, 1, 5, 1, false),
            f2i(25, 2, 5, 2, false),
            f2i(26, 8, 5, 3, false),
            f2i(27, 9, 5, 3, false),
            f2i(28, 10, 5, 3, true),
        ];
        let Some((regs, _)) = run_regs(&program, &seed, &[], &[]) else {
            return;
        };
        assert_eq!(
            regs[16..24],
            [
                0,
                2,
                (-2i32) as u32,
                i32::MAX as u32,
                u32::MAX,
                0,
                i32::MAX as u32,
                i32::MIN as u32
            ]
        );
        assert_eq!(regs[24..29], [1, (-1i32) as u32, 0, 0, 0]);
    }

    #[test]
    fn f2i_f64_clamps_to_64_bits() {
        let values = [1.5f64, 9223372036854775808.0, f64::NAN, f64::NEG_INFINITY];
        let seed: Vec<_> = values
            .iter()
            .flat_map(|x| {
                let bits = x.to_bits();
                [bits as u32, (bits >> 32) as u32]
            })
            .collect();
        let skip_nearest = crate::gpu::device().is_ok_and(|d| d.is_llvmpipe());
        if skip_nearest {
            eprintln!("skipping f64 round-to-nearest on llvmpipe");
        }
        let program = [
            f2i64(16, 0, 7, if skip_nearest { 3 } else { 0 }),
            f2i64(18, 2, 7, 3),
            f2i64(20, 4, 6, 3),
            f2i64(22, 6, 7, 3),
        ];
        let Some((regs, _)) = run_regs(&program, &seed, &[], &[]) else {
            return;
        };
        let expected = [if skip_nearest { 1 } else { 2i64 } as u64, i64::MAX as u64, 0, i64::MIN as u64];
        for (i, value) in expected.into_iter().enumerate() {
            assert_eq!(
                [regs[16 + i * 2], regs[17 + i * 2]],
                [value as u32, (value >> 32) as u32]
            );
        }
    }

    #[test]
    fn frnd_matches_rust_rounding() {
        let values = [1.5f32, -1.5, -0.0, f32::from_bits(1)];
        let seed: Vec<_> = values.iter().map(|x| x.to_bits()).collect();
        let program = [
            frnd(16, 0, 0, false),
            frnd(17, 1, 1, false),
            frnd(18, 1, 2, false),
            frnd(19, 2, 3, false),
            frnd(20, 3, 0, true),
        ];
        let Some((regs, _)) = run_regs(&program, &seed, &[], &[]) else {
            return;
        };
        assert_eq!(
            regs[16..21],
            [
                1.5f32.round_ties_even().to_bits(),
                (-1.5f32).floor().to_bits(),
                (-1.5f32).ceil().to_bits(),
                (-0.0f32).trunc().to_bits(),
                0
            ]
        );
    }

    #[test]
    fn integer_conversions_match_rust() {
        let seed = [
            i32::MIN as u32,
            u32::MAX,
            (-1i32) as u32,
            300,
            (-200i32) as u32,
            0,
            u32::MAX,
            u32::MAX,
            0,
            0x8000_0000,
        ];
        let program = [
            i2f(16, 0, 5, 0),
            i2f(17, 1, 4, 0),
            i2i(18, 2, 0),
            i2i(19, 3, 0),
            i2i(20, 4, 1),
            i2f_wide(21, 6, 2, 6),
            i2f_wide(22, 8, 2, 7),
            i2f_wide(24, 0, 3, 5),
        ];
        let Some((regs, _)) = run_regs(&program, &seed, &[], &[]) else {
            return;
        };
        assert_eq!(
            regs[16..21],
            [
                (i32::MIN as f32).to_bits(),
                (u32::MAX as f32).to_bits(),
                0,
                u8::MAX as u32,
                i8::MIN as i32 as u32
            ]
        );
        assert_eq!(regs[21], (u64::MAX as f32).to_bits());
        assert_eq!(regs[22], (i64::MIN as f32).to_bits());
        let f64_bits = i32::MIN as f64;
        let f64_bits = f64_bits.to_bits();
        assert_eq!(
            [regs[24], regs[25]],
            [f64_bits as u32, (f64_bits >> 32) as u32]
        );
    }

    #[test]
    fn f2f_f32_f64_round_trip_and_ftz() {
        let halfway = (1.0f64 + 2f64.powi(-24)).to_bits();
        let negative_halfway = (-1.0f64 - 2f64.powi(-24)).to_bits();
        let seed = [
            1.5f32.to_bits(),
            f32::from_bits(1).to_bits(),
            0,
            0,
            halfway as u32,
            (halfway >> 32) as u32,
            negative_halfway as u32,
            (negative_halfway >> 32) as u32,
        ];
        let program = [
            f2f(16, 0, 19, 0, false),
            f2f(18, 16, 26, 0, false),
            f2f(20, 1, 19, 0, true),
            f2f(22, 20, 26, 0, false),
            f2f(24, 4, 26, 0, false),
            f2f(25, 4, 26, 1, false),
            f2f(26, 4, 26, 2, false),
            f2f(27, 4, 26, 3, false),
            f2f(28, 6, 26, 1, false),
            f2f(29, 6, 26, 2, false),
        ];
        let Some((regs, _)) = run_regs(&program, &seed, &[], &[]) else {
            return;
        };
        assert_eq!(regs[18], 1.5f32.to_bits());
        assert_eq!(regs[22], 0);
        assert_eq!(
            regs[24..28],
            [
                1.0f32.to_bits(),
                1.0f32.to_bits(),
                1.0f32.to_bits() + 1,
                1.0f32.to_bits()
            ]
        );
        assert_eq!(regs[28..30], [(-1.0f32).to_bits() + 1, (-1.0f32).to_bits()]);
    }
}
