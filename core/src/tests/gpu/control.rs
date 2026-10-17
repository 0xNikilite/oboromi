#[cfg(test)]
mod tests {
    use crate::gpu::sm86::plan;
    use crate::tests::gpu::harness::{PRED, UPRED, UREG, both_ways, run_regs};
    use crate::tests::gpu_test::inst::{
        IMM_B, UIADD3, alu3, bmov_atexit, bra, bra_conv, bra_pp, bra_div, bra_upred, brx, bssy, bsync, call_rel, exit, guard, iadd_imm, jmx, kill, mov_imm, preds, ret_rel,
        uldc64, W2I_ARRIVES, W2I_BREAK, W2I_CCTLT, W2I_CSMTEST, W2I_DEPBAR, W2I_ERRBAR, W2I_IDE, W2I_JMP,
        W2I_LEPC, W2I_NANOSLEEP, W2I_PMTRIG, W2I_QSPC, W2I_R2B, W2I_RPCMOV_HI, W2I_RPCMOV_LO, W2I_RPCMOV_SET_HI,
        W2I_RPCMOV_SET_LO, W2I_RTT, W2I_SETLMEMBASE, W2I_YIELD,
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
    fn bra_takes_both_its_guard_and_its_condition_predicate() {
        let program = [guard(bra_pp(1, 16), 0, false), mov_imm(0, 1)];
        for (preds, expected) in [(0b11, 10), (0b01, 1), (0b10, 1)] {
            let mut seed = vec![10];
            seed.resize(PRED, 0);
            seed.push(preds);
            let Some((regs, _)) = run_regs(&program, &seed, &[], &[]) else {
                return;
            };
            assert_eq!(regs[0], expected, "P1P0 = {preds:02b}");
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
            let Some(regs) = run_structured(&program, &seed) else {
                return;
            };
            assert_eq!(regs[..2], expected, "P0 = {p0}");
        }
    }

    // ISETP.LT.AND Pu, PT, Ra, Rb or an immediate, PT
    fn isetp_lt(pu: u32, ra: u32, b: u32, imm: bool) -> u128 {
        let base = if imm { 0x004fc80003f01270000000000000780c } else { 0x004fc80003f01270000000000000720c };
        base | (pu as u128) << 81 | (ra as u128) << 24 | (b as u128) << 32
    }

    fn run_structured(program: &[u128], seed: &[u32]) -> Option<Vec<u32>> {
        let [dispatched, planned] = both_ways(|| run_regs(program, seed, &[], &[]).map(|(regs, _)| regs));
        assert_eq!(dispatched, planned);
        planned
    }

    #[test]
    fn if_else_nests_in_a_structured_loop() {
        let program = [
            iadd_imm(0, 0, 1),
            isetp_lt(0, 0, 3, true),
            guard(bra(0x20), 0, true),
            iadd_imm(2, 2, 10),
            bra(0x10),
            iadd_imm(2, 2, 1),
            isetp_lt(1, 0, 5, true),
            guard(bra(-0x80), 1, false),
        ];
        let Some(regs) = run_structured(&program, &[0, 0, 100]) else {
            return;
        };
        assert_eq!(regs[..3], [5, 0, 123]);
    }

    #[test]
    fn exit_inside_a_structured_loop_skips_the_code_after_it() {
        let program = [
            iadd_imm(0, 0, 1),
            isetp_lt(0, 0, 5, false),
            guard(exit(), 0, true),
            iadd_imm(1, 1, 1),
            isetp_lt(1, 0, 4, true),
            guard(bra(-0x60), 1, false),
            mov_imm(2, 7),
        ];
        for (r5, expected) in [(10, [4, 4, 7]), (2, [2, 1, 0])] {
            let Some(regs) = run_structured(&program, &[0, 0, 0, 0, 0, r5]) else {
                return;
            };
            assert_eq!(regs[..3], expected, "R5 = {r5}");
        }
    }

    #[test]
    fn structured_loop_leaves_through_either_exit() {
        let program = [
            iadd_imm(0, 0, 1),
            isetp_lt(0, 0, 5, false),
            guard(bra(0x40), 0, true),
            isetp_lt(1, 0, 4, true),
            guard(bra(-0x50), 1, false),
            mov_imm(2, 1),
            bra(0x10),
            mov_imm(2, 2),
            iadd_imm(3, 3, 1),
        ];
        for (r5, expected) in [(10, [4, 1, 1]), (2, [2, 2, 1])] {
            let Some(regs) = run_structured(&program, &[0, 0, 0, 0, 0, r5]) else {
                return;
            };
            assert_eq!([regs[0], regs[2], regs[3]], expected, "R5 = {r5}");
        }
    }

    #[test]
    fn irreducible_loop_stays_on_the_dispatcher() {
        let program = [guard(bra(0x10), 0, false), iadd_imm(0, 0, 1), iadd_imm(1, 1, 1), guard(bra(-0x30), 1, false), exit()];
        assert_eq!(plan(&program).unwrap_err(), "irreducible loop into pc 2");
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

    #[test]
    fn special_noops_leave_registers_untouched() {
        let words = [
            W2I_DEPBAR, W2I_YIELD, W2I_NANOSLEEP, W2I_PMTRIG, W2I_ERRBAR, W2I_CSMTEST, W2I_CCTLT, W2I_IDE, W2I_RTT, W2I_QSPC,
            W2I_R2B, W2I_ARRIVES, W2I_BREAK, W2I_SETLMEMBASE,
        ];
        for word in words {
            let Some((regs, _)) = run_regs(&[word], &[10, 11, 12], &[], &[]) else {
                return;
            };
            assert_eq!(regs[..3], [10, 11, 12], "{word:#x}");
        }
    }

    #[test]
    fn lepc_writes_its_own_byte_address() {
        let Some((regs, _)) = run_regs(&[mov_imm(0, 1), W2I_LEPC], &[10, 11, 12, 13], &[], &[]) else {
            return;
        };
        assert_eq!(regs[..4], [1, 11, 16, 0]);
    }

    #[test]
    fn exit_runs_the_atexit_routine_once() {
        for install in [bmov_atexit(0x30), 0x0001e400000000000000000000007357] {
            let program = [install, mov_imm(0, 1), exit(), iadd_imm(1, 1, 1), exit()];
            let Some((regs, _)) = run_regs(&program, &[0x30, 11], &[], &[]) else { return };
            assert_eq!(regs[..2], [1, 12]);
        }
    }

    #[test]
    fn jmp_uses_an_absolute_byte_address() {
        let program = [W2I_JMP, mov_imm(0, 1), exit(), mov_imm(1, 2)];
        let Some((regs, _)) = run_regs(&program, &[10, 11], &[], &[]) else {
            return;
        };
        assert_eq!(regs[..2], [10, 2]);
    }

    #[test]
    fn rpcmov_reads_the_next_byte_address_pair() {
        let program = [W2I_RPCMOV_SET_LO, W2I_RPCMOV_SET_HI, W2I_RPCMOV_LO, W2I_RPCMOV_HI];
        let Some((regs, _)) = run_regs(&program, &[10, 11, 12, 13], &[], &[]) else {
            return;
        };
        assert_eq!(regs[..4], [10, 11, 0x1234, 0x5678]);
    }
}
