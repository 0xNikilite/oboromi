#[cfg(test)]
mod tests {
    use crate::gpu::sm86::STATE_WORDS;
    use crate::tests::gpu::harness::run_regs;
    use crate::tests::gpu_test::inst::{bra, exit, guard, kill, mov_imm, uldc64};

    const P0: usize = 254;
    const UR4: usize = P0 + 1 + 4;

    #[test]
    fn predicate_guards_mov() {
        let program = [guard(mov_imm(0, 1), 0, false), guard(mov_imm(1, 2), 0, true)];
        for (p0, expected) in [(1, [1, 11]), (0, [10, 2])] {
            let mut seed = vec![10, 11];
            seed.resize(P0, 0);
            seed.push(p0);
            let Some((regs, _)) = run_regs(&program, &seed, &[], &[]) else {
                return;
            };
            assert_eq!(regs[..2], expected, "P0 = {p0}");
        }
    }

    #[test]
    fn uniform_instruction_is_guarded_by_uniform_predicate() {
        let program = [guard(uldc64(4, 0, 0), 0, false)];
        for (p0, up0, expected) in [(1, 0, [7, 7]), (0, 1, [10, 11])] {
            let mut seed = vec![0; STATE_WORDS];
            seed[P0] = p0;
            seed[UR4..UR4 + 2].fill(7);
            seed[STATE_WORDS - 1] = up0;
            let Some((regs, _)) = run_regs(&program, &seed, &[10, 11], &[]) else {
                return;
            };
            assert_eq!(regs[UR4..UR4 + 2], expected, "P0 = {p0}, UP0 = {up0}");
        }
    }

    #[test]
    fn predicated_bra_and_kill_skip_mov() {
        for skip in [guard(bra(16), 0, false), kill(0, false)] {
            for (p0, expected) in [(1, 10), (0, 1)] {
                let mut seed = vec![10];
                seed.resize(P0, 0);
                seed.push(p0);
                let Some((regs, _)) = run_regs(&[skip, mov_imm(0, 1)], &seed, &[], &[]) else {
                    return;
                };
                assert_eq!(regs[0], expected, "{skip:#x}, P0 = {p0}");
            }
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
}
