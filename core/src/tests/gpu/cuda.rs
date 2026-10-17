#[cfg(test)]
mod tests {
    use crate::gpu::cuda::{CudaDevice, CudaError};
    use crate::tests::gpu::harness::{GLOBAL_BASE, run_grid};
    use std::time::Instant;

    fn arrays<'a>(json: &'a str, key: &str) -> Vec<Vec<&'a str>> {
        json.split(&format!("\"{key}\":"))
            .skip(1)
            .map(|value| {
                let items = value.split_once('[').unwrap().1.split_once(']').unwrap().0;
                items
                    .split(',')
                    .map(|item| item.trim().trim_matches('"'))
                    .collect()
            })
            .collect()
    }

    fn words(json: &str) -> Vec<u128> {
        arrays(json, "words")[0]
            .iter()
            .map(|word| u128::from_str_radix(word, 16).unwrap())
            .collect()
    }

    fn u32s(items: &[&str]) -> Vec<u32> {
        items.iter().map(|item| item.parse().unwrap()).collect()
    }

    fn bytes(words: &[u32]) -> Vec<u8> {
        words.iter().flat_map(|word| word.to_le_bytes()).collect()
    }

    fn device() -> Option<CudaDevice> {
        match CudaDevice::new() {
            Ok(device) => Some(device),
            Err(error @ (CudaError::MissingLibrary | CudaError::NoDevice)) => {
                eprintln!("skipping CUDA test: {error:?}");
                None
            }
            Err(error) => panic!("CUDA device creation failed: {error:?}"),
        }
    }

    #[test]
    fn native_mul_add_matches_translator() {
        let Some(device) = device() else { return };
        let json = include_str!("../../../tests/fixtures/mul-add-32.json");
        let program = words(json);
        let [grid, block] =
            ["grid", "block"].map(|key| u32s(&arrays(json, key)[0]).try_into().unwrap());
        let cbuf_hex = json
            .split_once("\"cbuf0\":\"")
            .unwrap()
            .1
            .split_once('"')
            .unwrap()
            .0;
        let cbuf: Vec<u8> = (0..cbuf_hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&cbuf_hex[i..i + 2], 16).unwrap())
            .collect();
        let pointer =
            |offset: usize| u64::from_le_bytes(cbuf[offset..offset + 8].try_into().unwrap());
        let mut a = bytes(&u32s(&arrays(json, "f32_bits")[0]));
        let mut b = bytes(&u32s(&arrays(json, "f32_bits")[1]));
        let mut native = vec![0; a.len()];
        let translated = run_grid(
            &program,
            block,
            grid,
            &cbuf,
            &[
                (pointer(0x160), &a),
                (pointer(0x168), &b),
                (pointer(0x170), &native),
            ],
        )
        .unwrap();
        let start = Instant::now();
        device
            .run_native(
                &program,
                grid,
                block,
                0,
                &32u32.to_le_bytes(),
                &mut [&mut a, &mut b, &mut native],
            )
            .unwrap();
        eprintln!("CUDA mul-add launch: {:?}", start.elapsed());
        assert_eq!(a, translated[0]);
        assert_eq!(b, translated[1]);
        assert_eq!(native, translated[2]);
    }

    #[test]
    fn native_gradient_matches_translator() {
        let Some(device) = device() else { return };
        let json = include_str!("../../../tests/fixtures/gradient-sm86.json");
        let mut program = words(json);
        let template = 0x000fe40000000f0000000a0000017a02u128;
        let mov_cbuf = |rd: u128, offset: u128| {
            template & !(0xff << 16) & !(0x3fff << 40) | rd << 16 | (offset / 4) << 40
        };
        program[16] = mov_cbuf(4, 0x160);
        program[17] = mov_cbuf(5, 0x164);
        let [grid, block] =
            ["grid", "block"].map(|key| u32s(&arrays(json, key)[0]).try_into().unwrap());
        let mut cbuf = vec![0; 0x168];
        cbuf[0x160..0x168].copy_from_slice(&GLOBAL_BASE.to_le_bytes());
        let mut native = vec![0; 1920 * 1080 * 4];
        let translated =
            run_grid(&program, block, grid, &cbuf, &[(GLOBAL_BASE, &native)]).unwrap()[0].clone();
        let start = Instant::now();
        device
            .run_native(&program, grid, block, 0, &[], &mut [&mut native])
            .unwrap();
        eprintln!("CUDA gradient launch: {:?}", start.elapsed());
        assert_eq!(native, translated);
    }
}
