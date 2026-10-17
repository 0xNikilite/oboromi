//! Readers for the RTX 3090 fixture files, and the tensor replay that decides whether a device's cooperative matrices
//! reproduce the silicon.
use crate::gpu::VkState;
use crate::gpu::sm86::{Hints, TensorMode, cooperative_tile, f16_hmma_shape};
use crate::tests::gpu::harness::{run_on, PRED};
use crate::tests::gpu_test::inst;

pub fn string(object: &str, key: &str) -> String {
    let value = object
        .split_once(&format!("\"{key}\":"))
        .unwrap()
        .1
        .trim_start();
    let value = value.strip_prefix('"').unwrap();
    value[..value.find('"').unwrap()].to_owned()
}

pub fn numbers(object: &str, key: &str) -> Vec<u32> {
    let Some((_, value)) = object.split_once(&format!("\"{key}\":")) else {
        return Vec::new();
    };
    let value = value.split_once('[').unwrap().1.split_once(']').unwrap().0;
    value
        .split(',')
        .filter_map(|n| n.trim().parse().ok())
        .collect()
}

pub fn observed(object: &str) -> bool {
    object.contains("\"observed\":true")
}

pub struct TensorFixture {
    pub id: String,
    pub case: String,
    pub modifier: String,
    pub words: Vec<u128>,
    pub observed: bool,
    pub before: Vec<u32>,
    pub after: Vec<u32>,
}

pub fn tensor_fixtures() -> Vec<TensorFixture> {
    include_str!("../../../tests/fixtures/sm86-tensor.jsonl")
        .lines()
        .map(|object| {
            let case = string(object, "case");
            let [dataset, modifier] = ["dataset", "modifier"].map(|key| {
                object
                    .contains(&format!("\"{key}\":\""))
                    .then(|| string(object, key))
                    .unwrap_or_default()
            });
            TensorFixture {
                id: format!(
                    "{}/tensor/{case}/{dataset}/{modifier}",
                    string(object, "id")
                ),
                case,
                modifier,
                words: object
                    .split_once("\"words\":[")
                    .unwrap()
                    .1
                    .split_once(']')
                    .unwrap()
                    .0
                    .split(',')
                    .map(|word| u128::from_str_radix(word.trim_matches('"'), 16).unwrap())
                    .collect(),
                observed: observed(object),
                before: numbers(object, "fragments_before"),
                after: numbers(object, "fragments_after"),
            }
        })
        .collect()
}

pub fn tensor_program(fixture: &TensorFixture) -> Vec<u128> {
    if fixture.words.len() == 1 {
        return fixture.words.clone();
    }
    let index = match fixture.case.as_str() {
        "m16n8k8-f32" => 0,
        "m16n8k16-f32" => 1,
        "m16n8k8-tf32" => 2,
        "ldmatrix-x4-trans" => 3,
        "ldmatrix-x4" => 4,
        "m16n8k16-f16" => 5,
        _ => unreachable!(),
    };
    if fixture.case.starts_with("ldmatrix") {
        vec![
            inst::sts(20, 16),
            inst::sts(21, 17),
            inst::sts(22, 18),
            inst::sts(23, 19),
            fixture.words[index],
        ]
    } else {
        vec![fixture.words[index]]
    }
}

pub fn tensor_seeds(fixture: &TensorFixture) -> Vec<Vec<u32>> {
    let mut seeds = vec![vec![0; PRED + 1]; 32];
    for (lane, seed) in seeds.iter_mut().enumerate() {
        let before = &fixture.before[lane * 12..][..12];
        match fixture.case.as_str() {
            "m16n8k8-f32" => {
                seed[6..8].copy_from_slice(&before[..2]);
                seed[0] = before[2];
                seed[8..12].copy_from_slice(&before[3..7]);
            }
            case if case.starts_with("ldmatrix") => {
                let slot = 16 * (5 * lane as u32 % 32);
                seed[0] = slot;
                seed[16..20].copy_from_slice(&before[..4]);
                for word in 0..4 {
                    seed[20 + word] = slot + 4 * word as u32;
                }
                seed[PRED] = (case == "ldmatrix-x4-trans") as u32;
            }
            case if case.starts_with("m8n8") || case.starts_with("m16n8k4") || case.starts_with("m16n8k32") || case.starts_with("m16n8k64") => {
                let [ra, rb, rc] = [24, 32, 64].map(|bit| (fixture.words[0] >> bit & 0xff) as usize);
                let (a, b, c) = match case {
                    "m16n8k4-tf32" | "m16n8k32-s4" => (2, 1, 4),
                    "m16n8k64-s4" | "m16n8k32-s8" | "m16n8k32-s8-satfinite" => (4, 2, 4),
                    _ => (1, 1, 2),
                };
                seed[ra..ra + a].copy_from_slice(&before[..a]);
                seed[rb..rb + b].copy_from_slice(&before[a..a + b]);
                seed[rc..rc + c].copy_from_slice(&before[a + b..a + b + c]);
            }
            _ => {
                seed[12..16].copy_from_slice(&before[..4]);
                seed[6..8].copy_from_slice(&before[4..6]);
                seed[8..12].copy_from_slice(&before[6..10]);
            }
        }
        let pg = (fixture.words[0] >> 12 & 7) as usize;
        if pg != 7 && fixture.words[0] >> 15 & 1 == 0 {
            seed[PRED] |= 1 << pg;
        }
    }
    seeds
}

pub fn tensor_d_words(fixture: &TensorFixture) -> usize {
    if fixture.case == "m16n8k16-f16" || fixture.modifier == "dst-f16" || fixture.case.starts_with("m8n8") { 2 } else { 4 }
}

pub fn tensor_mismatch(fixture: &TensorFixture, lanes: &[Vec<u32>]) -> Option<(usize, Vec<u32>, Vec<u32>)> {
    let width = tensor_d_words(fixture);
    let rd = (fixture.words[0] >> 16 & 0xff) as usize;
    lanes.iter().enumerate().find_map(|(lane, regs)| {
        let expected = &fixture.after[lane * 4..][..width];
        let actual = &regs[rd..rd + width];
        (actual != expected).then_some((lane, expected.to_vec(), actual.to_vec()))
    })
}

/// Runs `fixture` on `state`, through cooperative matrices when `cooperative` is true and the device has a fitting tile.
pub fn replay_tensor(state: &VkState, fixture: &TensorFixture, cooperative: Option<bool>, tensor: TensorMode) -> Option<Vec<Vec<u32>>> {
    let program = tensor_program(fixture);
    let body = |decoder: &mut crate::gpu::sm86::Decoder| {
        decoder.force_cooperative_hmma = cooperative;
        decoder.translate_program(&program);
    };
    run_on(state, body, &tensor_seeds(fixture), &[], &[], &Default::default(), Hints { tensor, ..Default::default() }).map(|(lanes, ..)| lanes)
}

/// The f16 HMMA shapes for which the device reproduces every observed fixture bit for bit through cooperative matrices.
pub fn exact_cooperative_hmma(state: &VkState) -> Vec<(u32, bool)> {
    let fixtures: Vec<_> = tensor_fixtures().into_iter().filter(|fixture| fixture.observed)
        .filter_map(|fixture| Some((f16_hmma_shape(*tensor_program(&fixture).last()?)?, fixture))).collect();
    let mut shapes: Vec<_> = fixtures.iter().map(|&(shape, _)| shape).collect();
    shapes.sort_unstable();
    shapes.dedup();
    shapes.retain(|&shape| cooperative_tile(state, shape).is_some() && fixtures.iter().filter(|&&(of, _)| of == shape).all(|(_, fixture)| {
        replay_tensor(state, fixture, Some(true), TensorMode::Exact).is_some_and(|lanes| tensor_mismatch(fixture, &lanes).is_none())
    }));
    shapes
}
