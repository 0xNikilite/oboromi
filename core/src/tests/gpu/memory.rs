#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::{GLOBAL_BASE, run_regs};
    use crate::tests::gpu_test::inst;

    const BASE: [u32; 2] = [GLOBAL_BASE as u32, (GLOBAL_BASE >> 32) as u32];
    const STL_32: u128 = 0x0001c000001008000000080100007387;
    const LDL_32: u128 = 0x00000000001008000000080000047983;
    const STL_U8: u128 = 0x0001c000001000000000000100007387;
    const LDL_S8: u128 = 0x00000000001002000000000000057983;
    const STL_U16: u128 = 0x0001c000001004000000040100007387;
    const LDL_S16: u128 = 0x00000000001006000000040000067983;
    const ST_32: u128 = 0x0001c000001009040000000000007385;
    const LD_32: u128 = 0x00000000001009000000000000057980;
    const LDG_U8: u128 = 0x00000000001e01000000010002047381;
    const LDG_S16: u128 = 0x00000000001e07000000020002057381;
    const LDG_UR: u128 = 0x000000000c1e09000000000402067981;

    #[test]
    fn ldg_reads_global_word() {
        let seed = [0, 0, BASE[0], BASE[1]];
        let Some((regs, _)) = run_regs(&[inst::ldg(4, 2, 8)], &seed, &[], &[1, 2, 0xdead_beef])
        else {
            return;
        };
        assert_eq!(regs[4], 0xdead_beef);
    }

    #[test]
    fn stg_writes_global_word() {
        let seed = [0, 0, BASE[0], BASE[1], 0, 0x1234];
        let Some((_, global)) = run_regs(&[inst::stg(2, 0x10, 5)], &seed, &[], &[0; 5]) else {
            return;
        };
        assert_eq!(global, [0, 0, 0, 0, 0x1234]);
    }

    #[test]
    fn ldc_and_uldc_read_cbuf() {
        let program = [inst::ldc(6, 0, 3, 4), inst::uldc64(4, 0, 0x10)];
        let Some((regs, _)) = run_regs(&program, &[0, 0, 0, 8], &[10, 11, 12, 13, 14, 15], &[])
        else {
            return;
        };
        let ur4 = 254 + 1 + 4;
        assert_eq!((regs[6], &regs[ur4..ur4 + 2]), (13, &[14, 15][..]));
    }

    #[test]
    fn local_round_trip_and_narrow_sign_extension() {
        let program = [STL_32, LDL_32, STL_U8, LDL_S8, STL_U16, LDL_S16];
        let Some((regs, _)) = run_regs(&program, &[0, 0x1234_8080], &[], &[]) else {
            return;
        };
        assert_eq!((regs[4], regs[5], regs[6]), (0x1234_8080, 0xffff_ff80, 0xffff_8080));
    }

    #[test]
    fn generic_load_store_use_global_memory() {
        let seed = [BASE[0], BASE[1], 0, 0, 0x1234_5678];
        let Some((regs, global)) = run_regs(&[ST_32, LD_32], &seed, &[], &[0]) else {
            return;
        };
        assert_eq!((regs[5], global[0]), (0x1234_5678, 0x1234_5678));
    }

    #[test]
    fn ldg_narrow_loads_extend() {
        let seed = [0, 0, BASE[0], BASE[1]];
        let Some((regs, _)) = run_regs(&[LDG_U8, LDG_S16], &seed, &[], &[0x8001_ff80]) else {
            return;
        };
        assert_eq!((regs[4], regs[5]), (0xff, 0xffff_8001));
    }

    #[test]
    fn ldg_adds_uniform_address_register() {
        let mut seed = vec![0; 254 + 1 + 5];
        seed[2..4].copy_from_slice(&BASE);
        seed[254 + 1 + 4] = 8;
        let Some((regs, _)) = run_regs(&[LDG_UR], &seed, &[], &[0, 0, 0xdead_beef]) else {
            return;
        };
        assert_eq!(regs[6], 0xdead_beef);
    }

    #[test]
    fn memory_words_match_nvdisasm() {
        let fixtures = [
            (STL_32, "STL [R0+0x8], R1 ;"),
            (LDL_32, "LDL R4, [R0+0x8] ;"),
            (STL_U8, "STL.U8 [R0], R1 ;"),
            (LDL_S8, "LDL.S8 R5, [R0] ;"),
            (STL_U16, "STL.U16 [R0+0x4], R1 ;"),
            (LDL_S16, "LDL.S16 R6, [R0+0x4] ;"),
            (ST_32, "ST.E [R0], R4 ;"),
            (LD_32, "LD.E R5, [R0] ;"),
            (LDG_U8, "LDG.E.U8 R4, [R2+0x1] ;"),
            (LDG_S16, "LDG.E.S16 R5, [R2+0x2] ;"),
            (LDG_UR, "LDG.E R6, [R2.64+UR4] ;"),
        ];
        let bin = std::env::temp_dir().join(format!("oboromi-memory-{}.bin", std::process::id()));
        for (word, expected) in fixtures {
            std::fs::write(&bin, word.to_le_bytes()).unwrap();
            let out = std::process::Command::new("/usr/local/cuda-13.1/bin/nvdisasm")
                .args(["--binary", "SM86"])
                .arg(&bin)
                .output()
                .unwrap();
            let text = String::from_utf8_lossy(&out.stdout);
            let disasm = text.lines().find_map(|line| line.split("/*0000*/").nth(1)).unwrap_or_default();
            assert_eq!(disasm.trim(), expected, "{word:#x}");
        }
        std::fs::remove_file(bin).unwrap();
    }
}
