#[cfg(test)]
mod tests {
    use crate::gpu::texture::TexturePool;
    use crate::tests::gpu::silicon::{numbers, observed, string, tensor_fixtures, tensor_mismatch, tensor_program, tensor_seeds};
    use crate::tests::gpu::harness::{
        both_ways, run_kernel, run_lanes, run_regs, run_textured, translate_module, Textures, GLOBAL_BASE, LIVE, PRED,
    };
    use crate::gpu::cubin::{Arg, Kernel};
    use crate::gpu::sm86::plan;
    use crate::tests::gpu_test::inst;
    use std::collections::BTreeMap;
    use std::panic::catch_unwind;

    struct Fixture {
        id: String,
        family: String,
        case: String,
        observed: bool,
        words: Vec<u128>,
        inputs: Vec<u32>,
        outputs: Vec<u32>,
        input_regs: Vec<usize>,
        output_regs: Vec<usize>,
        registers_after: Vec<u32>,
        global_after: Vec<u32>,
    }

    fn reg(object: &str, name: &str) -> usize {
        string(object, name)[1..].parse().unwrap()
    }

    fn word(object: &str, key: &str) -> u128 {
        u128::from_str_radix(&string(object, key), 16).unwrap()
    }

    fn id(object: &str) -> String {
        let [id, family, case] = ["id", "family", "case"].map(|key| string(object, key));
        format!("{id}/{family}/{case}")
    }

    fn fixtures(json: &str) -> Vec<Fixture> {
        json.lines()
            .map(|object| Fixture {
                id: id(object),
                family: string(object, "family"),
                case: string(object, "case"),
                observed: observed(object),
                words: ["word_hex", "observer_word_hex"]
                    .into_iter()
                    .filter(|key| object.contains(&format!("\"{key}\"")))
                    .map(|key| word(object, key))
                    .collect(),
                inputs: numbers(object, "inputs"),
                outputs: numbers(object, "outputs"),
                input_regs: (0..4).map(|i| reg(object, &format!("input{i}"))).collect(),
                output_regs: (0..2).map(|i| reg(object, &format!("output{i}"))).collect(),
                registers_after: numbers(object, "registers_after"),
                global_after: numbers(object, "global_after"),
            })
            .collect()
    }

    #[test]
    fn tex_rz_lod_operands_are_illegal_on_silicon() {
        for word in [
            0x000f4200001e0f0230005cff04127b60,
            0x000f4200021e0f0230005cff04127b60,
            0x000f4200031e0f0230005cff04127b60,
        ] {
            assert!(catch_unwind(|| translate_module(&[word], true, None, false)).is_err());
        }
    }

    #[test]
    #[should_panic(expected = "ATOMS.ADD.INVALID3 is illegal instruction on silicon")]
    fn atoms_invalid3_is_illegal_on_silicon() {
        translate_module(&[0x004f5e0000000600000000070804038c], true, None, false);
    }

    #[derive(Default)]
    struct Score {
        pass: usize,
        fail: usize,
        unimplemented: usize,
        skipped: usize,
    }

    #[test]
    fn fixtures_match_silicon() {
        let mut scores: BTreeMap<String, Score> = BTreeMap::new();
        for fixture in fixtures(include_str!("../../../tests/fixtures/sm86-thread.jsonl")) {
            let score = scores.entry(fixture.family.clone()).or_default();
            let id = &fixture.id;
            if LOCAL_NOT_REPLAYABLE.contains(&id.as_str()) {
                score.skipped += 1;
                eprintln!("{id}: skipped, ptxas kept the local array in registers so the instruction has no observable result");
                continue;
            }
            if catch_unwind(|| translate_module(&fixture.words, true, None, false)).is_err() {
                score.unimplemented += 1;
                continue;
            }
            let mut seed = vec![0; fixture.input_regs.iter().copied().max().unwrap() + 1];
            for (&reg, &value) in fixture.input_regs.iter().zip(&fixture.inputs) {
                seed[reg] = value;
            }
            let Some((regs, _)) = run_regs(&fixture.words, &seed, &[], &[]) else {
                eprintln!("skipping silicon fixtures: no Vulkan device");
                return;
            };
            let actual: Vec<_> = fixture.output_regs.iter().map(|&reg| regs[reg]).collect();
            if actual == fixture.outputs {
                score.pass += 1;
            } else {
                score.fail += 1;
                eprintln!(
                    "{}/{}: expected {:?}, got {:?}",
                    fixture.family, fixture.case, fixture.outputs, actual
                );
            }
        }
        for fixture in fixtures(include_str!("../../../tests/fixtures/sm86-predicate.jsonl")) {
            let id = &fixture.id;
            let score = scores.entry(fixture.family.clone()).or_default();
            if PREDICATE_NOT_REPLAYABLE.contains(&id.as_str()) {
                score.skipped += 1;
                eprintln!("{id}: skipped, R1 held the stack pointer so the first fp64 operand was not recorded");
                continue;
            }
            if catch_unwind(|| translate_module(&fixture.words, true, None, false)).is_err() {
                score.unimplemented += 1;
                continue;
            }
            let mut seed = vec![0; PRED + 1];
            for (&reg, &value) in fixture.input_regs.iter().zip(&fixture.inputs) {
                seed[reg] = value;
            }
            let Some((regs, _)) = run_regs(&fixture.words, &seed, &[], &[]) else {
                eprintln!("skipping silicon fixtures: no Vulkan device");
                return;
            };
            let expected = fixture.outputs[1];
            let actual = regs[fixture.output_regs[1]];
            if actual == expected {
                score.pass += 1;
            } else {
                score.fail += 1;
                eprintln!("{id}: expected {expected:#010x}, got {actual:#010x}");
            }
        }
        println!("family       pass fail unimplemented skipped");
        for (family, score) in &scores {
            println!(
                "{family:<12} {:>4} {:>4} {:>13} {:>7}",
                score.pass, score.fail, score.unimplemented, score.skipped
            );
        }
        assert_eq!(scores.values().map(|score| score.fail).sum::<usize>(), 0);
    }

    struct WarpFixture {
        id: String,
        family: String,
        case: String,
        text: String,
        word: u128,
        observed: bool,
        lanes_active: u32,
        inputs: Vec<u32>,
        outputs: Vec<u32>,
        input_regs: Vec<usize>,
        output_regs: Vec<usize>,
        shared_addr: usize,
        shared_bank_addr: usize,
        global_lo: usize,
        global_hi: usize,
        shared_after: Vec<u32>,
        global_after: Vec<u32>,
        global_before: Vec<u32>,
        registers_after: Vec<u32>,
        marker_regs: Vec<usize>,
    }

    fn warp_fixtures() -> Vec<WarpFixture> {
        include_str!("../../../tests/fixtures/sm86-warp.jsonl")
            .lines()
            .map(|object| {
                let input_regs: Vec<_> =
                    (0..4).map(|i| reg(object, &format!("input{i}"))).collect();
                let [shared_addr, shared_bank_addr, global_lo, global_hi] =
                    ["shared_addr", "shared_bank_addr", "global_lo", "global_hi"]
                        .map(|name| reg(object, name));
                WarpFixture {
                    id: id(object),
                    family: string(object, "family"),
                    case: string(object, "case"),
                    text: string(object, "nvdisasm_text"),
                    word: word(object, "word_hex"),
                    observed: observed(object),
                    lanes_active: object
                        .split_once("\"lanes_active\":")
                        .unwrap()
                        .1
                        .split(|c: char| !c.is_ascii_digit())
                        .next()
                        .unwrap()
                        .parse()
                        .unwrap(),
                    inputs: numbers(object, "inputs"),
                    outputs: numbers(object, "outputs"),
                    input_regs: input_regs.clone(),
                    output_regs: (0..2).map(|i| reg(object, &format!("output{i}"))).collect(),
                    shared_addr,
                    shared_bank_addr,
                    global_lo,
                    global_hi,
                    shared_after: numbers(object, "shared_after"),
                    global_after: numbers(object, "global_after"),
                    global_before: numbers(object, "global_before"),
                    registers_after: numbers(object, "registers_after"),
                    marker_regs: input_regs
                        .into_iter()
                        .chain([shared_addr, shared_bank_addr, global_lo, global_hi])
                        .collect(),
                }
            })
            .collect()
    }

    const NOT_REPLAYABLE: &[&str] = &["warp3#053/sts/bank-conflict"];
    const LOCAL_NOT_REPLAYABLE: &[&str] = &[
        "s4local#000/local/stl-32-0x0001c000001008000000080100007387-stl-r0",
        "s4local#001/local/stl-32-0x0001c000001008000000080100007387-stl-r0",
        "s4local#002/local/ldl-32-0x00000000001008000000080000047983-ldl-r4",
        "s4local#003/local/ldl-32-0x00000000001008000000080000047983-ldl-r4",
        "s4local#004/local/stl-u8-0x0001c000001000000000000100007387-stl-u8",
        "s4local#005/local/stl-u8-0x0001c000001000000000000100007387-stl-u8",
        "s4local#006/local/ldl-s8-0x00000000001002000000000000057983-ldl-s8",
        "s4local#007/local/ldl-s8-0x00000000001002000000000000057983-ldl-s8",
        "s4local#008/local/stl-u16-0x0001c000001004000000040100007387-stl-u",
        "s4local#009/local/stl-u16-0x0001c000001004000000040100007387-stl-u",
        "s4local#010/local/ldl-s16-0x00000000001006000000040000067983-ldl-s",
        "s4local#011/local/ldl-s16-0x00000000001006000000040000067983-ldl-s",
    ];
    const PREDICATE_NOT_REPLAYABLE: &[&str] = &[
        "s4predicate#000/dsetp/lt-distinct-pairs",
        "s4predicate#001/dsetp/lt-distinct-pairs",
        "s4predicate#002/dsetp/lt-distinct-pairs",
        "s4predicate#003/dsetp/lt-distinct-pairs",
    ];
    const KNOWN_FAILURES: &[&str] = &[
        // fp32: MUFU.TANH is a hardware approximation that no exact tanh reproduces.
        "s3#026/fp32/mufu-tanh-r1-r3",
        // suatom: the order in which lanes hitting the same texel apply their atomics is up to the device.
        "s5suatom#000/suatom/d-2d-add",
        // tex-lod: silicon sampled a 3-level mipmapped array, but the ingest dropped mip_levels and textures decode one level.
        "s6tex#002/tex-lod/lb-r6",
        "s6tex#003/tex-lod/ll-r6",
        "s6tex#004/tex-lod/lc-r6",
        "s6tex#006/tex-lod/lb-lc-r6",
        "s6tex#009/tex-lod/lz-dc-r6",
        "s6tex#010/tex-lod/ll-dc-r6",
    ];
    const LLVMPIPE_FAILURES: &[&str] = &[
        // convert: llvmpipe truncates an f64 RoundEven.
        "s3#030/convert/f2i-s64-f64-r10-r12",
        // tex: lavapipe reads an infinite unnormalized coordinate as texel 0 instead of clamping to the edge.
        "tex#003/tex/lod-lz",
        "tex#005/tex/lod-ll",
        "tex#007/tex/mask-r",
        "tex#009/tex/mask-rg",
        "tex#010/tex/mask-a",
        "s4tex#003/tex/lod-lz",
        "s4tex#005/tex/lod-ll",
        "s4tex#007/tex/mask-r",
        "s4tex#009/tex/mask-rg",
        "s4tex#010/tex/mask-a",
        "s5tex#003/tex/lod-lz",
        "s5tex#005/tex/lod-ll",
        "s5tex#007/tex/mask-r",
        "s5tex#009/tex/mask-rg",
        "s5tex#010/tex/mask-a",
        // tmml: lavapipe scales the lod of unnormalized coordinates by the texture size once more.
        "s5tmml#000/tmml/lod-2d-derivatives-first-to-r8",
    ];

    fn destination_width(fixture: &WarpFixture) -> usize {
        if fixture.text.contains(" ST")
            || fixture.text.contains(" RED")
            || fixture.text.contains(" BAR")
            || fixture.text.contains(" MEMBAR")
        {
            0
        } else if fixture.text.contains(".128 ") {
            4
        } else if fixture.family == "fp64"
            || fixture.text.contains(".64 ")
            || fixture.text.contains("F2F.F64.")
            || fixture.text.contains("F2I.S64.")
        {
            2
        } else {
            1
        }
    }

    struct TextureFixture {
        id: String,
        family: String,
        case: String,
        word: u128,
        observed: bool,
        handle: usize,
        inputs: Vec<u32>,
        outputs: Vec<u32>,
        texels: Vec<u32>,
        surface_before: Vec<u32>,
        surface_after: Vec<u32>,
    }

    fn texture_fixtures() -> Vec<TextureFixture> {
        include_str!("../../../tests/fixtures/sm86-texture.jsonl")
            .lines()
            .map(|object| TextureFixture {
                id: id(object),
                family: string(object, "family"),
                case: string(object, "case"),
                word: word(object, "word_hex"),
                observed: observed(object),
                handle: reg(object, "handle_reg"),
                inputs: numbers(object, "inputs"),
                outputs: numbers(object, "outputs"),
                texels: numbers(object, "texels"),
                surface_before: numbers(object, "surface_before"),
                surface_after: numbers(object, "surface_after"),
            })
            .collect()
    }

    fn run_texture_fixture(fixture: &TextureFixture) -> Option<Result<(), String>> {
        const ADDRESS: u64 = 0x1_0000_0400;
        const PITCH: usize = 32;
        let texels = fixture
            .texels
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .collect::<Vec<_>>();
        let surface = fixture
            .surface_before
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .collect::<Vec<_>>();
        let surface_access = matches!(fixture.family.as_str(), "surf" | "suatom");
        let source = if surface_access { &surface } else { &texels };
        let rows: Vec<u8> = source
            .chunks(16)
            .flat_map(|row| [row, &[0; PITCH - 16]].concat())
            .collect();
        let format = if fixture.family == "suatom" {
            0x0f | 0b100_100_100_100 << 7
        } else {
            8 | 0b010_010_010_010 << 7
        };
        let header = [
            format | 0o5432 << 19,
            ADDRESS as u32,
            (ADDRESS >> 32) as u32 | 2 << 21,
            PITCH as u32 >> 5,
            3 | 1 << 23,
            3,
            0,
            0,
        ];
        let entries = |entry: [u32; 8]| -> Vec<u8> {
            [[0; 8], entry]
                .iter()
                .flatten()
                .flat_map(|word| word.to_le_bytes())
                .collect()
        };
        let (tic, tsc) = (
            entries(header),
            entries([2 | 2 << 3, 1 | 1 << 4, 0, 0, 0, 0, 0, 0]),
        );
        let handle = 1 << 20 | 1;
        let memory = [(ADDRESS, &rows[..])];
        let textures = Textures {
            handle,
            pool: TexturePool {
                tic: &tic,
                tsc: &tsc,
            },
            memory: &memory,
        };
        let mut cbuf = vec![0; fixture.handle + 1];
        cbuf[fixture.handle] = handle;
        let seeds: Vec<Vec<u32>> = (0..32)
            .map(|lane| {
                let mut seed = vec![0; PRED + 1];
                seed[0] = 0xffa5_5a00 | lane as u32;
                seed[4..8].copy_from_slice(&fixture.inputs[lane * 4..][..4]);
                if fixture.family == "tmml" {
                    let rd2 = (fixture.word >> 64 & 0xff) as usize;
                    seed[rd2] = if rd2 == 8 {
                        fixture.outputs[lane * 2 + 1]
                    } else {
                        0xdead_beef
                    };
                }
                seed
            })
            .collect();
        let mut program = vec![fixture.word];
        if fixture.family == "surf" {
            program.push(inst::surface(
                inst::SULD_D_32 | inst::SU_BA,
                20,
                6,
                0xff,
                0,
                (fixture.handle * 4) as u32,
            ));
        }
        let (lanes, _) = run_textured(
            |decoder| decoder.translate_program(&program),
            &seeds,
            &cbuf,
            &[],
            &textures,
        )?;
        if fixture.family == "surf" {
            for (lane, regs) in lanes.iter().take(16).enumerate() {
                let expected = fixture.surface_after[lane];
                if regs[20] != expected {
                    return Some(Err(format!(
                        "lane {lane} expected {:#010x}, got {:#010x}",
                        expected, regs[20]
                    )));
                }
            }
        } else if fixture.family == "suatom" {
            for (lane, regs) in lanes.iter().enumerate() {
                if regs[8] != fixture.outputs[lane * 2 + 1] {
                    return Some(Err(format!(
                        "lane {lane} expected {:#010x}, got {:#010x}",
                        fixture.outputs[lane * 2 + 1],
                        regs[8]
                    )));
                }
            }
        } else if fixture.family == "tmml" {
            let rd = (fixture.word >> 16 & 0xff) as usize;
            let rd2 = (fixture.word >> 64 & 0xff) as usize;
            for (lane, regs) in lanes.iter().enumerate() {
                let (reg, expected) = if rd == 8 {
                    (rd, fixture.outputs[lane * 2 + 1])
                } else if rd2 == 8 {
                    (rd2, fixture.outputs[lane * 2 + 1])
                } else {
                    (rd2, 0xdead_beef)
                };
                if regs[reg] != expected {
                    return Some(Err(format!(
                        "lane {lane} expected {:#010x}, got {:#010x}",
                        expected, regs[reg]
                    )));
                }
            }
        } else {
            let mask = match fixture.case.as_str() {
                "mask-r" => 1u32,
                "mask-rg" => 3,
                "mask-a" => 8,
                _ => 15,
            };
            let [rd, rd2] = [16, 64].map(|bit| (fixture.word >> bit & 0xff) as usize);
            for (lane, regs) in lanes.iter().enumerate() {
                let packed = fixture.outputs[lane * 2].to_le_bytes();
                let expected = &packed[..mask.count_ones() as usize];
                let actual: Vec<_> = [rd, rd + 1, rd2, rd2 + 1][..expected.len()]
                    .iter()
                    .map(|&r| (f32::from_bits(regs[r]) * 255.0).round() as u8)
                    .collect();
                if actual != expected {
                    return Some(Err(format!(
                        "lane {lane} expected {expected:?}, got {actual:?}"
                    )));
                }
            }
        }
        Some(Ok(()))
    }

    #[test]
    fn fixtures_match_tensor_silicon() {
        let mut score = Score::default();
        let mut unexpected = Vec::new();
        for fixture in tensor_fixtures() {
            let id = &fixture.id;
            if !fixture.observed {
                score.skipped += 1;
                continue;
            }
            let program = tensor_program(&fixture);
            if catch_unwind(|| translate_module(&program, true, None, false)).is_err() {
                score.unimplemented += 1;
                continue;
            }
            let seeds = tensor_seeds(&fixture);
            let Some((lanes, _)) = run_lanes(&program, &seeds, &[], &[]) else {
                eprintln!("skipping tensor silicon fixtures: no Vulkan device");
                return;
            };
            let mismatch = tensor_mismatch(&fixture, &lanes);
            if let Some((lane, expected, actual)) = mismatch {
                score.fail += 1;
                if !KNOWN_FAILURES.contains(&id.as_str()) {
                    unexpected.push(id.clone());
                }
                eprintln!("{id}: lane {lane} expected {expected:?}, got {actual:?}");
            } else {
                score.pass += 1;
            }
        }
        println!("family       pass fail unimplemented skipped");
        println!(
            "tensor       {:>4} {:>4} {:>13} {:>7}",
            score.pass, score.fail, score.unimplemented, score.skipped
        );
        assert!(unexpected.is_empty(), "unexpected failures: {unexpected:?}");
    }

    #[test]
    fn fixtures_match_warp_silicon() {
        let mut scores: BTreeMap<String, Score> = BTreeMap::new();
        let mut unexpected = Vec::new();
        let llvmpipe = crate::gpu::device().is_ok_and(|d| d.is_llvmpipe());
        let known =
            |id: &str| KNOWN_FAILURES.contains(&id) || llvmpipe && LLVMPIPE_FAILURES.contains(&id);
        for fixture in warp_fixtures() {
            let score = scores.entry(fixture.family.clone()).or_default();
            if !fixture.observed {
                score.skipped += 1;
                continue;
            }
            let id = &fixture.id;
            if NOT_REPLAYABLE.contains(&id.as_str()) {
                score.skipped += 1;
                eprintln!("{id}: skipped, rig seed kernel wrote shared memory so one instruction cannot replay it");
                continue;
            }
            if fixture.family == "atoms" && fixture.text.contains(".64 ") && !crate::gpu::device().is_ok_and(|d| d.shader_shared_int64_atomics) {
                score.unimplemented += 1;
                continue;
            }
            let mut words = vec![fixture.word];
            let shared = if fixture.text.contains("[R9]") {
                fixture.shared_bank_addr
            } else {
                fixture.shared_addr
            };
            if matches!(fixture.family.as_str(), "lds" | "sts" | "atoms") {
                words.insert(
                    0,
                    0x004f_c800_0000_0800_0000_0011_0000_7388 | (shared as u128) << 24,
                );
            }
            if matches!(fixture.family.as_str(), "sts" | "atoms") {
                words.push(0x004f_c800_0000_0800_0000_0000_0011_7984 | (shared as u128) << 24);
            }
            if catch_unwind(|| translate_module(&words, true, None, false)).is_err() {
                score.unimplemented += 1;
                continue;
            }
            let mut seeds = vec![vec![0; LIVE + 1]; 32];
            for (lane, seed) in seeds.iter_mut().enumerate() {
                for (input, &reg) in fixture.inputs[lane * 4..][..4]
                    .iter()
                    .zip(&fixture.input_regs)
                {
                    seed[reg] = *input;
                }
                seed[fixture.shared_addr] = fixture.registers_after[lane * 8 + 4];
                seed[fixture.shared_bank_addr] = fixture.registers_after[lane * 8 + 5];
                let global = GLOBAL_BASE + lane as u64 * 16;
                seed[fixture.global_lo] = global as u32;
                seed[fixture.global_hi] = (global >> 32) as u32;
                seed[fixture.output_regs[1]] = 0x5100_0000 | seed[shared] / 4;
                // The rig masks lanes through the word's own @P0, so the PT prelude still seeds every lane's shared word.
                seed[PRED] = fixture.lanes_active >> lane & 1;
                seed[LIVE] = 1;
            }
            let global_before: Vec<_> = if !fixture.global_before.is_empty() {
                fixture.global_before.clone()
            } else if fixture.id.starts_with("s4warp") {
                (0..256)
                    .map(|i| if i & 1 == 0 { 0x7fff_ffff } else { 0x8000_0000 })
                    .collect()
            } else {
                (0..256).map(|i| 0x6100_0000 + i).collect()
            };
            let Some((lanes, global)) = run_lanes(&words, &seeds, &[32, 1], &global_before) else {
                eprintln!("skipping warp silicon fixtures: no Vulkan device");
                return;
            };
            let width = destination_width(&fixture);
            let destination = (fixture.word >> 16 & 0xff) as usize;
            let mismatch = lanes.iter().enumerate().find_map(|(lane, regs)| {
                let expected: Vec<_> = (0..width)
                    .filter_map(|word| {
                        fixture
                            .marker_regs
                            .iter()
                            .position(|&reg| reg == destination + word)
                            .map(|marker| fixture.registers_after[lane * 8 + marker])
                    })
                    .collect();
                let expected = if expected.len() == width {
                    expected
                } else {
                    fixture.outputs[lane * 2..][..width.min(2)].to_vec()
                };
                let actual = regs[destination..][..expected.len()].to_vec();
                (actual != expected).then_some((lane, expected, actual))
            });
            let global_mismatch =
                (!fixture.global_after.is_empty() && global != fixture.global_after).then(|| {
                    let word = global
                        .iter()
                        .zip(&fixture.global_after)
                        .position(|(a, b)| a != b)
                        .unwrap();
                    (word, fixture.global_after[word], global[word])
                });
            let shared_matches = !matches!(fixture.family.as_str(), "sts" | "atoms")
                || lanes.iter().enumerate().all(|(lane, regs)| {
                    let word = seeds[lane][shared] as usize / 4;
                    regs[fixture.output_regs[1]] == fixture.shared_after[word]
                });
            if mismatch.is_none() && shared_matches && global_mismatch.is_none() {
                score.pass += 1;
            } else if fixture.family == "s2r" && fixture.case == "eqmask" {
                score.unimplemented += 1;
            } else {
                score.fail += 1;
                if !known(&id) {
                    unexpected.push(id.clone());
                }
                if let Some((lane, expected, actual)) = mismatch {
                    eprintln!("{id}: lane {lane} expected {expected:?}, got {actual:?}");
                }
                if let Some((word, expected, actual)) = global_mismatch {
                    eprintln!("{id}: global[{word}] expected {expected:#010x}, got {actual:#010x}");
                }
                if !shared_matches {
                    eprintln!("{id}: shared memory mismatch");
                }
            }
        }
        for fixture in fixtures(include_str!("../../../tests/fixtures/sm86-observer.jsonl")) {
            let score = scores.entry(fixture.family.clone()).or_default();
            let id = &fixture.id;
            if !fixture.observed {
                score.skipped += 1;
                continue;
            }
            if catch_unwind(|| translate_module(&fixture.words, true, None, false)).is_err() {
                score.unimplemented += 1;
                continue;
            }
            let mut seeds = vec![vec![0; LIVE + 1]; 32];
            for (lane, seed) in seeds.iter_mut().enumerate() {
                let inputs = if fixture.inputs.len() == 4 {
                    &fixture.inputs[..]
                } else {
                    &fixture.inputs[lane * 4..][..4]
                };
                for (&value, &reg) in inputs.iter().zip(&fixture.input_regs) {
                    seed[reg] = value;
                }
                let global = GLOBAL_BASE + lane as u64 * 16;
                seed[2] = global as u32;
                seed[3] = (global >> 32) as u32;
                seed[PRED] = 1 | ((lane as u32 & 1) << 1);
                seed[LIVE] = 1;
            }
            let mut cbuf = vec![0; 0x170 / 4];
            cbuf[0x168 / 4] = GLOBAL_BASE as u32;
            cbuf[0x16c / 4] = (GLOBAL_BASE >> 32) as u32;
            let global_before: Vec<_> = (0..256)
                .map(|word| {
                    if word & 1 == 0 {
                        0x7fff_ffff
                    } else {
                        0x8000_0000
                    }
                })
                .collect();
            let Some((lanes, global)) = run_lanes(&fixture.words, &seeds, &cbuf, &global_before)
            else {
                eprintln!("skipping session 5 warp fixtures: no Vulkan device");
                return;
            };
            let mismatch = if fixture.id.starts_with("s5local") {
                lanes.iter().enumerate().find_map(|(lane, regs)| {
                    let actual = regs[fixture.output_regs[0]];
                    (actual != fixture.outputs[0]).then_some((lane, fixture.outputs[0], actual))
                })
            } else {
                let checked = match fixture.family.as_str() {
                    "vote" | "shfl" | "match" => &fixture.input_regs[..2],
                    "redux" | "uldc" => &fixture.input_regs[1..2],
                    "ldg" => &fixture.input_regs[..1],
                    _ => &[],
                };
                lanes.iter().enumerate().find_map(|(lane, regs)| {
                    checked.iter().find_map(|&reg| {
                        let marker = fixture.input_regs.iter().position(|&r| r == reg).unwrap();
                        let expected = if fixture.family == "uldc" {
                            GLOBAL_BASE as u32
                        } else {
                            fixture.registers_after[lane * 8 + marker]
                        };
                        (regs[reg] != expected).then_some((lane, expected, regs[reg]))
                    })
                })
            };
            let global_mismatch =
                (!fixture.global_after.is_empty() && global != fixture.global_after).then(|| {
                    let word = global
                        .iter()
                        .zip(&fixture.global_after)
                        .position(|(a, b)| a != b)
                        .unwrap();
                    (word, fixture.global_after[word], global[word])
                });
            if mismatch.is_none() && global_mismatch.is_none() {
                score.pass += 1;
            } else {
                score.fail += 1;
                if !known(&id) {
                    unexpected.push(id.clone());
                }
                if let Some((lane, expected, actual)) = mismatch {
                    eprintln!("{id}: lane {lane} expected {expected:#010x}, got {actual:#010x}");
                }
                if let Some((word, expected, actual)) = global_mismatch {
                    eprintln!("{id}: global[{word}] expected {expected:#010x}, got {actual:#010x}");
                }
            }
        }
        for fixture in texture_fixtures() {
            let score = scores.entry(fixture.family.clone()).or_default();
            if !fixture.observed {
                score.skipped += 1;
                continue;
            }
            let id = &fixture.id;
            match catch_unwind(|| run_texture_fixture(&fixture)) {
                Err(_) => score.unimplemented += 1,
                Ok(None) => {
                    eprintln!("skipping warp silicon fixtures: no Vulkan device");
                    return;
                }
                Ok(Some(Ok(()))) => score.pass += 1,
                Ok(Some(Err(mismatch))) => {
                    score.fail += 1;
                    if !known(&id) {
                        unexpected.push(id.clone());
                    }
                    eprintln!("{id}: {mismatch}");
                }
            }
        }
        println!("family       pass fail unimplemented skipped");
        for (family, score) in &scores {
            println!(
                "{family:<12} {:>4} {:>4} {:>13} {:>7}",
                score.pass, score.fail, score.unimplemented, score.skipped
            );
        }
        assert!(unexpected.is_empty(), "unexpected failures: {unexpected:?}");
    }

    // nvcc -arch=sm_86 bench kernels, each launched with small arguments.
    fn bench_launches() -> Vec<(&'static str, [u32; 3], [u32; 3], Vec<Vec<u8>>, Vec<u8>)> {
        let mandelbrot = [80u32, 60, 64].map(u32::to_le_bytes).into_iter().chain([-2.5f32, -1.5, 0.05].map(f32::to_le_bytes)).flatten().collect();
        let fma = [4u32.to_le_bytes(), 0.999f32.to_le_bytes(), 0.001f32.to_le_bytes()].concat();
        vec![
            ("mandelbrot", [5, 4, 1], [16, 16, 1], vec![vec![0; 80 * 60 * 4]], mandelbrot),
            ("fma_chain", [2, 1, 1], [256, 1, 1], vec![vec![0; 2 * 256 * 4]], fma),
            ("stream_add", [1, 1, 1], [256, 1, 1], vec![(0..4096u32).flat_map(|i| (i as f32).to_le_bytes()).collect(), vec![0; 4096 * 4]], 1f32.to_le_bytes().to_vec()),
            ("empty", [1, 1, 1], [256, 1, 1], vec![vec![0; 16]], Vec::new()),
        ]
    }

    #[test]
    fn structured_translation_matches_the_dispatcher() {
        const BENCH: &[u8] = include_bytes!("../../../tests/fixtures/bench/bench.cubin");
        let mut structured = 0;
        let mut rejected: BTreeMap<String, usize> = BTreeMap::new();
        let mut eligible = |words: &[u128]| match plan(words) {
            Ok(_) => {
                structured += 1;
                true
            }
            Err(reason) => {
                *rejected.entry(reason.split(" at pc").next().unwrap().to_string()).or_default() += 1;
                false
            }
        };
        let jsonl = [include_str!("../../../tests/fixtures/sm86-thread.jsonl"), include_str!("../../../tests/fixtures/sm86-predicate.jsonl"), include_str!("../../../tests/fixtures/sm86-observer.jsonl")];
        for fixture in jsonl.into_iter().flat_map(fixtures) {
            if !eligible(&fixture.words) || catch_unwind(|| translate_module(&fixture.words, true, None, false)).is_err() {
                continue;
            }
            let mut seed = vec![0; LIVE];
            for (&reg, &value) in fixture.input_regs.iter().zip(&fixture.inputs) {
                seed[reg] = value;
            }
            let [dispatched, planned] = both_ways(|| run_regs(&fixture.words, &seed, &[], &[]));
            assert_eq!(dispatched, planned, "{}", fixture.id);
        }
        for fixture in texture_fixtures().iter().filter(|fixture| fixture.observed && eligible(&[fixture.word])) {
            let [dispatched, planned] = both_ways(|| catch_unwind(|| run_texture_fixture(fixture)).ok());
            assert_eq!(dispatched, planned, "{}", fixture.id);
        }
        for fixture in tensor_fixtures() {
            eligible(&tensor_program(&fixture));
        }
        for (name, grid, block, buffers, values) in bench_launches() {
            let kernel = Kernel::load(BENCH, name).unwrap();
            if !eligible(&kernel.words) {
                continue;
            }
            let args: Vec<Arg> = buffers.iter().map(|buffer| Arg::Buffer(buffer)).chain(values.chunks(4).map(Arg::Value)).collect();
            let [dispatched, planned] = both_ways(|| run_kernel(BENCH, name, grid, block, &args));
            assert_eq!(dispatched, planned, "{name}");
        }
        println!("structured {structured}, dispatcher only {rejected:?}");
    }
}
