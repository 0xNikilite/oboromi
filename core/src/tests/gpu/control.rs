#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::{PRED, UPRED, UREG, run_regs};
    use crate::tests::gpu_test::inst::{
        IMM_B, UIADD3, alu3, bra, bra_conv, bra_div, bra_upred, brx, bssy, bsync, call_rel, exit, guard, iadd_imm, jmx, kill, mov_imm, preds, ret_rel,
        uldc64,
    };

    #[test]
    fn predicate_guards_mov() {
        let program = [guard(mov_imm(0, 1), 0, false), guard(mov_imm(1, 2), 0, true)];
        for (p0, expected) in [(1, [1, 11]), (0, [10, 2])] {
            let mut seed = vec![10, 11];
            seed.resize(PRED, 0);
            seed.push(p0);
            let Some((regs, _)) = run_regs(&program, &seed, &[], &[]) else {
                return;
            };
            assert_eq!(regs[..2], expected, "P0 = {p0}");
        }
    }

    #[test]
    fn uniform_instruction_is_guarded_by_uniform_predicate() {
        let program = [guard(uldc64(4, 0, 0), 0, false), guard(alu3(UIADD3 ^ IMM_B | preds(7, 7, 7, 7), 6, 63, 1, 63), 0, true)];
        for (p0, up0, expected) in [(1, 0, [7, 7, 1]), (0, 1, [10, 11, 0])] {
            let mut seed = vec![0; UPRED + 1];
            seed[PRED] = p0;
            seed[UREG + 4..UREG + 6].fill(7);
            seed[UPRED] = up0;
            let Some((regs, _)) = run_regs(&program, &seed, &[10, 11], &[]) else {
                return;
            };
            assert_eq!(regs[UREG + 4..UREG + 7], expected, "P0 = {p0}, UP0 = {up0}");
        }
    }

    #[test]
    fn predicated_bra_skips_mov() {
        let program = [guard(bra(16), 0, false), mov_imm(0, 1)];
        for (p0, expected) in [(1, 10), (0, 1)] {
            let mut seed = vec![10];
            seed.resize(PRED, 0);
            seed.push(p0);
            let Some((regs, _)) = run_regs(&program, &seed, &[], &[]) else {
                return;
            };
            assert_eq!(regs[0], expected, "P0 = {p0}");
        }
    }

    #[test]
    fn bra_forward_and_back_then_exit() {
        let program = [
            mov_imm(0, 1),
            bra(48),
            mov_imm(1, 2),
            exit(),
            mov_imm(2, 3),
            mov_imm(3, 4),
            bra(-80),
            mov_imm(4, 5),
        ];
        let Some((regs, _)) = run_regs(&program, &[10, 11, 12, 13, 14], &[], &[]) else {
            return;
        };
        assert_eq!(regs[..5], [1, 2, 12, 4, 14]);
    }

    #[test]
    fn bssy_bsync_diamond_reconverges() {
        let program = [
            bssy(0, 0x40),
            guard(bra(0x20), 0, false),
            mov_imm(0, 1),
            bra(0x10),
            mov_imm(0, 2),
            bsync(0),
            mov_imm(1, 3),
        ];
        for (p0, expected) in [(0, [1, 3]), (1, [2, 3])] {
            let mut seed = vec![10, 11];
            seed.resize(PRED, 0);
            seed.push(p0);
            let Some((regs, _)) = run_regs(&program, &seed, &[], &[]) else {
                return;
            };
            assert_eq!(regs[..2], expected, "P0 = {p0}");
        }
    }

    #[test]
    fn call_returns_to_the_address_in_r20() {
        let program = [
            mov_imm(20, 0x20),
            call_rel(0x20),
            iadd_imm(2, 0, 1),
            exit(),
            iadd_imm(0, 5, 0x10),
            ret_rel(20, -0x60),
        ];
        let Some((regs, _)) = run_regs(&program, &[10, 11, 12, 13, 14, 7], &[], &[]) else {
            return;
        };
        assert_eq!(regs[..3], [0x17, 11, 0x18]);
    }

    #[test]
    fn brx_and_jmx_jump_to_register_targets() {
        for jump in [brx(4, -0x10), jmx(4, 0)] {
            let program = [jump, mov_imm(0, 1), exit(), mov_imm(1, 2), mov_imm(2, 3)];
            for (r4, expected) in [(0x10, [1, 11, 12]), (0x30, [10, 2, 3]), (0x40, [10, 11, 3])] {
                let Some((regs, _)) = run_regs(&program, &[10, 11, 12, 13, r4], &[], &[]) else {
                    return;
                };
                assert_eq!(regs[..3], expected, "{jump:#x} R4 = {r4:#x}");
            }
        }
    }

    #[test]
    fn uniform_bra_follows_up0_and_a_converged_warp() {
        let program = [
            bra_upred(0, false, 0x10),
            mov_imm(0, 1),
            bra_div(0x10),
            mov_imm(1, 2),
            bra_conv(0x10),
            mov_imm(2, 3),
        ];
        for (up0, expected) in [(1, [10, 2, 12]), (0, [1, 2, 12])] {
            let mut seed = vec![10, 11, 12];
            seed.resize(UPRED, 0);
            seed.push(up0);
            let Some((regs, _)) = run_regs(&program, &seed, &[], &[]) else {
                return;
            };
            assert_eq!(regs[..3], expected, "UP0 = {up0}");
        }
    }

    #[test]
    fn kill_ends_the_invocation() {
        let program = [mov_imm(0, 1), kill(0, false), mov_imm(1, 2)];
        for (p0, expected) in [(1, [1, 11]), (0, [1, 2])] {
            let mut seed = vec![10, 11];
            seed.resize(PRED, 0);
            seed.push(p0);
            let Some((regs, _)) = run_regs(&program, &seed, &[], &[]) else {
                return;
            };
            assert_eq!(regs[..2], expected, "P0 = {p0}");
        }
    }
}
