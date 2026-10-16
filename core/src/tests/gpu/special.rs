#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::{UREG, run_regs};
    use crate::tests::gpu_test::inst::{cs2r, s2r, s2ur};

    #[test]
    fn special_registers_write_their_destination_width() {
        let seed = vec![u32::MAX; UREG + 6];
        let program = [s2r(0, 33), s2r(1, 39), s2r(2, 0), cs2r(4, 80, true), cs2r(6, 81, false), s2ur(5, 37)];
        let Some((regs, _)) = run_regs(&program, &seed, &[], &[]) else {
            return;
        };
        assert_eq!(regs[..8], [0, 0, 0, u32::MAX, 0, 0, 0, u32::MAX]);
        assert_eq!(regs[UREG + 5], 0);
    }
}
