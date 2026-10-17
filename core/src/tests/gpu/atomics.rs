#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::{GLOBAL_BASE, UREG, run_lanes};
    use crate::tests::gpu_test::inst::{ATOM, ATOM_DEC, ATOM_INC, ATOM_MAX, ATOM_MIN, ATOMG, ATOMG_CAS, ATOM_ADD_F16X2, ATOM_ADD_F32, ATOM_MAX_F16X2, ATOM_MIN_F16X2, RED, RED_UR_64, S32, SZ_32, W2J_ATOMG_U64_ADD, alu, alu3, stg};

    fn run(program: &[u128], r5: impl Fn(u32) -> u32, r6: impl Fn(u32) -> u32, global: &[u32]) -> Option<(Vec<Vec<u32>>, Vec<u32>)> {
        let next = GLOBAL_BASE + 4;
        let seeds: Vec<Vec<u32>> = (0..32)
            .map(|lane| {
                let byte = next + lane as u64;
                let mut seed = vec![0; UREG + 5];
                seed[2..12].copy_from_slice(&[GLOBAL_BASE as u32, (GLOBAL_BASE >> 32) as u32, 0, r5(lane), r6(lane), 0, next as u32, (next >> 32) as u32, byte as u32, (byte >> 32) as u32]);
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
    fn atom_add_returns_each_old_value_once_and_byte_stores_do_not_race() {
        let program = [alu(ATOM, 4, 2, 5), stg(10, 0, 6) ^ SZ_32];
        let Some((lanes, global)) = run(&program, |_| 1, |lane| lane, &[0; 9]) else {
            return;
        };
        let bytes: Vec<u8> = global[1..].iter().flat_map(|word| word.to_le_bytes()).collect();
        assert_eq!((global[0], sorted(&lanes, 4), bytes), (32, (0..32).collect(), (0..32).collect()));
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
    fn atom_float_add_and_half2_operations_are_atomic() {
        let Some((_, global)) = run(&[alu(ATOM_ADD_F32, 4, 2, 5)], |_| 1.0f32.to_bits(), |_| 0, &[0]) else { return };
        assert_eq!(global[0], 32.0f32.to_bits());
        if !crate::gpu::device().is_ok_and(|d| d.shader_float16) {
            eprintln!("skipping half2 atomics: no shaderFloat16");
            return;
        }
        let program = [
            alu(ATOM_MIN_F16X2, 4, 2, 5),
            alu(ATOM_MAX_F16X2 | 4 << 40, 12, 2, 5),
            alu(ATOM_ADD_F16X2 | 8 << 40, 8, 2, 6),
        ];
        let Some((_, global)) = run(&program, |_| 0xc500_4500, |_| 0x3c00_3c00, &[0xc900_4900, 0xc900_4900, 0]) else { return };
        assert_eq!(global, [0xc900_4500, 0xc500_4900, 0x5000_5000]);
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

    #[test]
    fn atomg_u64_add_returns_full_old_values() {
        let seeds = (0..32).map(|_| {
            let mut seed = vec![0; UREG + 1];
            seed[2] = GLOBAL_BASE as u32;
            seed[3] = (GLOBAL_BASE >> 32) as u32;
            seed[6] = 1;
            seed
        }).collect::<Vec<_>>();
        let Some((lanes, global)) = run_lanes(&[W2J_ATOMG_U64_ADD], &seeds, &[], &[0, 0]) else {
            return;
        };
        let mut old = lanes.iter().map(|regs| (regs[5] as u64) << 32 | regs[4] as u64).collect::<Vec<_>>();
        old.sort_unstable();
        assert_eq!((old, global), ((0..32).collect(), vec![32, 0]));
    }
}
