#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::{PRED, UPRED, UREG, run_lanes, run_regs};
    use crate::tests::gpu_test::inst::*;

    const VOTE_ANY_R4_PT_P0: u128 = 0x004fc800000e01000000000000040806;

    // The A6000 warp rig seeds lane i with R4..R7 = i, 4i, 0, 256 + i and P0 = bit i of the lane mask.
    fn rig_seeds(p0: u32) -> Vec<Vec<u32>> {
        (0..32).map(|i| seeds_with(&[(4, i), (5, 4 * i), (7, 256 + i), (PRED, p0 >> i & 1)])).collect()
    }

    fn seeds_with(slots: &[(usize, u32)]) -> Vec<u32> {
        let mut words = vec![0; UPRED + 1];
        for &(slot, value) in slots {
            words[slot] = value;
        }
        words
    }

    fn run(program: &[u128], seeds: &[Vec<u32>]) -> Option<Vec<Vec<u32>>> {
        run_lanes(program, seeds, &[], &[]).map(|(lanes, _)| lanes)
    }

    fn column(lanes: &[Vec<u32>], slot: usize) -> Vec<u32> {
        lanes.iter().map(|regs| regs[slot]).collect()
    }

    #[test]
    fn vote_ballots_pp_by_lane_and_sets_mode_result() {
        for mask in [u32::MAX, 0x5555_5555, 0x8000_0000] {
            let Some(lanes) = run(&[VOTE_ANY_R4_PT_P0], &rig_seeds(mask)) else { return };
            for (lane, r4) in (0..32).zip(column(&lanes, 4)) {
                assert_eq!(r4, if mask >> lane & 1 != 0 { mask } else { lane }, "lane {lane}");
            }
        }
        let program = [
            alu(VOTE | preds(1, 0, 0, 0), 8, 0, 0),
            alu(VOTE | VOTE_ANY | preds(2, 0, 0, 0), 9, 0, 0),
            alu(VOTE | VOTE_EQ | preds(3, 0, 0, 0), 10, 0, 0),
        ];
        for (mask, all_any_eq) in [(0, 0b100), (0x5555_5555, 0b010), (u32::MAX, 0b111)] {
            let Some(lanes) = run(&program, &rig_seeds(mask)) else { return };
            for regs in lanes {
                assert_eq!(regs[8..11], [mask; 3]);
                assert_eq!(regs[PRED] >> 1 & 7, all_any_eq, "{mask:#x}");
            }
        }
    }

    #[test]
    fn a_single_seeded_lane_votes_alone() {
        let Some((regs, _)) = run_regs(&[VOTE_ANY_R4_PT_P0], &seeds_with(&[(PRED, 1)]), &[], &[]) else { return };
        assert_eq!(regs[4], 1);
    }

    #[test]
    fn voteu_writes_ballot_and_result_to_uniform_registers() {
        let program = [alu(VOTEU | VOTE_EQ | preds(2, 0, 0, 0), 5, 0, 0), alu(VOTEU | VOTE_ANY | preds(3, 0, 0, 0), 6, 0, 0)];
        let Some(lanes) = run(&program, &rig_seeds(0x8000_0003)) else { return };
        for regs in lanes {
            assert_eq!(regs[UREG + 5..UREG + 7], [0x8000_0003; 2]);
            assert_eq!(regs[UPRED] >> 2 & 3, 0b10);
        }
    }

    #[test]
    fn shfl_matches_silicon_and_clamps_to_segments() {
        let fixtures: [(u128, [u32; 32]); 4] = [
            (0x004fc800000e000004e00f0005040f89, [0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44, 48, 52, 56, 60, 64, 68, 72, 76, 80, 84, 60, 64, 68, 72, 76, 80, 84, 88, 92, 96]),
            (0x004fc800000e000008e00f0005040f89, [28, 32, 36, 40, 44, 48, 52, 56, 60, 36, 40, 44, 48, 52, 56, 60, 64, 68, 72, 76, 80, 84, 88, 92, 96, 100, 104, 108, 112, 116, 120, 124]),
            (0x004fc800000e00000ce01f0005040f89, [28, 24, 20, 16, 12, 8, 4, 0, 60, 56, 52, 48, 44, 40, 36, 32, 92, 88, 84, 80, 76, 72, 68, 64, 124, 120, 116, 112, 108, 104, 100, 96]),
            (0x004fc800000e000000e01f0005040f89, [28; 32]),
        ];
        for (word, r4) in fixtures {
            let Some(lanes) = run(&[word], &rig_seeds(u32::MAX)) else { return };
            assert_eq!(column(&lanes, 4), r4, "{word:#x}");
        }
        // Width 8 as CUDA encodes it: IDX takes c = 0x181f and UP c = 0x1800, both from registers here.
        let program = [alu(SHFL_RI | shfl_lanes(0, 0x181f) | preds(1, 0, 0, 0), 8, 5, 6), alu3(SHFL_RR | SHFL_UP | preds(2, 0, 0, 0), 9, 5, 7, 12)];
        let seeds: Vec<Vec<u32>> = (0..32).map(|i| seeds_with(&[(5, 4 * i), (6, 3), (7, 1), (12, 0x1800)])).collect();
        let Some(lanes) = run(&program, &seeds) else { return };
        for (lane, regs) in (0..32).zip(lanes) {
            let up = if lane % 8 == 0 { lane } else { lane - 1 };
            assert_eq!(regs[8..10], [4 * (lane & !7 | 3), 4 * up], "lane {lane}");
            assert_eq!(regs[PRED] >> 1 & 3, 1 | ((lane % 8 != 0) as u32) << 1, "lane {lane}");
        }
    }

    #[test]
    fn shfl_from_an_inactive_lane_reads_zero_out_of_range() {
        let word = guard(alu(SHFL_II | SHFL_BFLY | shfl_lanes(16, 0x1f) | preds(1, 0, 0, 0), 7, 5, 0), 0, false);
        let mask = 0xffff_00ff;
        let Some(lanes) = run(&[word], &rig_seeds(mask)) else { return };
        let active = |lane: u32| mask >> lane & 1 != 0;
        for (lane, regs) in (0..32).zip(lanes) {
            let expected = match (active(lane), active(lane ^ 16)) {
                (false, _) => [256 + lane, 0],
                (true, true) => [4 * (lane ^ 16), 1],
                (true, false) => [0, 0],
            };
            assert_eq!([regs[7], regs[PRED] >> 1 & 1], expected, "lane {lane}");
        }
    }

    // The rig cannot observe REDUX's UR, so its fixture only shows R4 kept.
    #[test]
    fn redux_reduces_the_warp_into_a_uniform_register() {
        const REDUX_SUM_UR0_R4: u128 = 0x004fc8000000c00000000000040003c4;
        let Some(lanes) = run(&[REDUX_SUM_UR0_R4], &rig_seeds(u32::MAX)) else { return };
        assert_eq!(column(&lanes, 4), (0..32).collect::<Vec<_>>());
        assert_eq!(column(&lanes, UREG), [496; 32]);
        let values: Vec<u32> = (0..32u32).map(|i| i.wrapping_mul(0x9e37_79b9) ^ i << 3).collect();
        let program: Vec<u128> = (0..6).map(|op| alu(REDUX | redux_op(op), op, 4, 0)).chain([4, 5].map(|op| alu(REDUX | redux_op(op) | S32, op + 2, 4, 0))).collect();
        let seeds: Vec<Vec<u32>> = values.iter().map(|&v| seeds_with(&[(4, v)])).collect();
        let Some(lanes) = run(&program, &seeds) else { return };
        let fold = |f: fn(u32, u32) -> u32| values.iter().copied().reduce(f).unwrap();
        let signed = values.iter().map(|&v| v as i32);
        let expected = [
            fold(|a, b| a & b),
            fold(|a, b| a | b),
            fold(|a, b| a ^ b),
            fold(u32::wrapping_add),
            fold(u32::min),
            fold(u32::max),
            signed.clone().min().unwrap() as u32,
            signed.max().unwrap() as u32,
        ];
        for regs in lanes {
            assert_eq!(regs[UREG..UREG + 8], expected);
        }
    }

    #[test]
    fn match_masks_lanes_holding_the_same_value() {
        const MATCH_ALL_PT_R4_R5: u128 = 0x004fc800000e000000000000050403a1;
        let Some(lanes) = run(&[MATCH_ALL_PT_R4_R5], &rig_seeds(u32::MAX)) else { return };
        assert_eq!(column(&lanes, 4), [0; 32]);
        let program = [
            alu(MATCH | MATCH_ANY, 8, 6, 0),
            alu(MATCH | preds(1, 0, 0, 0), 9, 7, 0),
            alu(MATCH | MATCH_ANY | MATCH_U64, 10, 12, 0),
        ];
        let seeds: Vec<Vec<u32>> = (0..32).map(|i| seeds_with(&[(6, i / 4), (7, 5), (12, i & 1), (13, i >> 4)])).collect();
        let Some(lanes) = run(&program, &seeds) else { return };
        for (lane, regs) in (0..32).zip(lanes) {
            let pair = 0x5555 << (lane & 1) << (lane & 16);
            assert_eq!(regs[8..11], [0xf << (lane & !3), u32::MAX, pair], "lane {lane}");
            assert_eq!(regs[PRED] >> 1 & 1, 1);
        }
    }

    // NAK lowers ddx to SHFL.BFLY 1 then FSWZADD with SUB on even lanes and SUBR on odd ones (PNNPPNNP).
    #[test]
    fn fswzadd_picks_its_op_by_lane_in_quad() {
        let program = [
            alu(SHFL_II | SHFL_BFLY | shfl_lanes(1, 0x1c03) | preds(7, 0, 0, 0), 5, 4, 0),
            alu3(FSWZADD, 6, 5, 0x66, 4),
            alu3(FSWZADD, 7, 4, 0x1b, 8),
        ];
        let x = |lane: u32| (lane * lane) as f32;
        let seeds: Vec<Vec<u32>> = (0..32).map(|i| seeds_with(&[(4, x(i).to_bits()), (8, 100f32.to_bits())])).collect();
        let Some(lanes) = run(&program, &seeds) else { return };
        for (lane, regs) in (0..32).zip(lanes) {
            let (a, c) = (x(lane), 100.0);
            let by_op = [a + c, a - c, c - a, c][lane as usize % 4];
            assert_eq!([f32::from_bits(regs[6]), f32::from_bits(regs[7])], [x(lane | 1) - x(lane & !1), by_op], "lane {lane}");
        }
    }

    #[test]
    fn lane_masks_match_silicon_eqmask() {
        const S2R_R4_SR_EQMASK: u128 = 0x004fc800000038000000000000040919;
        let program = [S2R_R4_SR_EQMASK, s2r(8, 57), s2r(9, 58), s2r(10, 59), s2r(11, 60)];
        let Some(lanes) = run(&program, &rig_seeds(u32::MAX)) else { return };
        for (lane, regs) in (0..32).zip(lanes) {
            let eq = 1u32 << lane;
            assert_eq!([regs[4], regs[8], regs[9], regs[10], regs[11]], [eq, eq - 1, eq | (eq - 1), !(eq | (eq - 1)), !(eq - 1)]);
        }
    }
}
