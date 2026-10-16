#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::run_regs;
    use crate::tests::gpu_test::inst;

    #[test]
    fn translated_mov_and_iadd3_run_on_gpu() {
        let program = [inst::mov_imm(0, 42), inst::iadd_reg(1, 2, 3)];
        let Some(regs) = run_regs(&program, &[0, 0, 40, 2], &[]) else {
            return;
        };
        assert_eq!(regs[..3], [42, 42, 40]);
    }
}
