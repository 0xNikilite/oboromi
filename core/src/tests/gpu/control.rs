#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::run_regs;
    use crate::tests::gpu_test::inst::{guard, mov_imm};

    const P0: usize = 254;

    #[test]
    fn predicate_guards_mov() {
        let program = [guard(mov_imm(0, 1), 0, false), guard(mov_imm(1, 2), 0, true)];
        for (p0, expected) in [(1, [1, 11]), (0, [10, 2])] {
            let mut seed = vec![10, 11];
            seed.resize(P0, 0);
            seed.push(p0);
            let Some(regs) = run_regs(&program, &seed, &[]) else {
                return;
            };
            assert_eq!(regs[..2], expected, "P0 = {p0}");
        }
    }
}
