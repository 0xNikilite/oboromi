#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::{run_regs, translate_module};
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
}
