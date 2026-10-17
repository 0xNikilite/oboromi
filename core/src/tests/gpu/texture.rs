#[cfg(test)]
mod tests {
    use crate::gpu::texture::{TexturePool, block_linear_to_pitch, pitch_to_block_linear};
    use crate::tests::gpu::harness::{Textures, run_textured};
    use crate::tests::gpu_test::inst::{self, texture};

    const TEXELS_ADDRESS: u64 = 0x1_0000_0400;
    const PITCH: usize = 32;
    const NEAREST: u32 = 1;
    const LINEAR: u32 = 2;
    const RGBA8_UNORM: u32 = 8 | 0b010_010_010_010 << 7;
    const R32_UINT: u32 = 0x0f | 0b100_100_100_100 << 7;

    fn run(program: &[u128], seeds: &[Vec<u32>], texels: &[u8], filter: u32) -> Option<Vec<Vec<u32>>> {
        run_format(program, seeds, texels, filter, RGBA8_UNORM)
    }

    // A pitch-linear 4x4 header of `format` and a clamp-to-edge sampler.
    fn run_format(program: &[u128], seeds: &[Vec<u32>], texels: &[u8], filter: u32, format: u32) -> Option<Vec<Vec<u32>>> {
        let header = [
            format | 0o5432 << 19,
            TEXELS_ADDRESS as u32,
            (TEXELS_ADDRESS >> 32) as u32 | 2 << 21,
            PITCH as u32 >> 5,
            3 | 1 << 23,
            3 | 1 << 31,
            0,
            0,
        ];
        let rows: Vec<u8> = texels.chunks(16).flat_map(|row| [row, &[0; PITCH - 16]].concat()).collect();
        run_entries(program, seeds, header, [2 | 2 << 3, filter | filter << 4, 0, 0, 0, 0, 0, 0], &rows)
    }

    // `header` at TIC index 1 and `sampler` at TSC index 1, the header's texels at TEXELS_ADDRESS.
    fn run_entries(program: &[u128], seeds: &[Vec<u32>], header: [u32; 8], sampler: [u32; 8], texels: &[u8]) -> Option<Vec<Vec<u32>>> {
        let entries = |entry: [u32; 8]| -> Vec<u8> { [[0; 8], entry].iter().flatten().flat_map(|w| w.to_le_bytes()).collect() };
        let (tic, tsc) = (entries(header), entries(sampler));
        let memory = [(TEXELS_ADDRESS, texels)];
        let textures = Textures { handle: 1 << 20 | 1, pool: TexturePool { tic: &tic, tsc: &tsc }, memory: &memory };
        run_textured(|d| d.translate_program(program), seeds, &[], &[], &textures).map(|(lanes, _)| lanes)
    }

    #[test]
    fn tld_fetches_unorm_texels_packed_by_write_mask() {
        let texels: Vec<u8> = (0..64).map(|i| 4 * i + 3).collect();
        let seeds: Vec<Vec<u32>> = (0..16).map(|t| vec![0, 0, t % 4, t / 4]).collect();
        let program = [texture(inst::TLD_LZ_2D, 4, 6, 2, 3, 0x10, 0xf), texture(inst::TLD_LZ_2D, 8, 0xff, 2, 3, 0x10, 0b1010)];
        let Some(lanes) = run(&program, &seeds, &texels, NEAREST) else { return };
        // Lavapipe lands one ulp off n / 255, which Vulkan allows, so registers are compared as bytes.
        for (t, regs) in lanes.iter().enumerate() {
            let bytes: Vec<u8> = regs[4..10].iter().map(|&r| (f32::from_bits(r) * 255.0).round() as u8).collect();
            assert_eq!(bytes, [0, 1, 2, 3, 1, 3].map(|c| texels[4 * t + c]), "texel {t}");
        }
    }

    #[test]
    fn tex_lod_zero_nearest_at_texel_centers_matches_tld() {
        let texels: Vec<u8> = (0..64).map(|i| 4 * i + 3).collect();
        let center = |i: u32| ((i as f32 + 0.5) / 4.0).to_bits();
        let seeds: Vec<Vec<u32>> = (0..16).map(|t| vec![t % 4, t / 4, center(t % 4), center(t / 4)]).collect();
        let program = [texture(inst::TLD_LZ_2D, 4, 6, 0, 3, 0x10, 0xf), texture(inst::TEX_LZ_2D, 8, 10, 2, 3, 0x10, 0xf)];
        let Some(lanes) = run(&program, &seeds, &texels, NEAREST) else { return };
        for (t, regs) in lanes.iter().enumerate() {
            assert_eq!(regs[8..12], regs[4..8], "texel {t}");
        }
    }

    #[test]
    fn tex_linear_between_two_texels_gives_their_midpoint() {
        let texels: Vec<u8> = (0..64).map(|i| (i * 37 % 256) as u8).collect();
        let seeds: Vec<Vec<u32>> = (0..12).map(|t| vec![0, 0, ((t % 3 + 1) as f32 / 4.0).to_bits(), ((t / 3) as f32 / 4.0 + 0.125).to_bits()]).collect();
        let program = [texture(inst::TEX_LZ_2D, 4, 6, 2, 3, 0x10, 0xf)];
        let Some(lanes) = run(&program, &seeds, &texels, LINEAR) else { return };
        for (t, regs) in lanes.iter().enumerate() {
            let left = 4 * (t / 3 * 4 + t % 3);
            for c in 0..4 {
                let midpoint = (texels[left + c] as f32 + texels[left + 4 + c] as f32) / 2.0;
                assert!((f32::from_bits(regs[4 + c]) * 255.0 - midpoint).abs() <= 1.0, "lane {t} component {c}");
            }
        }
    }

    #[test]
    fn tex_lod_bias_explicit_lod_and_clamps_sample_the_only_mip() {
        let texels: Vec<u8> = (0..64).map(|i| 4 * i + 3).collect();
        let center = |i: u32| ((i as f32 + 0.5) / 4.0).to_bits();
        let mut seed = vec![0; 9];
        (seed[4], seed[5], seed[6], seed[8]) = (center(1), center(2), 0, 0);
        let sample = |op, rd, rb| inst::with_rb(texture(op, rd, 0xff, 4, 3, 0x10, 1), rb);
        let program = [
            sample(inst::TEX_LB_2D, 10, 8),
            sample(inst::TEX_LL_2D, 11, 8),
            sample(inst::TEX_LC_2D, 12, 8),
            sample(inst::TEX_LB_LC_2D, 13, 8),
        ];
        let Some(lanes) = run(&program, &vec![seed; 4], &texels, NEAREST) else { return };
        let expected = texels[4 * (2 * 4 + 1)];
        for regs in lanes {
            assert!(regs[10..14].iter().all(|&r| (f32::from_bits(r) * 255.0).round() as u8 == expected));
        }
    }

    #[test]
    fn tex_depth_compare_forms_pass_spirv_validation() {
        let word = inst::with_rb(texture(inst::TEX_LZ_2D | inst::TEX_DC, 8, 12, 4, 3, 0x10, 0xf), 6);
        spirv_val(&translate_module(&[word], true, None, false));
    }

    #[test]
    fn txq_returns_width_height_and_levels() {
        let program = [texture(inst::TXQ_DIMENSION, 4, 6, 0xff, 3, 0x10, 0b1011)];
        let Some(lanes) = run(&program, &[vec![]], &[0; 64], NEAREST) else { return };
        assert_eq!(lanes[0][4..7], [4, 4, 1]);
    }

    #[test]
    fn sust_texels_read_back_through_suld_in_both_forms() {
        let pattern = |t: u32, base: u32| -> [u8; 4] { [0, 1, 2, 3].map(|c| (base + 16 * t + 4 * c) as u8) };
        let unorm = |byte: u8| (byte as f32 / 255.0).to_bits();
        let seeds: Vec<Vec<u32>> = (0..16)
            .map(|t| {
                let [x, y] = [t % 4, t / 4];
                let [r, g, b, a] = pattern(t, 1).map(unorm);
                vec![0, 0, x, y, 0, u32::from_le_bytes(pattern(t, 2)), 0, 0, r, g, b, a]
            })
            .collect();
        let program = [
            inst::surface(inst::SUST_P_RGBA, 0xff, 2, 8, 3, 0x10),
            inst::surface(inst::SULD_D_32, 4, 2, 0xff, 3, 0x10),
            inst::surface(inst::SUST_D_32, 0xff, 2, 5, 3, 0x10),
            inst::surface(inst::SULD_P_RGBA, 12, 2, 0xff, 3, 0x10),
        ];
        let Some(lanes) = run(&program, &seeds, &[0; 64], NEAREST) else { return };
        for (t, regs) in lanes.iter().enumerate() {
            assert_eq!(regs[4], u32::from_le_bytes(pattern(t as u32, 1)), "lane {t} .D readback");
            let bytes = regs[12..16].iter().map(|&r| (f32::from_bits(r) * 255.0).round() as u8);
            assert_eq!(bytes.collect::<Vec<_>>(), pattern(t as u32, 2), "lane {t} .P readback");
        }
    }

    #[test]
    fn tld4_gathers_the_red_footprint_in_gather_order() {
        let texels: Vec<u8> = (0..64).map(|i| 4 * i + 3).collect();
        let red = |x: u32, y: u32| texels[16 * y as usize + 4 * x as usize];
        let corner = |i: u32| ((i + 1) as f32 / 4.0).to_bits();
        let seeds: Vec<Vec<u32>> = (0..9).map(|t| vec![0, 0, corner(t % 3), corner(t / 3)]).collect();
        let program = [texture(inst::TLD4_R_2D, 4, 6, 2, 3, 0x10, 0xf)];
        let Some(lanes) = run(&program, &seeds, &texels, NEAREST) else { return };
        for (t, regs) in lanes.iter().enumerate() {
            let [x, y] = [t as u32 % 3, t as u32 / 3];
            let bytes: Vec<u8> = regs[4..8].iter().map(|&r| (f32::from_bits(r) * 255.0).round() as u8).collect();
            assert_eq!(bytes, [red(x, y + 1), red(x + 1, y + 1), red(x + 1, y), red(x, y)], "lane {t}");
        }
    }

    #[test]
    fn txd_on_the_only_mip_matches_tex_lod_zero_for_any_gradient() {
        let texels: Vec<u8> = (0..64).map(|i| 4 * i + 3).collect();
        let center = |i: u32| ((i as f32 + 0.5) / 4.0).to_bits();
        let seeds: Vec<Vec<u32>> = (0..16)
            .map(|t| [vec![0, 0, center(t % 4), center(t / 4)], vec![0; 16], vec![8f32.to_bits(); 4]].concat())
            .collect();
        let program = [
            texture(inst::TEX_LZ_2D, 8, 10, 2, 3, 0x10, 0xf),
            inst::with_rb(texture(inst::TXD_2D, 12, 14, 2, 3, 0x10, 0xf), 4),
            inst::with_rb(texture(inst::TXD_2D, 16, 18, 2, 3, 0x10, 0xf), 20),
        ];
        let Some(lanes) = run(&program, &seeds, &texels, NEAREST) else { return };
        for (t, regs) in lanes.iter().enumerate() {
            assert_eq!([&regs[12..16], &regs[16..20]], [&regs[8..12]; 2], "lane {t}");
        }
    }

    #[test]
    fn tmml_returns_fixed_point_lod_then_level_from_quad_derivatives() {
        let step = |quad: u32| quad as f32 + 1.0;
        let seeds: Vec<Vec<u32>> = (0..8)
            .map(|t| {
                let [quad, x, y] = [t / 4, t & 1, t >> 1 & 1];
                let coord = |offset: u32| ((0.5 + (offset as f32) * step(quad)) / 4.0).to_bits();
                vec![0, 0, coord(x), coord(y)]
            })
            .collect();
        let program = [texture(inst::TMML_LOD_2D, 4, 0xff, 2, 3, 0x10, 0x3)];
        let Some(lanes) = run(&program, &seeds, &[0; 64], NEAREST) else { return };
        for (t, regs) in lanes.iter().enumerate() {
            assert_eq!(regs[4..6], [0x100 * (t as u32 / 4), 0], "lane {t}");
        }
    }

    #[test]
    fn sured_adds_from_every_lane_and_suatom_returns_each_old_value() {
        let seeds = vec![vec![0, 0, 1, 1, 1]; 32];
        let program = [
            inst::surface(inst::SURED_ADD, 0, 2, 4, 3, 0x10),
            inst::bar_sync(),
            inst::surface(inst::SUATOM_ADD, 5, 2, 4, 3, 0x10),
        ];
        let Some(lanes) = run_format(&program, &seeds, &[0; 64], NEAREST, R32_UINT) else { return };
        let mut old: Vec<u32> = lanes.iter().map(|regs| regs[5]).collect();
        old.sort();
        assert_eq!(old, (32..64).collect::<Vec<_>>());
    }

    fn converter_block_linear(pitch: &[u8], row_bytes: u32, gob_height_log2: u32) -> Option<Vec<u8>> {
        let script = "import sys, numpy as np; sys.path.insert(0, 'scripts'); from gltf_to_raw import block_linear; \
            n, g = map(int, sys.argv[1:]); sys.stdout.buffer.write(block_linear(np.frombuffer(sys.stdin.buffer.read(), np.uint8).reshape(-1, n), g).tobytes())";
        let mut child = std::process::Command::new("python3")
            .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
            .args(["-B", "-c", script, &row_bytes.to_string(), &gob_height_log2.to_string()])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .ok()?;
        std::io::Write::write_all(&mut child.stdin.take().unwrap(), pitch).unwrap();
        let out = child.wait_with_output().unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success() || stderr.contains("ModuleNotFoundError"), "{stderr}");
        out.status.success().then_some(out.stdout)
    }

    #[test]
    fn block_linear_round_trips_and_matches_the_converter() {
        let rgba: Vec<u8> = (0..64 * 64).flat_map(|i| [i % 64 * 4, i / 64 * 4, (i % 64 + i / 64) * 2, 255].map(|c| c as u8)).collect();
        let r8: Vec<u8> = (0..128 * 32).map(|i| (i % 128 * 2 + i / 128) as u8).collect();
        for (pitch, width, height, bytes_per_pixel) in [(&rgba, 64, 64, 4), (&r8, 128, 32, 1)] {
            let swizzled = pitch_to_block_linear(pitch, width, height, bytes_per_pixel, 1);
            assert_eq!(block_linear_to_pitch(&swizzled, width, height, bytes_per_pixel, 1), *pitch);
            match converter_block_linear(pitch, width * bytes_per_pixel, 1) {
                Some(converted) => assert_eq!(converted, swizzled, "{width}x{height}"),
                None => eprintln!("skipping the converter: no numpy"),
            }
        }
        // Pixel (9, 11) is byte 36 of row 11: the second GOB of its block, the upper 32 bytes, row pair 1, odd row, byte 4.
        let swizzled = pitch_to_block_linear(&rgba, 64, 64, 4, 1);
        assert_eq!(swizzled[512 + 256 + 64 + 16 + 4..][..4], rgba[4 * (11 * 64 + 9)..][..4]);
    }

    #[test]
    fn tex_ll_samples_each_level_of_a_block_linear_mip_chain() {
        let level0: Vec<u8> = (0..64 * 16 * 4).map(|i| (i * 3 + 1) as u8).collect();
        let level1: Vec<u8> = (0..32 * 8 * 4).map(|i| (255 - i * 5) as u8).collect();
        // Level 1 is eight rows, so its gob height halves to one GOB and its second GOB column starts 512 bytes in.
        let texels = [pitch_to_block_linear(&level0, 64, 16, 4, 1), pitch_to_block_linear(&level1, 32, 8, 4, 0)].concat();
        let header = [
            RGBA8_UNORM | 0o5432 << 19,
            TEXELS_ADDRESS as u32,
            (TEXELS_ADDRESS >> 32) as u32 | 3 << 21,
            1 << 3 | 1 << 28,
            63 | 1 << 23,
            15 | 1 << 31,
            0,
            0,
        ];
        const MIP_POINT: u32 = 2;
        let sampler = [2 | 2 << 3, NEAREST | NEAREST << 4 | MIP_POINT << 6, 256 << 12, 0, 0, 0, 0, 0];
        let center = |i: u32, size: u32| ((i as f32 + 0.5) / size as f32).to_bits();
        let level0_texels: Vec<[u32; 2]> = (0..256).map(|t| [t % 64, t / 64 * 5]).collect();
        let seeds: Vec<Vec<u32>> = level0_texels
            .iter()
            .map(|&[x, y]| vec![0, 0, center(x, 64), center(y, 16), 0, 0, 0, 0, 0])
            .chain((0..256).map(|t| vec![0, 0, center(t % 32, 32), center(t / 32, 8), 0, 0, 0, 0, 1f32.to_bits()]))
            .collect();
        let program = [inst::with_rb(texture(inst::TEX_LL_2D, 4, 6, 2, 3, 0x10, 0xf), 8)];
        let Some(lanes) = run_entries(&program, &seeds, header, sampler, &texels) else { return };
        let expected = level0_texels.iter().map(|&[x, y]| &level0[4 * (64 * y + x) as usize..][..4]).chain(level1.chunks(4));
        for (t, (regs, texel)) in lanes.iter().zip(expected).enumerate() {
            let bytes: Vec<u8> = regs[4..8].iter().map(|&r| (f32::from_bits(r) * 255.0).round() as u8).collect();
            assert_eq!(bytes, texel, "lane {t}");
        }
    }

    #[test]
    fn tld_fetches_bc1_blocks_from_a_block_linear_texture() {
        let device = crate::gpu::device();
        if device.is_ok_and(|d| unsafe { d.instance.get_physical_device_features(d.physical_device) }.texture_compression_bc == 0) {
            eprintln!("skipping: Vulkan device lacks textureCompressionBC");
            return;
        }
        const DXT1_UNORM: u32 = 0x24 | 0b010_010_010_010 << 7;
        // Each 4x4 block is its first 565 endpoint everywhere: red, green, blue and white.
        let colors = [0xf800u16, 0x07e0, 0x001f, 0xffff];
        let blocks: Vec<u8> = colors.iter().flat_map(|c| [c.to_le_bytes(), [0; 2], [0; 2], [0; 2]].concat()).collect();
        let header = [
            DXT1_UNORM | 0o5432 << 19,
            TEXELS_ADDRESS as u32,
            (TEXELS_ADDRESS >> 32) as u32 | 3 << 21,
            0,
            7 | 1 << 23,
            7 | 1 << 31,
            0,
            0,
        ];
        let seeds: Vec<Vec<u32>> = (0..64).map(|t| vec![0, 0, t % 8, t / 8]).collect();
        let program = [texture(inst::TLD_LZ_2D, 4, 6, 2, 3, 0x10, 0xf)];
        let sampler = [2 | 2 << 3, NEAREST | NEAREST << 4, 0, 0, 0, 0, 0, 0];
        let Some(lanes) = run_entries(&program, &seeds, header, sampler, &pitch_to_block_linear(&blocks, 2, 2, 8, 0)) else { return };
        let rgba = [[1, 0, 0, 1], [0, 1, 0, 1], [0, 0, 1, 1], [1, 1, 1, 1]];
        for (t, regs) in lanes.iter().enumerate() {
            let block = t / 32 * 2 + t % 8 / 4;
            assert_eq!(regs[4..8].iter().map(|&r| f32::from_bits(r)).collect::<Vec<_>>(), rgba[block].map(|c| c as f32), "texel {t}");
        }
    }
}
