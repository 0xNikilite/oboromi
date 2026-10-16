#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::{GLOBAL_BASE, UREG, run_lanes};
    use crate::tests::gpu_test::inst::{ATOM, ATOM_DEC, ATOM_INC, ATOM_MAX, ATOM_MIN, ATOMG, ATOMG_CAS, RED, RED_UR_64, S32, alu, alu3};

    fn run(program: &[u128], r5: impl Fn(u32) -> u32, r6: impl Fn(u32) -> u32, global: &[u32]) -> Option<(Vec<Vec<u32>>, Vec<u32>)> {
        let next = GLOBAL_BASE + 4;
        let seeds: Vec<Vec<u32>> = (0..32)
            .map(|lane| {
                let mut seed = vec![0; UREG + 5];
                seed[2..10].copy_from_slice(&[GLOBAL_BASE as u32, (GLOBAL_BASE >> 32) as u32, 0, r5(lane), r6(lane), 0, next as u32, (next >> 32) as u32]);
                seed[UREG + 4] = 8;
                seed
            })
            .collect();
        run_lanes(program, &seeds, &[], global)
    }

    fn sorted(lanes: &[Vec<u32>], reg: usize) -> Vec<u32> {
        let mut values: Vec<u32> = lanes.iter().map(|regs| regs[reg]).collect();
        values.sort_unstable();
        values
    }

    #[test]
    fn atom_add_returns_each_old_value_once() {
        let Some((lanes, global)) = run(&[alu(ATOM, 4, 2, 5)], |_| 1, |_| 0, &[0]) else {
            return;
        };
        assert_eq!((global[0], sorted(&lanes, 4)), (32, (0..32).collect()));
    }

    #[test]
    fn atomg_max_is_unsigned_and_min_s32_is_signed() {
        let program = [alu(ATOMG | ATOM_MAX, 4, 2, 5), alu(ATOMG | ATOM_MIN | S32, 6, 8, 5)];
        let Some((_, global)) = run(&program, |lane| lane.wrapping_sub(16), |_| 0, &[0, i32::MAX as u32]) else {
            return;
        };
        assert_eq!(global, [u32::MAX, -16i32 as u32]);
    }

    #[test]
    fn atomg_cas_has_one_winner() {
        let Some((lanes, global)) = run(&[alu3(ATOMG_CAS, 4, 2, 5, 6)], |_| 0, |lane| lane + 1, &[0]) else {
            return;
        };
        let winners: Vec<usize> = (0..32).filter(|&lane| lanes[lane][4] == 0).collect();
        assert_eq!(winners, [global[0] as usize - 1]);
        assert!(lanes.iter().all(|regs| regs[4] == 0 || regs[4] == global[0]));
    }

    #[test]
    fn red_add_through_uniform_address() {
        let Some((_, global)) = run(&[alu3(RED | RED_UR_64, 0, 2, 5, 4)], |_| 1, |_| 0, &[0; 3]) else {
            return;
        };
        assert_eq!(global, [0, 0, 32]);
    }

    #[test]
    fn inc_and_dec_wrap_at_the_bound() {
        let program = [alu(ATOMG | ATOM_INC, 4, 2, 5), alu(ATOMG | ATOM_DEC, 6, 8, 5)];
        let Some((lanes, global)) = run(&program, |_| 9, |_| 0, &[0, 0]) else {
            return;
        };
        let mut incs: Vec<u32> = (0..32).map(|i| i % 10).collect();
        let mut decs: Vec<u32> = (0..32).map(|i| (10 - i % 10) % 10).collect();
        incs.sort_unstable();
        decs.sort_unstable();
        assert_eq!((global, sorted(&lanes, 4), sorted(&lanes, 6)), (vec![2, 8], incs, decs));
    }
}
