#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::{PRED, run_lanes};
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
    fn sts_u8_from_every_lane_leaves_one_lanes_byte() {
        let seeds = lanes(|i| vec![0, 0, 0, 0, i + 1]);
        let program = [inst::sts(255, 255), inst::bar_sync(), inst::sts(255, 4) & !inst::SZ_32, inst::bar_sync(), inst::lds(6, 255)];
        let Some((regs, _)) = run_lanes(&program, &seeds, &[], &[]) else {
            return;
        };
        assert!(regs.iter().all(|r| r[6] == regs[0][6] && (1..=LANES).contains(&r[6])), "{}", regs[0][6]);
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

    #[test]
    fn atoms_cast_compares_and_exchanges_32_and_64_bits() {
        let seeds = lanes(|i| vec![0, 0, 0, 0, 0, 0, i + 1]);
        let program = [inst::sts(255, 255), inst::bar_sync(), inst::atoms_cast(4, 255, 5, 6, false), inst::bar_sync(), inst::lds(7, 255)];
        let Some((regs, _)) = run_lanes(&program, &seeds, &[], &[]) else { return };
        assert_eq!(regs.iter().filter(|r| r[4] == 0).count(), 1);
        assert!(regs.iter().all(|r| (1..=LANES).contains(&r[7])));
        if !crate::gpu::device().is_ok_and(|d| d.shader_shared_int64_atomics) {
            eprintln!("skipping 64-bit CAST: no shaderSharedInt64Atomics");
            return;
        }
        let seeds = lanes(|i| {
            let mut seed = vec![0; 17];
            seed[8] = i + 1;
            seed[16] = 4;
            seed
        });
        let program = [inst::sts(255, 255), inst::sts(16, 255), inst::bar_sync(), inst::atoms_cast(4, 255, 6, 8, true), inst::bar_sync(), inst::lds(10, 255), inst::lds(11, 16)];
        let Some((regs, _)) = run_lanes(&program, &seeds, &[], &[]) else { return };
        assert_eq!(regs.iter().filter(|r| [r[4], r[5]] == [0, 0]).count(), 1);
        assert!(regs.iter().all(|r| (1..=LANES).contains(&r[10]) && r[11] == 0));
    }

    #[test]
    fn atoms_u64_add_returns_full_old_values() {
        if !crate::gpu::device().is_ok_and(|d| d.shader_shared_int64_atomics) {
            eprintln!("skipping: no shaderSharedInt64Atomics");
            return;
        }
        let seeds = lanes(|_| {
            let mut seed = vec![0; 17];
            seed[6] = 1;
            seed[16] = 4;
            seed
        });
        let program = [
            inst::sts(255, 255),
            inst::sts(16, 255),
            inst::bar_sync(),
            inst::W2J_ATOMS_U64_ADD,
            inst::bar_sync(),
            inst::lds(8, 255),
            inst::lds(9, 16),
        ];
        let Some((regs, _)) = run_lanes(&program, &seeds, &[], &[]) else {
            return;
        };
        let mut old = regs.iter().map(|r| (r[5] as u64) << 32 | r[4] as u64).collect::<Vec<_>>();
        old.sort_unstable();
        assert_eq!(old, (0..32).collect::<Vec<_>>());
        assert!(regs.iter().all(|r| [r[8], r[9]] == [32, 0]));
    }

    fn seeds(seed: impl Fn(u32) -> Vec<(usize, u32)>) -> Vec<Vec<u32>> {
        lanes(|lane| {
            let mut regs = vec![0; PRED + 1];
            for (slot, value) in seed(lane) {
                regs[slot] = value;
            }
            regs
        })
    }

    // Lanes 16..31 store on a detour after the BAR in pc order, so the scheduler reaches the BAR with lanes 0..15 first.
    #[test]
    fn bar_holds_early_lanes_until_the_late_writers_arrive() {
        let seeds = seeds(|lane| vec![(1, 4 * lane), (2, 100 + lane), (3, 4 * (lane ^ 16)), (PRED, (lane >= 16) as u32)]);
        let program = [
            inst::guard(inst::bra(0x40), 0, false),
            inst::sts(1, 2),
            inst::bar_sync(),
            inst::lds(4, 3),
            inst::exit(),
            inst::mov_imm(5, 1),
            inst::sts(1, 2),
            inst::bra(-0x60),
        ];
        let Some((regs, _)) = run_lanes(&program, &seeds, &[], &[]) else { return };
        assert_eq!(regs.iter().map(|r| r[4]).collect::<Vec<_>>(), (0..LANES).map(|lane| 100 + (lane ^ 16)).collect::<Vec<_>>());
    }

    #[test]
    fn consecutive_bars_wait_for_each_generation_of_writers() {
        let seeds = seeds(|lane| vec![(1, 4 * lane), (2, 100 + lane), (3, 4 * (lane ^ 16)), (PRED, (lane >= 16) as u32)]);
        let program = [
            inst::guard(inst::bra(0x60), 0, false),
            inst::bar_sync(),
            inst::lds(4, 3),
            inst::guard(inst::bra(0x50), 0, true),
            inst::bar_sync(),
            inst::lds(5, 3),
            inst::exit(),
            inst::sts(1, 2),
            inst::bra(-0x80),
            inst::iadd_imm(6, 2, 1000),
            inst::sts(1, 6),
            inst::bra(-0x80),
        ];
        let Some((regs, _)) = run_lanes(&program, &seeds, &[], &[]) else { return };
        for (lane, r) in (0..LANES).zip(&regs) {
            let read = if lane < 16 { r[4] } else { r[5] - 1000 };
            assert_eq!(read, 100 + (lane ^ 16), "lane {lane}");
        }
    }

    #[test]
    fn bar_red_reduces_over_arriving_lanes_only() {
        let seeds = seeds(|lane| vec![(PRED, (lane != 31) as u32 | ((lane == 31) as u32) << 1)]);
        let program = [
            inst::guard(inst::exit(), 1, false),
            inst::bar(inst::BAR_RED_AND, 0),
            inst::b2r_result(5, 2),
            inst::bar(inst::BAR_RED_POPC, 0),
            inst::b2r_result(6, 3),
        ];
        let Some((regs, _)) = run_lanes(&program, &seeds, &[], &[]) else { return };
        for r in &regs[..31] {
            assert_eq!([r[5], r[6], r[PRED] >> 2 & 3], [1, 31, 3]);
        }
    }

    #[test]
    fn bar_short_of_its_thread_count_reports_the_stuck_lanes() {
        let seeds = seeds(|lane| vec![(PRED, ((lane == 31) as u32) << 1)]);
        let program = [inst::guard(inst::exit(), 1, false), inst::bar(0, LANES), inst::exit()];
        let run = std::panic::catch_unwind(|| run_lanes(&program, &seeds, &[], &[]));
        assert!(!matches!(run, Ok(Some(_))), "31 arrivals released a BAR waiting for 32");
    }
}
