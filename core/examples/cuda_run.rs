//! Runs a kernel of an nvcc-built sm_8x cubin through the SASS -> SPIR-V -> Vulkan path and writes its `out` buffer as
//! an RGBA8 image (alpha dropped): `cargo run --release -p oboromi-core --example cuda_run -- k.cubin name gx,gy,gz
//! bx,by,bz [out | number]... --image width height out.ppm`. A parameter is `out`, the image buffer passed as its
//! address, or a number sized by the parameter: an integer (0x for hex), or with a '.' an f32 or f64.
use oboromi_core::gpu::cubin::{Arg, Kernel};
use oboromi_core::gpu::device;
use oboromi_core::tests::gpu::harness::{GLOBAL_BASE, Grid};
use std::time::Instant;

const RUNS: usize = 50;

fn dims(arg: &str) -> [u32; 3] {
    let dims: Vec<u32> = arg.split(',').map(|dim| dim.parse().unwrap()).collect();
    dims.try_into().expect("dimensions are x,y,z")
}

fn value(arg: &str, size: usize) -> Vec<u8> {
    let bytes = match (arg.contains('.'), size) {
        (true, 4) => arg.parse::<f32>().unwrap().to_le_bytes().to_vec(),
        (true, _) => arg.parse::<f64>().unwrap().to_le_bytes().to_vec(),
        _ => arg.strip_prefix("0x").map_or_else(|| arg.parse(), |hex| i64::from_str_radix(hex, 16)).unwrap().to_le_bytes().to_vec(),
    };
    bytes[..size].to_vec()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let image = args.iter().position(|arg| arg == "--image").expect("--image width height out.ppm");
    let [width, height]: [usize; 2] = [1, 2].map(|i| args[image + i].parse().unwrap());
    let kernel = Kernel::load(&std::fs::read(&args[0]).unwrap(), &args[1]).unwrap();
    let (grid, block, params) = (dims(&args[2]), dims(&args[3]), &args[4..image]);
    let pixels = vec![0; width * height * 4];
    let values: Vec<_> = params.iter().zip(&kernel.params).map(|(arg, &(_, size))| if arg == "out" { vec![] } else { value(arg, size) }).collect();
    let launch: Vec<_> = params.iter().zip(&values).map(|(arg, value)| if arg == "out" { Arg::Buffer(&pixels) } else { Arg::Value(value) }).collect();
    let (banks, regions) = kernel.launch(grid, block, &launch, GLOBAL_BASE);
    let mut kernel_grid = Grid::new(device().unwrap(), &kernel.words, block, &banks, &regions);
    let mut ms: Vec<f64> = (0..RUNS).map(|_| {
        let start = Instant::now();
        kernel_grid.run(grid);
        start.elapsed().as_secs_f64() * 1e3
    }).collect();
    ms.sort_by(f64::total_cmp);
    println!("{}: p50 {:.2} ms per launch and readback over {RUNS} runs", args[1], ms[RUNS / 2]);
    let rgb: Vec<u8> = kernel_grid.run(grid)[0].chunks(4).flat_map(|pixel| &pixel[..3]).copied().collect();
    std::fs::write(&args[image + 3], [format!("P6\n{width} {height}\n255\n").as_bytes(), &rgb].concat()).unwrap();
}
