//! Latency of the hot paths in microseconds: `cargo run --release -p oboromi-core --example perf`.
use ash::vk::DescriptorType;
use oboromi_core::display::present;
use oboromi_core::ffi::oboromi_frame_read;
use oboromi_core::gpu::compute::{ComputePipeline, HostBuffer};
use oboromi_core::gpu::sm86::STATE_WORDS;
use oboromi_core::gpu::{device, frame::render_test_frame};
use oboromi_core::tests::gpu::harness::translate_module;
use oboromi_core::tests::gpu_test::inst;
use std::hint::black_box;
use std::time::Instant;

fn bench(name: &str, warmup: usize, iters: usize, mut f: impl FnMut()) {
    (0..warmup).for_each(|_| f());
    let mut us: Vec<f64> = (0..iters)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed().as_secs_f64() * 1e6
        })
        .collect();
    us.sort_by(f64::total_cmp);
    let [p50, p95, p99, max] =
        [0.5, 0.95, 0.99, 1.0].map(|p| us[(p * iters as f64).ceil() as usize - 1]);
    println!("{name:<26}{iters:>6}{p50:>11.1}{p95:>11.1}{p99:>11.1}{max:>11.1}");
}

fn main() {
    println!("path                           n     p50 us     p95 us     p99 us     max us");
    // KILL is left out: OpKill terminates the block, so nothing may follow it.
    let fixtures = [
        inst::iadd_imm(1, 2, 42),
        inst::iadd_reg(1, 2, 3),
        inst::iadd3_reg(1, 2, 3, 4),
        inst::mov_imm(1, 0xCAFEBABE),
        inst::mov_reg(1, 2),
    ];
    let program: Vec<u128> = fixtures.iter().copied().cycle().take(1000).collect();
    bench("translate 1000 inst", 20, 200, || {
        black_box(translate_module(black_box(&program), true, None));
    });
    let (w, h) = (1920, 1080);
    let pixels = vec![0x80u8; w * h * 4];
    let mut dst = vec![0u8; pixels.len()];
    bench("present+frame_read 1080p", 10, 200, || {
        present(w as u32, h as u32, &pixels);
        let (mut fw, mut fh) = (0, 0);
        unsafe { oboromi_frame_read(dst.as_mut_ptr(), dst.len(), &mut fw, &mut fh) };
        black_box(&dst);
    });
    bench("vk device init (cold)", 0, 1, || drop(device()));
    let state = match device() {
        Ok(state) => state,
        Err(e) => return println!("gpu paths skipped: {e:?}"),
    };
    let props = unsafe {
        state
            .instance
            .get_physical_device_properties(state.physical_device)
    };
    println!("device: {:?}", props.device_name_as_c_str().unwrap());
    let program = [inst::mov_imm(0, 42), inst::iadd_reg(1, 2, 3)];
    let words = translate_module(&program, true, Some(state)).words();
    let ssbo = DescriptorType::STORAGE_BUFFER;
    let bindings = [(10, ssbo), (9, ssbo)];
    bench("pipeline create (cold)", 0, 1, || {
        drop(ComputePipeline::new(state, &words, &bindings).unwrap());
    });
    let mut pipeline = ComputePipeline::new(state, &words, &bindings).unwrap();
    let mut input = HostBuffer::new(state, STATE_WORDS as u64 * 4).unwrap();
    let output = HostBuffer::new(state, STATE_WORDS as u64 * 4).unwrap();
    pipeline.bind(&[(10, &input), (9, &output)]);
    let mut out = [0u8; STATE_WORDS * 4];
    bench("dispatch+fence+readback", 50, 1000, || {
        input.write(&[0; STATE_WORDS * 4]).unwrap();
        state.dispatch(&pipeline, [1, 1, 1]).unwrap();
        output.read(black_box(&mut out)).unwrap();
    });
    bench("render_test_frame e2e", 2, 20, || {
        render_test_frame().unwrap()
    });
}
