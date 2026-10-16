#[cfg(test)]
mod tests {
    use crate::gpu::sm86::STATE_WORDS;
    use crate::tests::gpu::harness::{
        GLOBAL_BASE, run_lanes, run_regs, spirv_val, translate_module,
    };
    use crate::tests::gpu_test::inst;

    fn s2r(rd: u32, sr: u32) -> u128 {
        0x7919 | (rd as u128) << 16 | (sr as u128) << 72
    }

    fn cs2r(rd: u32, sr: u32, pair: bool) -> u128 {
        0x7805 | (rd as u128) << 16 | (sr as u128) << 72 | (pair as u128) << 80
    }

    fn s2ur(urd: u32, sr: u32) -> u128 {
        0x79c3 | (urd as u128) << 16 | (sr as u128) << 72
    }

    fn predicated(word: u128, pred: u32, invert: bool) -> u128 {
        word & !(0xf << 12) | (pred as u128) << 12 | (invert as u128) << 15
    }

    #[test]
    fn special_registers_read_compute_ids_and_zero_stubs() {
        let ur5 = 254 + 1 + 5;
        let mut seed = vec![0; ur5 + 1];
        seed[..15].fill(u32::MAX);
        seed[ur5] = u32::MAX;
        let program = [
            s2r(0, 33),
            s2r(1, 34),
            s2r(2, 35),
            s2r(3, 37),
            s2r(4, 38),
            s2r(5, 39),
            s2r(6, 0),
            s2r(7, 3),
            cs2r(8, 80, true),
            s2r(10, 55),
            cs2r(12, 32, true),
            cs2r(14, 81, false),
            s2ur(5, 37),
        ];
        let Some((regs, _)) = run_regs(&program, &seed, &[], &[]) else {
            return;
        };
        let mut expected = [0; 15];
        expected[11] = u32::MAX;
        assert_eq!(regs[..15], expected);
        assert_eq!(regs[ur5], 0);
    }

    #[test]
    fn lanes_keep_own_registers_and_share_global_memory() {
        let seeds: Vec<Vec<u32>> = (0..32u64)
            .map(|i| {
                let address = GLOBAL_BASE + 4 * i;
                vec![0, 0, address as u32, (address >> 32) as u32]
            })
            .collect();
        let program = [s2r(0, 33), inst::stg(2, 0, 0)];
        let Some((lanes, global)) = run_lanes(&program, &seeds, &[], &[u32::MAX; 32]) else {
            return;
        };
        let ids: Vec<u32> = (0..32).collect();
        assert_eq!(lanes.iter().map(|regs| regs[0]).collect::<Vec<_>>(), ids);
        assert_eq!(global, ids);
    }

    #[test]
    fn special_register_builtins_are_entry_point_inputs() {
        let module = translate_module(&[s2r(0, 33), s2r(1, 37), s2r(2, 0)], false, None);
        spirv_val(&module);
        let path = std::env::temp_dir().join(format!("oboromi-special-{}.spv", std::process::id()));
        std::fs::write(&path, module.to_bytes()).unwrap();
        let output = match std::process::Command::new("spirv-dis").arg(&path).output() {
            Ok(output) => output,
            Err(e) => {
                eprintln!("skipping, spirv-dis unavailable: {e}");
                return;
            }
        };
        std::fs::remove_file(path).unwrap();
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        let entry = text
            .lines()
            .find(|line| line.contains("OpEntryPoint"))
            .unwrap();
        for builtin in ["LocalInvocationId", "WorkgroupId", "LocalInvocationIndex"] {
            let line = text
                .lines()
                .find(|line| line.contains(&format!("BuiltIn {builtin}")))
                .unwrap();
            let var = line.split_whitespace().nth(1).unwrap();
            assert!(
                entry.split_whitespace().any(|word| word == var),
                "{builtin}"
            );
        }
    }

    #[test]
    fn special_register_reads_obey_predicates() {
        let ur5 = 254 + 1 + 5;
        let ur6 = ur5 + 1;
        let mut seed = vec![0; STATE_WORDS];
        seed[0..3].fill(u32::MAX);
        seed[ur5..=ur6].fill(u32::MAX);
        let program = [
            predicated(s2r(0, 80), 0, false),
            predicated(s2r(1, 80), 7, true),
            predicated(s2r(2, 80), 0, true),
            predicated(s2ur(5, 37), 0, false),
            predicated(s2ur(6, 37), 0, true),
        ];
        let Some((regs, _)) = run_regs(&program, &seed, &[], &[]) else {
            return;
        };
        assert_eq!(regs[0..3], [u32::MAX, u32::MAX, 0]);
        assert_eq!([regs[ur5], regs[ur6]], [u32::MAX, 0]);
    }

    #[test]
    #[should_panic(expected = "unknown special register 200")]
    fn unknown_special_register_panics_with_index() {
        translate_module(&[s2r(0, 200)], false, None);
    }

    #[test]
    fn special_register_words_match_nvdisasm() {
        let fixtures = [
            (s2r(1, 33), "S2R R1, SR_TID.X ;"),
            (s2r(2, 37), "S2R R2, SR_CTAID.X ;"),
            (s2r(3, 0), "S2R R3, SR_LANEID ;"),
            (cs2r(4, 80, false), "CS2R.32 R4, SR_CLOCKLO;"),
            (cs2r(4, 80, true), "CS2R R4, SR_CLOCKLO;"),
            (s2ur(5, 37), "S2UR UR5, SR_CTAID.X ;"),
        ];
        let path = std::env::temp_dir().join(format!("oboromi-special-{}.bin", std::process::id()));
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
            assert_eq!(disasm.trim(), expected);
        }
        std::fs::remove_file(path).unwrap();
    }
}
