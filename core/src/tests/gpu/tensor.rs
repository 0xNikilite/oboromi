#[cfg(test)]
mod tests {
    use crate::tests::gpu::fp::fp16::tests::h;
    use crate::tests::gpu::harness::run_lanes;
    use crate::tests::gpu_test::inst::*;

    const RD: usize = 4;
    const RA: usize = 8;
    const RB: usize = 12;
    const RC: usize = 16;
    type Matrix = fn(usize, usize) -> f32;

    struct Sources {
        per_reg: usize,
        pack: fn(f32) -> u32,
    }

    const F16: Sources = Sources { per_reg: 2, pack: h };
    const BF16: Sources = Sources { per_reg: 2, pack: |x| x.to_bits() >> 16 };
    const INT8: Sources = Sources { per_reg: 4, pack: |x| x as i32 as u32 & 0xff };
    const INT4: Sources = Sources { per_reg: 8, pack: |x| x as i32 as u32 & 0xf };
    const TF32: Sources = Sources { per_reg: 1, pack: f32::to_bits };

    // Lane 4g + t holds A rows g and g + 8 and B column g at the per_reg consecutive k from per_reg * t, 4 * per_reg further
    // in every second A register and in every B register, and C/D rows g and g + 8 at columns 2t, 2t + 1.
    fn quad(lane: usize) -> (usize, usize) {
        (lane / 4, lane % 4)
    }

    fn pack(sources: &Sources, element: impl Fn(usize) -> f32) -> u32 {
        let bits = 32 / sources.per_reg;
        (0..sources.per_reg).fold(0, |word, i| word | (sources.pack)(element(i)) << bits * i)
    }

    fn d_element(lane: usize, reg: usize) -> (usize, usize) {
        let (g, t) = quad(lane);
        (g + 8 * (reg / 2), 2 * t + reg % 2)
    }

    fn run(program: &[u128], seeds: &[Vec<u32>]) -> Option<Vec<Vec<u32>>> {
        if !crate::gpu::device().is_ok_and(|d| d.shader_float16) {
            eprintln!("skipping: no shaderFloat16");
            return None;
        }
        run_lanes(program, seeds, &[], &[]).map(|(lanes, _)| lanes)
    }

    // Returns each lane's D registers next to A * B + C over k columns of A; a C of four registers holds `accumulator`
    // values, else f16 pairs.
    fn mma(word: u128, k: usize, sources: &Sources, [a, b, c]: [Matrix; 3], accumulator: Option<fn(f32) -> u32>) -> Option<Vec<(Vec<u32>, [f32; 4])>> {
        let k_groups = k / (4 * sources.per_reg);
        let seeds: Vec<Vec<u32>> = (0..32).map(|lane| {
            let mut regs = vec![0; RC + 4];
            for k_group in 0..k_groups {
                let k = sources.per_reg * (quad(lane).1 + 4 * k_group);
                for half in 0..2 {
                    regs[RA + 2 * k_group + half] = pack(sources, |i| a(quad(lane).0 + 8 * half, k + i));
                }
                regs[RB + k_group] = pack(sources, |i| b(k + i, quad(lane).0));
            }
            for reg in 0..4 {
                let (row, col) = d_element(lane, reg);
                match accumulator {
                    Some(bits) => regs[RC + reg] = bits(c(row, col)),
                    None => regs[RC + reg / 2] |= h(c(row, col)) << 16 * (reg % 2),
                }
            }
            regs
        }).collect();
        let lanes = run(&[alu3(word, RD as u32, RA as u32, RB as u32, RC as u32)], &seeds)?;
        Some((0..32).zip(lanes).map(|(lane, regs)| {
            let want = [0, 1, 2, 3].map(|reg| {
                let (row, col) = d_element(lane, reg);
                c(row, col) + (0..k).map(|i| a(row, i) * b(i, col)).sum::<f32>()
            });
            (regs[RD..RD + 4].to_vec(), want)
        }).collect())
    }

    fn assert_d(word: u128, k: usize, sources: &Sources, matrices: [Matrix; 3], accumulator: fn(f32) -> u32) {
        let Some(lanes) = mma(word, k, sources, matrices, Some(accumulator)) else { return };
        for (lane, (got, want)) in lanes.iter().enumerate() {
            assert_eq!(*got, want.map(accumulator), "lane {lane} {word:#x}");
        }
    }

    const SMALL_INTS: [Matrix; 3] = [
        |row, k| ((row * 3 + k * 5) % 7) as f32 - 3.0,
        |k, col| ((k * 2 + col * 5) % 7) as f32 - 3.0,
        |row, col| ((row * 4 + col * 5) % 7) as f32 * 100.0 - 300.0,
    ];

    #[test]
    fn hmma_16816_f32_matches_a_reference_matrix_multiply() {
        assert_d(HMMA | MMA_16816 | MMA_F32, 16, &F16, SMALL_INTS, f32::to_bits);
        assert_d(HMMA | MMA_16816 | MMA_F32 | MMA_BF16, 16, &BF16, SMALL_INTS, f32::to_bits);
    }

    #[test]
    fn hmma_1688_reduces_over_eight_k() {
        assert_d(HMMA | MMA_F32, 8, &F16, SMALL_INTS, f32::to_bits);
    }

    #[test]
    fn hmma_tf32_reduces_over_four_and_eight_k() {
        assert_d(HMMA | MMA_1684 | MMA_F32 | MMA_TF32, 4, &TF32, SMALL_INTS, f32::to_bits);
        assert_d(HMMA | MMA_F32 | MMA_TF32, 8, &TF32, SMALL_INTS, f32::to_bits);
    }

    // 2048 + 8 * 17/32 rounds to 2052 and 2052 + 8 * 17/32 to 2056, so D is rounded per group of eight k; rounding after
    // every product would stay at 2048.
    #[test]
    fn hmma_f16_rounds_each_group_of_eight_k_to_f16() {
        let matrices: [Matrix; 3] = [|_, _| 1.0, |_, _| 17.0 / 32.0, |_, _| 2048.0];
        let Some(lanes) = mma(HMMA | MMA_16816, 16, &F16, matrices, None) else { return };
        for (lane, (got, _)) in lanes.iter().enumerate() {
            assert_eq!(got[..2], [h(2056.0) * 0x1_0001; 2], "lane {lane}");
        }
    }

    #[test]
    fn imma_sums_signed_and_unsigned_bytes() {
        let unsigned: [Matrix; 3] = [|row, k| ((row * 37 + k * 11) % 256) as f32, |k, col| ((k * 13 + col * 71) % 256) as f32, |row, col| (row * 8 + col) as f32];
        assert_d(IMMA | IMMA_16816, 16, &INT8, unsigned, |x| x as i32 as u32);
        assert_d(IMMA | IMMA_16832 | IMMA_S8_A | IMMA_S8_B, 32, &INT8, SMALL_INTS, |x| x as i32 as u32);
    }

    // An 8-row tile holds A row g in RA and D row g in RD, RD + 1, so it is the top half of the 16-row layout.
    #[test]
    fn imma_sums_nibbles_and_8_row_tiles() {
        let unsigned: [Matrix; 3] = [|row, k| ((row * 7 + k * 3) % 16) as f32, |k, col| ((k * 5 + col * 11) % 16) as f32, |row, col| (row * 8 + col) as f32];
        for (word, k, sources, matrices, d_regs) in [
            (IMMA | IMMA_S8_A | IMMA_S8_B, 16, &INT8, SMALL_INTS, 2),
            (IMMA | IMMA_8832 | IMMA_4_BIT, 32, &INT4, unsigned, 2),
            (IMMA | IMMA_16832 | IMMA_4_BIT | IMMA_S8_A | IMMA_S8_B, 32, &INT4, SMALL_INTS, 4),
            (IMMA | IMMA_16864 | IMMA_4_BIT | IMMA_S8_A | IMMA_S8_B, 64, &INT4, SMALL_INTS, 4),
        ] {
            let Some(lanes) = mma(word, k, sources, matrices, Some(|x| x as i32 as u32)) else { return };
            for (lane, (got, want)) in lanes.iter().enumerate() {
                assert_eq!(got[..d_regs], want.map(|x| x as i32 as u32)[..d_regs], "lane {lane} {word:#x}");
            }
        }
    }

    // Rows 0 mod 3 start at 2^31 - 128 and rows 1 mod 3 at its negation, and add at least 1600 of their sign; f32 to i32 saturates.
    #[test]
    fn imma_sat_clamps_overflow_to_s32() {
        let matrices: [Matrix; 3] = [
            |row, _| if row % 3 == 1 { -100.0 } else { 100.0 },
            |k, col| ((k + col) % 5 + 1) as f32,
            |row, col| [2147483520.0, -2147483520.0, (row * 8 + col) as f32][row % 3],
        ];
        assert_d(IMMA | IMMA_16832 | IMMA_S8_A | IMMA_S8_B | IMMA_SAT, 32, &INT8, matrices, |x| x as i32 as u32);
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

    // Lane 4g + t holds A[g][t] in RA:RA+1, B[t][g] in RB:RB+1 and C/D[g][2t + i] in the pair at RC/RD + 2i.
    #[test]
    fn dmma_884_matches_an_f64_reference() {
        let a = |row: usize, k: usize| ((row * 3 + k * 5) % 7) as f64 - 3.0;
        let b = |k: usize, col: usize| ((k * 2 + col * 5) % 7) as f64 - 3.0;
        let c = |row: usize, col: usize| (row * 8 + col) as f64 * 0.5 - 7.0;
        let words = |x: f64| [x.to_bits() as u32, (x.to_bits() >> 32) as u32];
        let seeds: Vec<Vec<u32>> = (0..32).map(|lane| {
            let (g, t) = quad(lane);
            let mut regs = vec![0; RC + 4];
            regs[RA..RA + 2].copy_from_slice(&words(a(g, t)));
            regs[RB..RB + 2].copy_from_slice(&words(b(t, g)));
            for i in 0..2 {
                regs[RC + 2 * i..RC + 2 * i + 2].copy_from_slice(&words(c(g, 2 * t + i)));
            }
            regs
        }).collect();
        let Some((lanes, _)) = run_lanes(&[alu3(DMMA, RD as u32, RA as u32, RB as u32, RC as u32)], &seeds, &[], &[]) else { return };
        for (lane, regs) in (0..32).zip(lanes) {
            let (g, t) = quad(lane);
            let want = [0, 1].map(|i| words(c(g, 2 * t + i) + (0..4).map(|k| a(g, k) * b(k, 2 * t + i)).sum::<f64>()));
            assert_eq!(regs[RD..RD + 4], want.concat(), "lane {lane}");
        }
    }

    // A word of k_group j at lane 4g + t holds bits 128j + 32t.. of A row g (+ 8 in the odd registers of a 16-row tile)
    // and of B column g; C/D are laid out as for IMMA.
    #[test]
    fn bmma_sums_popcounts_of_xor_and_and() {
        let a = |row: usize, chunk: usize| (row as u32).wrapping_mul(0x9e37_79b9) ^ (chunk as u32).wrapping_mul(0x85eb_ca6b);
        let b = |chunk: usize, col: usize| (col as u32).wrapping_mul(0xc2b2_ae35) ^ (chunk as u32).wrapping_mul(0x27d4_eb2f);
        let c = |row: usize, col: usize| (row * 8 + col) as u32 * 3;
        for (word, rows, k_groups, product) in [
            (BMMA, 1, 1, (|x, y| x ^ y) as fn(u32, u32) -> u32),
            (BMMA | BMMA_168128 | BMMA_AND, 2, 1, |x, y| x & y),
            (BMMA | BMMA_168256, 2, 2, |x, y| x ^ y),
        ] {
            let chunk = |lane: usize, k_group: usize| quad(lane).1 + 4 * k_group;
            let seeds: Vec<Vec<u32>> = (0..32).map(|lane| {
                let mut regs = vec![0; RC + 4];
                for k_group in 0..k_groups {
                    for half in 0..rows {
                        regs[RA + rows * k_group + half] = a(quad(lane).0 + 8 * half, chunk(lane, k_group));
                    }
                    regs[RB + k_group] = b(chunk(lane, k_group), quad(lane).0);
                }
                for reg in 0..2 * rows {
                    let (row, col) = d_element(lane, reg);
                    regs[RC + reg] = c(row, col);
                }
                regs
            }).collect();
            let Some((lanes, _)) = run_lanes(&[alu3(word, RD as u32, RA as u32, RB as u32, RC as u32)], &seeds, &[], &[]) else { return };
            for (lane, regs) in (0..32).zip(lanes) {
                let want: Vec<u32> = (0..2 * rows).map(|reg| {
                    let (row, col) = d_element(lane, reg);
                    c(row, col) + (0..4 * k_groups).map(|chunk| product(a(row, chunk), b(chunk, col)).count_ones()).sum::<u32>()
                }).collect();
                assert_eq!(regs[RD..RD + 2 * rows], want, "lane {lane} {word:#x}");
            }
        }
    }

    // Lane 4g + t holds halves (g, 2t) and (g, 2t + 1) of an 8x8 tile, as LDSM.16.M88 leaves it; half (row, col) holds 8 * row + col.
    #[test]
    fn movm_mt88_transposes_an_8x8_tile() {
        let half = |row: usize, col: usize| (8 * row + col) as u32;
        let seeds: Vec<Vec<u32>> = (0..32).map(|lane| {
            let (g, t) = quad(lane);
            let mut regs = vec![0; RA + 1];
            regs[RA] = half(g, 2 * t + 1) << 16 | half(g, 2 * t);
            regs
        }).collect();
        let Some((lanes, _)) = run_lanes(&[alu(MOVM, RD as u32, RA as u32, 0)], &seeds, &[], &[]) else { return };
        for (lane, regs) in (0..32).zip(lanes) {
            let (g, t) = quad(lane);
            assert_eq!(regs[RD], half(2 * t + 1, g) << 16 | half(2 * t, g), "lane {lane}");
        }
    }
}
