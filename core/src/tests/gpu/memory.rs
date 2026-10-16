#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::{GLOBAL_BASE, run_regs};
    use crate::tests::gpu_test::inst;

    const BASE: [u32; 2] = [GLOBAL_BASE as u32, (GLOBAL_BASE >> 32) as u32];

    #[test]
    fn ldg_reads_global_word() {
        let seed = [0, 0, BASE[0], BASE[1]];
        let Some((regs, _)) = run_regs(&[inst::ldg(4, 2, 8)], &seed, &[], &[1, 2, 0xdead_beef])
        else {
            return;
        };
        assert_eq!(regs[4], 0xdead_beef);
    }

    #[test]
    fn stg_writes_global_word() {
        let seed = [0, 0, BASE[0], BASE[1], 0, 0x1234];
        let Some((_, global)) = run_regs(&[inst::stg(2, 0x10, 5)], &seed, &[], &[0; 5]) else {
            return;
        };
        assert_eq!(global, [0, 0, 0, 0, 0x1234]);
    }

    #[test]
    fn ldc_and_uldc_read_cbuf() {
        let program = [inst::ldc(6, 0, 3, 4), inst::uldc64(4, 0, 0x10)];
        let Some((regs, _)) = run_regs(&program, &[0, 0, 0, 8], &[10, 11, 12, 13, 14, 15], &[])
        else {
            return;
        };
        let ur4 = 254 + 1 + 4;
        assert_eq!((regs[6], &regs[ur4..ur4 + 2]), (13, &[14, 15][..]));
    }
}
