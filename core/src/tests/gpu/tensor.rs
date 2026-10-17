#[cfg(test)]
mod tests {
    use crate::tests::gpu::fp16::tests::h;
    use crate::tests::gpu::harness::run_lanes;
    use crate::tests::gpu_test::inst::*;

    const RD: usize = 4;
    const RA: usize = 8;
    const RB: usize = 12;
    const RC: usize = 16;
    type Tile<const N: usize> = [[f32; N]; 16];

    // Lane 4g + t holds A rows g and g + 8 at k = 2t, 2t + 1 (k + 8 from the third register), B column g at the same k,
    // and C/D rows g and g + 8 at columns 2t, 2t + 1.
    fn quad(lane: usize) -> (usize, usize) {
        (lane / 4, lane % 4)
    }

    fn a_register(a: &Tile<16>, lane: usize, reg: usize, pack: fn(f32) -> u32) -> u32 {
        let (g, t) = quad(lane);
        let (row, k) = (g + 8 * (reg % 2), 2 * t + 8 * (reg / 2));
        pack(a[row][k + 1]) << 16 | pack(a[row][k])
    }

    fn b_register(b: &Tile<8>, lane: usize, reg: usize, pack: fn(f32) -> u32) -> u32 {
        let (g, t) = quad(lane);
        let k = 2 * t + 8 * reg;
        pack(b[k + 1][g]) << 16 | pack(b[k][g])
    }

    fn d_element(lane: usize, reg: usize) -> (usize, usize) {
        let (g, t) = quad(lane);
        (g + 8 * (reg / 2), 2 * t + reg % 2)
    }

    fn bf16(x: f32) -> u32 {
        x.to_bits() >> 16
    }

    fn run(program: &[u128], seeds: &[Vec<u32>]) -> Option<Vec<Vec<u32>>> {
        if !crate::gpu::device().is_ok_and(|d| d.shader_float16) {
            eprintln!("skipping: no shaderFloat16");
            return None;
        }
        run_lanes(program, seeds, &[], &[]).map(|(lanes, _)| lanes)
    }

    // Returns each lane's D registers next to A * B + C over the first `k` columns of A.
    fn mma(word: u128, k: usize, a: &Tile<16>, b: &Tile<8>, c: &Tile<8>, pack: fn(f32) -> u32) -> Option<Vec<(Vec<u32>, [f32; 4])>> {
        let f32_acc = word & MMA_F32 != 0;
        let seeds: Vec<Vec<u32>> = (0..32).map(|lane| {
            let mut regs = vec![0; RC + 4];
            for reg in 0..k / 4 {
                regs[RA + reg] = a_register(a, lane, reg, pack);
            }
            for reg in 0..k / 8 {
                regs[RB + reg] = b_register(b, lane, reg, pack);
            }
            for reg in 0..4 {
                let (row, col) = d_element(lane, reg);
                if f32_acc {
                    regs[RC + reg] = c[row][col].to_bits();
                } else {
                    regs[RC + reg / 2] |= h(c[row][col]) << 16 * (reg % 2);
                }
            }
            regs
        }).collect();
        let lanes = run(&[alu3(word, RD as u32, RA as u32, RB as u32, RC as u32)], &seeds)?;
        Some((0..32).zip(lanes).map(|(lane, regs)| {
            let want = [0, 1, 2, 3].map(|reg| {
                let (row, col) = d_element(lane, reg);
                c[row][col] + (0..k).map(|i| a[row][i] * b[i][col]).sum::<f32>()
            });
            (regs[RD..RD + 4].to_vec(), want)
        }).collect())
    }

    fn small_ints<const N: usize>(seed: usize) -> Tile<N> {
        std::array::from_fn(|i| std::array::from_fn(|j| ((i * seed + j * 5) % 7) as f32 - 3.0))
    }

    #[test]
    fn hmma_16816_f32_matches_a_reference_matrix_multiply() {
        let (a, b, c) = (small_ints(3), small_ints(2), small_ints::<8>(4).map(|row| row.map(|x| x * 100.0)));
        for (word, pack) in [(HMMA | MMA_16816 | MMA_F32, h as fn(f32) -> u32), (HMMA | MMA_16816 | MMA_F32 | MMA_BF16, bf16)] {
            let Some(lanes) = mma(word, 16, &a, &b, &c, pack) else { return };
            for (lane, (got, want)) in lanes.iter().enumerate() {
                assert_eq!(*got, want.map(f32::to_bits), "lane {lane} {word:#x}");
            }
        }
    }

    #[test]
    fn hmma_1688_reduces_over_eight_k() {
        let (a, b, c) = (small_ints(3), small_ints(2), small_ints(4));
        let Some(lanes) = mma(HMMA | MMA_F32, 8, &a, &b, &c, h) else { return };
        for (lane, (got, want)) in lanes.iter().enumerate() {
            assert_eq!(*got, want.map(f32::to_bits), "lane {lane}");
        }
    }

    // 2048 + 16 * 17/32 = 2056.5 rounds to 2056, so C is kept in f32 across k; summing in f16 would stay at 2048.
    #[test]
    fn hmma_f16_accumulates_in_f32_and_rounds_d_to_f16() {
        let (a, b, c) = ([[1.0; 16]; 16], [[17.0 / 32.0; 8]; 16], [[2048.0; 8]; 16]);
        let Some(lanes) = mma(HMMA | MMA_16816, 16, &a, &b, &c, h) else { return };
        for (lane, (got, _)) in lanes.iter().enumerate() {
            assert_eq!(got[..2], [h(2056.0) * 0x1_0001; 2], "lane {lane}");
        }
    }

    // Lane L stores row L of the four 8x8 tiles at row slot 5L mod 32 and hands that address to LDSM, so a lane reading
    // the wrong lane's address reads the wrong row; half (row, col) holds 8 * row + col.
    #[test]
    fn ldsm_x4_distributes_rows_and_mt88_transposes_them() {
        let slot = |row: u32| 16 * (5 * row % 32);
        let half = |row: u32, col: u32| 8 * row + col;
        let seeds: Vec<Vec<u32>> = (0..32).map(|lane| {
            let mut regs = vec![0; 24];
            regs[2] = slot(lane);
            for w in 0..4 {
                regs[20 + w] = slot(lane) + 4 * w as u32;
                regs[16 + w] = half(lane, 2 * w as u32 + 1) << 16 | half(lane, 2 * w as u32);
            }
            regs
        }).collect();
        let program = [
            sts(20, 16), sts(21, 17), sts(22, 18), sts(23, 19),
            alu(LDSM | LDSM_X4, 4, 2, 0),
            alu(LDSM | LDSM_X4 | LDSM_MT88, 8, 2, 0),
        ];
        let Some(lanes) = run(&program, &seeds) else { return };
        for (lane, regs) in (0..32).zip(lanes) {
            let (g, t) = (lane / 4, lane % 4);
            let rows = [0, 1, 2, 3].map(|tile| half(8 * tile + g, 2 * t + 1) << 16 | half(8 * tile + g, 2 * t));
            let columns = [0, 1, 2, 3].map(|tile| half(8 * tile + 2 * t + 1, g) << 16 | half(8 * tile + 2 * t, g));
            assert_eq!(regs[4..8], rows, "lane {lane}");
            assert_eq!(regs[8..12], columns, "lane {lane}");
        }
    }
}
