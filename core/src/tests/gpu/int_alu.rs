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

    #[test]
    fn mov_reads_cbuf_and_uniform_register() {
        let ur4 = 254 + 1 + 4;
        let mut seed = vec![0; ur4 + 1];
        seed[ur4] = 0x5678;
        let program = [inst::mov_cbuf(5, 0, 0x10), inst::mov_ureg(6, 4)];
        let Some(regs) = run_regs(&program, &seed, &[0, 0, 0, 0, 0x1234]) else {
            return;
        };
        assert_eq!(regs[5..7], [0x1234, 0x5678]);
    }
}
