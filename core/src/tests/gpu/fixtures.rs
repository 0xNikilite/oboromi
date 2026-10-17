#[cfg(test)]
mod tests {
    use crate::gpu::texture::TexturePool;
    use crate::tests::gpu::harness::{
        run_lanes, run_regs, run_textured, translate_module, Textures, GLOBAL_BASE, LIVE, PRED,
    };
    use crate::tests::gpu_test::inst;
    use std::collections::BTreeMap;
    use std::panic::catch_unwind;

    struct Fixture {
        family: String,
        case: String,
        words: Vec<u128>,
        inputs: Vec<u32>,
        outputs: Vec<u32>,
        input_regs: Vec<usize>,
        output_regs: Vec<usize>,
    }

    fn string(object: &str, key: &str) -> String {
        let value = object
            .split_once(&format!("\"{key}\":"))
            .unwrap()
            .1
            .trim_start();
        let value = value.strip_prefix('"').unwrap();
        value[..value.find('"').unwrap()].to_owned()
    }

    fn numbers(object: &str, key: &str) -> Vec<u32> {
        let value = object.split_once(&format!("\"{key}\":")).unwrap().1;
        let value = value.split_once('[').unwrap().1.split_once(']').unwrap().0;
        value
            .split(',')
            .filter_map(|n| n.trim().parse().ok())
            .collect()
    }

    fn nested_numbers(object: &str, key: &str) -> Vec<u32> {
        let value = object.split_once(&format!("\"{key}\":")).unwrap().1;
        let start = value.find('[').unwrap();
        let mut depth = 0;
        let end = value[start..]
            .char_indices()
            .find_map(|(i, c)| {
                depth += (c == '[') as usize;
                depth -= (c == ']') as usize;
                (depth == 0).then_some(start + i)
            })
            .unwrap();
        value[start..=end]
            .split(|c: char| !c.is_ascii_digit())
            .filter_map(|n| n.parse().ok())
            .collect()
    }

    fn number(object: &str, key: &str) -> u32 {
        object
            .split_once(&format!("\"{key}\":"))
            .unwrap()
            .1
            .trim_start()
            .split(|c: char| !c.is_ascii_digit())
            .next()
            .unwrap()
            .parse()
            .unwrap()
    }

    fn objects(json: &'static str) -> impl Iterator<Item = &'static str> {
        json.split("\n  },\n  {")
    }

    fn fixtures() -> Vec<Fixture> {
        objects(include_str!("../../../tests/fixtures/sm86-silicon.json"))
            .map(|object| {
                let reg = |name: String| string(object, &name)[1..].parse().unwrap();
                let mut words =
                    vec![u128::from_str_radix(&string(object, "word_hex"), 16).unwrap()];
                if object.contains("\"observer_word_hex\"") {
                    words.push(
                        u128::from_str_radix(&string(object, "observer_word_hex"), 16).unwrap(),
                    );
                }
                Fixture {
                    family: string(object, "family"),
                    case: string(object, "case"),
                    words,
                    inputs: numbers(object, "inputs"),
                    outputs: numbers(object, "outputs"),
                    input_regs: (0..4).map(|i| reg(format!("input{i}"))).collect(),
                    output_regs: (0..2).map(|i| reg(format!("output{i}"))).collect(),
                }
            })
            .collect()
    }

    fn predicate_fixtures() -> Vec<Fixture> {
        [
            include_str!("../../../tests/fixtures/sm86-s3-predicate-silicon.json"),
            include_str!("../../../tests/fixtures/sm86-s3-predicate-notpt-silicon.json"),
        ]
        .into_iter()
        .flat_map(objects)
        .map(|object| {
            let reg = |name: String| string(object, &name)[1..].parse().unwrap();
            Fixture {
                family: string(object, "family"),
                case: string(object, "case"),
                words: ["word_hex", "observer_word_hex"]
                    .map(|key| u128::from_str_radix(&string(object, key), 16).unwrap())
                    .to_vec(),
                inputs: numbers(object, "inputs"),
                outputs: numbers(object, "outputs"),
                input_regs: (0..4).map(|i| reg(format!("input{i}"))).collect(),
                output_regs: (0..2).map(|i| reg(format!("output{i}"))).collect(),
            }
        })
        .collect()
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
        for fixture in fixtures() {
            let score = scores.entry(fixture.family.clone()).or_default();
            if catch_unwind(|| translate_module(&fixture.words, true, None)).is_err() {
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
        let mut vectors = BTreeMap::new();
        for fixture in predicate_fixtures() {
            let vector = vectors.entry(fixture.case.clone()).or_insert(0);
            let id = format!("predicate/{}/{}", fixture.case, *vector);
            *vector += 1;
            let score = scores.entry(fixture.family.clone()).or_default();
            if catch_unwind(|| translate_module(&fixture.words, true, None)).is_err() {
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
        println!("family       pass fail unimplemented");
        for (family, score) in &scores {
            println!(
                "{family:<12} {:>4} {:>4} {:>13}",
                score.pass, score.fail, score.unimplemented
            );
        }
        assert_eq!(scores.values().map(|score| score.fail).sum::<usize>(), 0);
    }

    struct WarpFixture {
        source: &'static str,
        index: usize,
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
        registers_after: Vec<u32>,
        marker_regs: Vec<usize>,
    }

    fn warp_fixtures() -> Vec<WarpFixture> {
        [
            (
                "warp3",
                include_str!("../../../tests/fixtures/sm86-warp3-silicon.json"),
            ),
            (
                "seg16",
                include_str!("../../../tests/fixtures/sm86-warp3-seg16-silicon.json"),
            ),
            (
                "s3",
                include_str!("../../../tests/fixtures/sm86-s3-silicon.json"),
            ),
        ]
        .into_iter()
        .flat_map(|(source, json)| {
            objects(json)
                .enumerate()
                .map(move |(index, object)| (source, index, object))
        })
        .map(|object| {
            let (source, index, object) = object;
            let reg = |name: &str| string(object, name)[1..].parse().unwrap();
            let input_regs: Vec<_> = (0..4).map(|i| reg(&format!("input{i}"))).collect();
            let shared_addr = reg("shared_addr");
            let shared_bank_addr = reg("shared_bank_addr");
            let global_lo = reg("global_lo");
            let global_hi = reg("global_hi");
            WarpFixture {
                source,
                index,
                family: string(object, "family"),
                case: string(object, "case"),
                text: string(object, "nvdisasm_text"),
                word: u128::from_str_radix(&string(object, "word_hex"), 16).unwrap(),
                observed: object.contains("\"observed\": true"),
                lanes_active: number(object, "lanes_active"),
                inputs: nested_numbers(object, "inputs"),
                outputs: if object.contains("\"outputs\"") {
                    nested_numbers(object, "outputs")
                } else {
                    Vec::new()
                },
                input_regs: input_regs.clone(),
                output_regs: (0..2).map(|i| reg(&format!("output{i}"))).collect(),
                shared_addr,
                shared_bank_addr,
                global_lo,
                global_hi,
                shared_after: if object.contains("\"shared_after\"") {
                    nested_numbers(object, "shared_after")
                } else {
                    Vec::new()
                },
                global_after: if object.contains("\"global_after\"") {
                    nested_numbers(object, "global_after")
                } else {
                    Vec::new()
                },
                registers_after: if object.contains("\"registers_after\"") {
                    nested_numbers(object, "registers_after")
                } else {
                    Vec::new()
                },
                marker_regs: input_regs
                    .into_iter()
                    .chain([shared_addr, shared_bank_addr, global_lo, global_hi])
                    .collect(),
            }
        })
        .collect()
    }

    const NOT_REPLAYABLE: &[&str] = &[
        "s3#067/fp16/hadd2-r15-r4-f32-r0",
        "s3#097/memory/stl-r0-0x8-r1",
        "s3#098/memory/ldl-r4-r0-0x8",
        "s3#099/memory/stl-u8-r0-r1",
        "s3#100/memory/ldl-s8-r5-r0",
        "s3#101/memory/stl-u16-r0-0x4-r1",
        "s3#102/memory/ldl-s16-r6-r0-0x4",
        "warp3#053/sts/bank-conflict",
    ];
    const LIVE_MASK_SUPPORTED: bool = false;
    const KNOWN_FAILURES: &[&str] = &[
        "s3#011/fp32/fadd-r1-r2-r3",
        "s3#012/fp32/fadd-r1-r2-r3",
        "s3#014/fp32/fadd-r1-r2-0-5",
        "s3#015/fp32/fmul-r1-r2-r3",
        "s3#016/fp32/fmul-fmz-r1-r2-r3",
        "s3#018/fp32/fmul-r1-r2-3",
        "s3#019/fp32/ffma-r1-r2-r3-r4",
        "s3#020/fp32/ffma-fmz-r1-r2-r3-r4",
        "s3#021/fp32/ffma-r1-r2-2-r4",
        "s3#022/fp32/ffma-r1-r2-r3-0-5",
        "s3#023/fp32/fmnmx-r1-r2-r3-pt",
        "s3#024/fp32/fmnmx-r1-r2-r3-p0",
        "s3#025/fp32/mufu-rcp-r1-r3",
        "s3#026/fp32/mufu-tanh-r1-r3",
        "s3#028/convert/f2f-f32-f64-rp-r13-r14",
        "s3#029/convert/f2i-r1-r3",
        "s3#030/convert/f2i-s64-f64-r10-r12",
        "s3#031/convert/frnd-r4-r5",
        "s3#035/fp64/dadd-r32-r0-r2",
        "s3#037/fp64/dadd-r36-r0-c-0x0-0x0",
        "s3#038/fp64/dadd-r38-r6-r6",
        "s3#039/fp64/dadd-r40-r8-r10",
        "s3#040/fp64/dadd-r42-r12-r0",
        "s3#041/fp64/dadd-r58-r0-r2",
        "s3#042/fp64/dmul-r44-r0-r2",
        "s3#043/fp64/dmul-r46-r0-2",
        "s3#044/fp64/dfma-r48-r0-r2-r4",
        "s3#045/fp64/dfma-r50-r0-2-r4",
        "s3#046/fp64/dfma-r52-r0-r2-2",
        "s3#047/fp64/dfma-r54-r0-c-0x0-0x0-r4",
        "s3#048/fp64/dfma-r56-r0-r2-c-0x0-0x0",
        "s3#049/fp64/dfma-r60-r0-r2-r4",
        "s3#062/fp16/hadd2-r10-r0-r1",
        "s3#063/fp16/hadd2-r11-r0-h0-h0-r1-h1-h1",
        "s3#064/fp16/hadd2-r12-r2-r3",
        "s3#065/fp16/hadd2-r13-r2-h0-h0-r2-h0-h0",
        "s3#066/fp16/hadd2-f32-r14-rz-r0-h1-h1",
        "s3#068/fp16/hadd2-r16-r0-0-5-1",
        "s3#069/fp16/hmul2-r17-r0-r5",
        "s3#070/fp16/hmul2-r18-r2-r2",
        "s3#071/fp16/hmul2-fmz-r19-r2-r2-h0-h0",
        "s3#072/fp16/hmul2-ftz-r25-r7-r7",
        "s3#073/fp16/hfma2-r20-r0-r1-r5",
        "s3#075/fp16/hfma2-relu-r22-r0-r1-r5",
        "s3#076/fp16/hfma2-r23-r0-2-0-5-r5",
        "s3#077/fp16/hfma2-r26-r0-r1-1-4",
        "s3#078/fp16/hfma2-mma-r24-r0-r1-r5",
        "s3#079/fp16/hset2-bf-lt-and-r10-r0-r5-pt",
        "s3#080/fp16/hset2-ge-and-r11-r0-r5-pt",
        "s3#081/fp16/hset2-bf-neu-and-r12-r3-r3-pt",
        "s3#082/fp16/hmnmx2-r13-r0-r5-pt",
        "s3#083/fp16/hmnmx2-r14-r0-r5-pt",
        "s3#084/fp16/hmnmx2-r15-r3-r0-pt",
        "s3#085/fp16/hmnmx2-nan-r16-r3-r0-pt",
        "s3#086/fp16/f2fp-pack-ab-r20-r6-r7",
        "s3#087/fp16/f2fp-pack-ab-r21-r8-r9",
        "s3#088/fp16/f2fp-merge-c-r22-r7-r0-h1",
        "s3#091/fp16/f2ip-u8-f32-r25-r6-r15-r10",
        "tex#003/tex/lod-lz",
        "tex#005/tex/lod-ll",
        "tex#007/tex/mask-r",
        "tex#009/tex/mask-rg",
        "tex#010/tex/mask-a",
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
        objects(include_str!(
            "../../../tests/fixtures/sm86-texture-silicon.json"
        ))
        .map(|object| TextureFixture {
            family: string(object, "family"),
            case: string(object, "case"),
            word: u128::from_str_radix(&string(object, "word_hex"), 16).unwrap(),
            observed: object.contains("\"observed\": true"),
            handle: string(object, "handle_reg")[1..].parse().unwrap(),
            inputs: nested_numbers(object, "inputs"),
            outputs: if object.contains("\"outputs\"") {
                nested_numbers(object, "outputs")
            } else {
                Vec::new()
            },
            texels: numbers(object, "texels"),
            surface_before: numbers(object, "surface_before"),
            surface_after: if object.contains("\"surface_after\"") {
                numbers(object, "surface_after")
            } else {
                Vec::new()
            },
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
        let source = if fixture.family == "surf" {
            &surface
        } else {
            &texels
        };
        let rows: Vec<u8> = source
            .chunks(16)
            .flat_map(|row| [row, &[0; PITCH - 16]].concat())
            .collect();
        let header = [
            8 | 0b010_010_010_010 << 7 | 0o5432 << 19,
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
                seed
            })
            .collect();
        let mut program = vec![fixture.word];
        if fixture.family == "surf" {
            program.push(inst::surface(
                inst::SULD_D_32 | inst::SU_BA,
                8,
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
                if regs[8] != fixture.surface_after[lane] {
                    return Some(Err(format!(
                        "lane {lane} expected {:#010x}, got {:#010x}",
                        fixture.surface_after[lane], regs[8]
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
    fn fixtures_match_warp_silicon() {
        let mut scores: BTreeMap<String, Score> = BTreeMap::new();
        let mut unexpected = Vec::new();
        for fixture in warp_fixtures() {
            let score = scores.entry(fixture.family.clone()).or_default();
            if !fixture.observed || fixture.lanes_active != u32::MAX && !LIVE_MASK_SUPPORTED {
                score.skipped += 1;
                continue;
            }
            let id = format!(
                "{}#{:03}/{}/{}",
                fixture.source, fixture.index, fixture.family, fixture.case
            );
            if NOT_REPLAYABLE.contains(&id.as_str()) {
                score.skipped += 1;
                eprintln!("{id}: skipped, rig seed kernel wrote shared memory so one instruction cannot replay it");
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
                    0x004f_c800_0000_0800_0000_0011_0000_0388 | (shared as u128) << 24,
                );
            }
            if matches!(fixture.family.as_str(), "sts" | "atoms") {
                words.push(0x004f_c800_0000_0800_0000_0000_0011_0984 | (shared as u128) << 24);
            }
            if catch_unwind(|| translate_module(&words, true, None)).is_err() {
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
                seed[PRED] = 1;
                seed[LIVE] = fixture.lanes_active >> lane & 1;
            }
            let global_before: Vec<_> = (0..256).map(|i| 0x6100_0000 + i).collect();
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
                if !KNOWN_FAILURES.contains(&id.as_str()) {
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
        for (index, fixture) in texture_fixtures().into_iter().enumerate() {
            let score = scores.entry(fixture.family.clone()).or_default();
            if !fixture.observed {
                score.skipped += 1;
                continue;
            }
            let id = format!("tex#{index:03}/{}/{}", fixture.family, fixture.case);
            match catch_unwind(|| run_texture_fixture(&fixture)) {
                Err(_) => score.unimplemented += 1,
                Ok(None) => {
                    eprintln!("skipping warp silicon fixtures: no Vulkan device");
                    return;
                }
                Ok(Some(Ok(()))) => score.pass += 1,
                Ok(Some(Err(mismatch))) => {
                    score.fail += 1;
                    if !KNOWN_FAILURES.contains(&id.as_str()) {
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
}
