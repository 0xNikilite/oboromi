#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::{run_lanes, run_regs, translate_module, GLOBAL_BASE, LIVE, PRED};
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

    fn fixtures() -> Vec<Fixture> {
        include_str!("../../../tests/fixtures/sm86-silicon.json")
            .split("\n  },\n  {")
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
        family: String,
        case: String,
        word: u128,
        observed: bool,
        lanes_active: u32,
        inputs: Vec<u32>,
        outputs: Vec<u32>,
        input_regs: Vec<usize>,
        output_regs: Vec<usize>,
        shared_addr: usize,
        global_lo: usize,
        global_hi: usize,
        shared_after: Vec<u32>,
        global_after: Vec<u32>,
    }

    fn warp_fixtures() -> Vec<WarpFixture> {
        include_str!("../../../tests/fixtures/sm86-warp-silicon.json")
            .split("\n  },\n  {")
            .map(|object| {
                let reg = |name: &str| string(object, name)[1..].parse().unwrap();
                WarpFixture {
                    family: string(object, "family"),
                    case: string(object, "case"),
                    word: u128::from_str_radix(&string(object, "word_hex"), 16).unwrap(),
                    observed: object.contains("\"observed\": true"),
                    lanes_active: number(object, "lanes_active"),
                    inputs: nested_numbers(object, "inputs"),
                    outputs: if object.contains("\"outputs\"") {
                        nested_numbers(object, "outputs")
                    } else {
                        Vec::new()
                    },
                    input_regs: (0..4).map(|i| reg(&format!("input{i}"))).collect(),
                    output_regs: (0..2).map(|i| reg(&format!("output{i}"))).collect(),
                    shared_addr: reg("shared_addr"),
                    global_lo: reg("global_lo"),
                    global_hi: reg("global_hi"),
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
                }
            })
            .collect()
    }

    const NOT_REPLAYABLE: &[&str] = &["sts/lane-address"];
    const LIVE_MASK_SUPPORTED: bool = false;

    #[test]
    fn fixtures_match_warp_silicon() {
        let mut scores: BTreeMap<String, Score> = BTreeMap::new();
        for fixture in warp_fixtures() {
            let score = scores.entry(fixture.family.clone()).or_default();
            if !fixture.observed || fixture.lanes_active != u32::MAX && !LIVE_MASK_SUPPORTED {
                score.skipped += 1;
                continue;
            }
            let id = format!("{}/{}", fixture.family, fixture.case);
            if NOT_REPLAYABLE.contains(&id.as_str()) {
                score.skipped += 1;
                eprintln!("{id}: skipped, rig seed kernel wrote shared memory so one instruction cannot replay it");
                continue;
            }
            let mut words = vec![fixture.word];
            if fixture.family == "lds" {
                words.insert(
                    0,
                    0x004f_c800_0000_0800_0000_0007_0f00_0388 & !(0xff << 32)
                        | (fixture.input_regs[0] as u128) << 32,
                );
            } else if fixture.family == "sts" {
                words.push(
                    0x004f_c800_0000_0800_0000_0000_0f04_0984 & !(0xff << 16)
                        | (fixture.output_regs[1] as u128) << 16,
                );
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
                seed[fixture.shared_addr] = fixture.inputs[lane * 4 + 1];
                seed[fixture.global_lo] = GLOBAL_BASE as u32;
                seed[fixture.global_hi] = (GLOBAL_BASE >> 32) as u32;
                seed[PRED] = 1;
                seed[LIVE] = fixture.lanes_active >> lane & 1;
            }
            let global_before: Vec<_> = (0..256).map(|i| 0x6100_0000 + i).collect();
            let Some((lanes, global)) = run_lanes(&words, &seeds, &[], &global_before) else {
                eprintln!("skipping warp silicon fixtures: no Vulkan device");
                return;
            };
            let outputs_match = lanes
                .iter()
                .enumerate()
                .all(|(lane, regs)| regs[fixture.output_regs[0]] == fixture.outputs[lane * 2]);
            let global_matches =
                !matches!(fixture.family.as_str(), "atom" | "atomg" | "red" | "stg")
                    || global == fixture.global_after;
            let shared_matches = fixture.family != "sts"
                || lanes.iter().enumerate().all(|(lane, regs)| {
                    let word = fixture.inputs[lane * 4 + 1] as usize / 4;
                    regs[fixture.output_regs[1]] == fixture.shared_after[word]
                });
            if outputs_match && shared_matches && global_matches {
                score.pass += 1;
            } else if fixture.family == "s2r" && fixture.case == "eqmask" {
                score.unimplemented += 1;
            } else {
                score.fail += 1;
                eprintln!("{}/{}: warp output mismatch", fixture.family, fixture.case);
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
}
