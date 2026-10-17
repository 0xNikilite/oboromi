#[cfg(test)]
mod tests {
    use crate::gpu::device;
    use crate::tests::gpu::harness::{GLOBAL_BASE, UREG, run_hints, run_lanes, run_regs, run_timed};
    use crate::tests::gpu_test::inst::{W2I_NANOSLEEP, W2I_PMTRIG, cs2r, s2r, s2ur, stg};

    #[test]
    fn special_registers_write_their_destination_width() {
        let seed = vec![u32::MAX; UREG + 6];
        let program = [s2r(0, 33), s2r(1, 39), s2r(2, 0), cs2r(4, 80, true), cs2r(6, 81, false), s2ur(5, 37)];
        let Some((regs, _)) = run_regs(&program, &seed, &[], &[]) else {
            return;
        };
        assert_eq!(regs[..8], [0, 0, 0, u32::MAX, 0, 0, 0, u32::MAX]);
        assert_eq!(regs[UREG + 5], 0);
    }

    #[test]
    fn lanes_keep_own_registers_and_share_global_memory() {
        let seeds: Vec<Vec<u32>> = (0..32u64)
            .map(|i| {
                let address = GLOBAL_BASE + 4 * i;
                vec![0, 0, address as u32, (address >> 32) as u32]
            })
            .collect();
        let program = [s2r(0, 33), stg(2, 0, 0)];
        let Some((lanes, global)) = run_lanes(&program, &seeds, &[], &[u32::MAX; 32]) else {
            return;
        };
        let ids: Vec<u32> = (0..32).collect();
        assert_eq!(lanes.iter().map(|regs| regs[0]).collect::<Vec<_>>(), ids);
        assert_eq!(global, ids);
    }

    #[test]
    fn nanosleep_timing_hint_waits_only_when_enabled() {
        let Ok(state) = device() else { return };
        if !state.shader_device_clock {
            return;
        }
        let sleep = W2I_NANOSLEEP | (1_000_000u128 << 32);
        let program = [sleep; 64];
        let mut off = (0..5).filter_map(|_| run_timed(&state, &program, false)).collect::<Vec<_>>();
        let mut on = (0..5).filter_map(|_| run_timed(&state, &program, true)).collect::<Vec<_>>();
        if off.iter().copied().max().unwrap_or_default() > 10_000_000 {
            eprintln!("skipping nanosleep timing test: disabled run starved");
            return;
        }
        off.sort_unstable();
        on.sort_unstable();
        let (off, on) = (off[off.len() / 2], on[on.len() / 2]);
        assert!(on >= off.saturating_mul(3) / 2, "off {off}ns, on {on}ns");
    }

    #[test]
    fn pmtrig_counts_triggers_by_immediate() {
        let trigger = W2I_PMTRIG | (17u128 << 32);
        let sleep = W2I_NANOSLEEP | (8u128 << 32);
        let Some((_, _, counts)) = run_hints(&[sleep, trigger, trigger], &[], &[], &[]) else {
            return;
        };
        assert_eq!(counts[17], 2);
        assert_eq!(counts.iter().sum::<u32>(), 2);
    }
}
