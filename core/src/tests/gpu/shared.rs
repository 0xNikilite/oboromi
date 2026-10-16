#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::run_lanes;
    use crate::tests::gpu_test::inst;

    const LANES: u32 = 32;

    fn lanes(seed: impl Fn(u32) -> Vec<u32>) -> Vec<Vec<u32>> {
        (0..LANES).map(seed).collect()
    }

    // sm86-0050-lds-lane-address from the warp rig: @P0 LDS R4, [R15], silicon read back lane i and i + 1.
    const SILICON_LDS_LANE_ADDRESS: u128 = 0x004fc80000000800000000000f040984;

    #[test]
    fn sts_bar_lds_reads_neighbour_lane() {
        let seeds = lanes(|i| {
            let mut seed = vec![0; 255];
            (seed[4], seed[15], seed[16], seed[254]) = (i, 4 * i, 4 * ((i + 1) % LANES), 1);
            seed
        });
        let program = [inst::sts(15, 4), inst::bar_sync(), SILICON_LDS_LANE_ADDRESS, inst::lds(13, 16)];
        let Some((regs, _)) = run_lanes(&program, &seeds, &[], &[]) else {
            return;
        };
        let read: Vec<[u32; 2]> = regs.iter().map(|r| [r[4], r[13]]).collect();
        assert_eq!(read, (0..LANES).map(|i| [i, (i + 1) % LANES]).collect::<Vec<_>>());
    }

    #[test]
    fn atoms_add_hands_each_lane_a_distinct_old_value() {
        let seeds = lanes(|_| vec![0, 0, 0, 0, 0, 1]);
        let program = [
            inst::sts(255, 255),
            inst::bar_sync(),
            inst::atoms_add(4, 255, 5),
            inst::bar_sync(),
            inst::lds(6, 255),
        ];
        let Some((regs, _)) = run_lanes(&program, &seeds, &[], &[]) else {
            return;
        };
        let mut old: Vec<u32> = regs.iter().map(|r| r[4]).collect();
        old.sort_unstable();
        assert_eq!(old, (0..LANES).collect::<Vec<_>>());
        assert!(regs.iter().all(|r| r[6] == LANES));
    }

    #[test]
    fn atoms_cas_lets_exactly_one_lane_win() {
        let seeds = lanes(|i| vec![0, 0, 0, 0, 0, 0, 0, 0, 0, i + 1]);
        let program = [
            inst::sts(255, 255),
            inst::bar_sync(),
            inst::atoms_cas(7, 255, 8, 9),
            inst::bar_sync(),
            inst::lds(6, 255),
        ];
        let Some((regs, _)) = run_lanes(&program, &seeds, &[], &[]) else {
            return;
        };
        let winners: Vec<&Vec<u32>> = regs.iter().filter(|r| r[7] == 0).collect();
        assert_eq!(winners.len(), 1);
        let stored = winners[0][9];
        assert!(regs.iter().all(|r| r[6] == stored && (r[7] == 0 || r[7] == stored)));
    }
}
