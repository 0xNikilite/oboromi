#[cfg(test)]
mod tests {
    use crate::tests::gpu::harness::run_grid;
    use std::io::Write;
    use std::process::{Command, Stdio};

    fn arrays<'a>(json: &'a str, key: &str) -> Vec<Vec<&'a str>> {
        json.split(&format!("\"{key}\":")).skip(1).map(|value| {
            let items = value.split_once('[').unwrap().1.split_once(']').unwrap().0;
            items.split(',').map(|item| item.trim().trim_matches('"')).collect()
        }).collect()
    }

    fn string<'a>(json: &'a str, key: &str) -> &'a str {
        json.split_once(&format!("\"{key}\": \"")).unwrap().1.split_once('"').unwrap().0
    }

    fn u32s(items: &[&str]) -> Vec<u32> {
        items.iter().map(|item| item.parse().unwrap()).collect()
    }

    fn bytes(words: &[u32]) -> Vec<u8> {
        words.iter().flat_map(|word| word.to_le_bytes()).collect()
    }

    fn launch(json: &str, cbuf0: &[u8], regions: &[(u64, &[u8])]) -> Option<Vec<Vec<u8>>> {
        let words: Vec<u128> = arrays(json, "words")[0].iter().map(|word| u128::from_str_radix(word, 16).unwrap()).collect();
        let [grid, block] = ["grid", "block"].map(|key| u32s(&arrays(json, key)[0]).try_into().unwrap());
        run_grid(&words, block, grid, cbuf0, regions)
    }

    // nvcc -arch=sm_86 for c[i] = a[i] * b[i] + i: the kernel reads its pointer parameters from c[0x0][0x160..0x178].
    fn run_mul_add(json: &str) {
        let cbuf_hex = string(json, "cbuf0");
        let cbuf0: Vec<u8> = (0..cbuf_hex.len()).step_by(2).map(|i| u8::from_str_radix(&cbuf_hex[i..i + 2], 16).unwrap()).collect();
        let param = |offset: usize| u64::from_le_bytes(cbuf0[offset..offset + 8].try_into().unwrap());
        let [a, b] = [0, 1].map(|i| bytes(&u32s(&arrays(json, "f32_bits")[i])));
        let expected = u32s(&arrays(json, "outputs")[0]);
        let c = vec![0; 4 * expected.len()];
        let Some(regions) = launch(json, &cbuf0, &[(param(0x160), &a), (param(0x168), &b), (param(0x170), &c)]) else {
            return;
        };
        assert_eq!(regions[2], bytes(&expected));
    }

    #[test]
    fn compiled_mul_add_kernel_matches_silicon() {
        run_mul_add(include_str!("../../../tests/fixtures/mul-add-32.json"));
        run_mul_add(include_str!("../../../tests/fixtures/mul-add-1024.json"));
    }

    // The program writes a 1920x1080 RGBA8 frame to the address its MOV R4, 0x1000 and MOV R5, 0x2 build.
    #[test]
    fn translated_gradient_program_matches_silicon_hash() {
        let json = include_str!("../../../tests/fixtures/gradient-sm86.json");
        let Some(regions) = launch(json, &[], &[(0x2_0000_1000, &vec![0; 1920 * 1080 * 4])]) else {
            return;
        };
        let mut sha256sum = Command::new("sha256sum").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
        sha256sum.stdin.take().unwrap().write_all(&regions[0]).unwrap();
        let digest = String::from_utf8(sha256sum.wait_with_output().unwrap().stdout).unwrap();
        assert_eq!(&digest[..64], string(json, "sha256"));
    }
}
