//! Latency of the hot paths in microseconds: `MESA_SHADER_CACHE_DISABLE=true cargo run --release -p oboromi-core --example perf`,
//! where the variable keeps Mesa's own shader cache from warming the cold pipeline rows.
use ash::vk::DescriptorType;
use oboromi_core::display::{attach_surface, present, present_buffer};
use oboromi_core::ffi::{oboromi_frame_read, oboromi_set_tensor_mode};
use oboromi_core::gpu::compute::{ComputePipeline, HostBuffer};
use oboromi_core::gpu::cubin::{Arg, Kernel};
use oboromi_core::gpu::pipeline_cache::Disk;
use oboromi_core::gpu::sm86::{CBUF_BINDING, GLOBAL_BINDING, STATE_WORDS, SUPPORT_BINDING, Stage, emit_module};
use oboromi_core::audio::demo_music::{music, sequence};
use oboromi_core::gpu::{VkState, demo, demo_bounce::render_bounce_frame, demo_particles::{self, render_particles_frame}, demo_post::render_post_frame, demo_sprites::render_sprites_frame, surface_device, frame::{julia_program, render_demo_frame, render_test_frame, render_triangle_demo}};
use oboromi_core::tests::gpu::engine::{TARGET, TEXTURES, VERTICES, texture_header, textured_quad_frame, triangle_frame, triangle_guest, vertex_bytes};
use oboromi_core::tests::gpu::{graphics, harness::{GLOBAL_BASE, Grid, translate_module}, silicon::exact_cooperative_hmma};
use oboromi_core::tests::gpu_test::inst;
use std::hint::black_box;
use std::time::Instant;

const KERNELS: &[u8] = include_bytes!("../tests/fixtures/kernels.cubin");
const BENCH: &[u8] = include_bytes!("../tests/fixtures/bench/bench.cubin");

/// Prints the row and returns its host and GPU p50 in microseconds.
fn bench(name: &str, warmup: usize, iters: usize, gpu: Option<&VkState>, mut f: impl FnMut()) -> [f64; 2] {
    (0..warmup).for_each(|_| f());
    let (mut us, mut gpu_us): (Vec<f64>, Vec<f64>) = (0..iters)
        .map(|_| {
            let t = Instant::now();
            f();
            (since(t), gpu.map_or(0.0, |state| state.last_gpu_time_ns() as f64 / 1e3))
        })
        .unzip();
    us.sort_by(f64::total_cmp);
    gpu_us.sort_by(f64::total_cmp);
    let [p50, p95, p99, max] =
        [0.5, 0.95, 0.99, 1.0].map(|p| us[(p * iters as f64).ceil() as usize - 1]);
    let gpu_p50 = gpu_us[iters.div_ceil(2) - 1];
    let gpu_column = gpu.map_or("-".into(), |_| format!("{gpu_p50:.1}"));
    println!("{name:<30}{iters:>6}{p50:>11.1}{p95:>11.1}{p99:>11.1}{max:>11.1}{gpu_column:>11}");
    [p50, gpu_p50]
}

fn since(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e6
}

fn breakdown(name: &str, [frame, gpu]: [f64; 2], copy: f64, present: f64, cold: f64) {
    let fence = frame - gpu - copy - present;
    println!("{name:<30}gpu {gpu:.0}, submit+fence {fence:.0}, readback copy {copy:.0}, present {present:.0}, translate+pipeline once {cold:.0}");
}

fn kernel_grid<'a>(state: &'a VkState, kernel: &Kernel, grid: [u32; 3], block: [u32; 3], args: &[Arg]) -> (Grid<'a>, f64) {
    let (banks, regions) = kernel.launch(grid, block, args, GLOBAL_BASE);
    let t = Instant::now();
    let built = Grid::new(state, &kernel.words, block, &banks, &regions);
    (built, since(t))
}

fn buffers<'a>(args: &[Arg<'a>]) -> Vec<&'a [u8]> {
    args.iter().filter_map(|arg| if let Arg::Buffer(bytes) = *arg { Some(bytes) } else { None }).collect()
}

fn values(args: &[Arg]) -> Vec<u8> {
    args.iter().flat_map(|arg| if let Arg::Value(bytes) = *arg { bytes } else { &[] }).copied().collect()
}

fn versus(state: &VkState, name: &str, spv: &[u8], groups: [u32; 3], block: [u32; 3], args: &[Arg]) -> [f64; 2] {
    let kernel = Kernel::load(BENCH, name).unwrap();
    let mut grid = kernel_grid(state, &kernel, groups, block, args).0;
    let translated = bench(&format!("{name} translated"), 2, 20, Some(state), || drop(black_box(grid.run(groups))))[1];
    let words = ash::util::read_spv(&mut std::io::Cursor::new(spv)).unwrap();
    let mut uniforms = HostBuffer::new(state, 32).unwrap();
    uniforms.write(&values(args)).unwrap();
    let mut out = vec![0; buffers(args).iter().map(|buffer| buffer.len()).sum()];
    let memory = HostBuffer::new(state, out.len() as u64).unwrap();
    let bindings = [(CBUF_BINDING, DescriptorType::UNIFORM_BUFFER), (GLOBAL_BINDING, DescriptorType::STORAGE_BUFFER)];
    let mut pipeline = ComputePipeline::new(state, &words, &bindings).unwrap();
    pipeline.descriptors.bind(&[(CBUF_BINDING, &uniforms), (GLOBAL_BINDING, &memory)]);
    let glsl = bench(&format!("{name} glsl twin"), 2, 20, Some(state), || {
        state.dispatch(&pipeline, groups).unwrap();
        memory.read(&mut out).unwrap();
    })[1];
    native(&format!("{name} native cuda"), &kernel, groups, block, args);
    [translated, glsl]
}

#[cfg_attr(not(feature = "cuda"), allow(unused_variables))]
fn native(name: &str, kernel: &Kernel, grid: [u32; 3], block: [u32; 3], args: &[Arg]) {
    #[cfg(feature = "cuda")]
    match oboromi_core::gpu::cuda::CudaDevice::new() {
        Ok(cuda) => {
            let (mut owned, params): (Vec<Vec<u8>>, _) = (buffers(args).into_iter().map(<[u8]>::to_vec).collect(), values(args));
            let mut memory: Vec<&mut [u8]> = owned.iter_mut().map(Vec::as_mut_slice).collect();
            bench(name, 2, 20, None, || cuda.run_native(&kernel.words, grid, block, 0, &params, &mut memory).unwrap());
        }
        Err(e) => println!("{name:<30}no cuda: {e:?}"),
    }
    #[cfg(not(feature = "cuda"))]
    println!("{name:<30}no cuda");
}

fn main() {
    println!("path                               n     p50 us     p95 us     p99 us     max us     gpu us");
    // KILL is left out: OpKill terminates the block, so nothing may follow it.
    let fixtures = [
        inst::iadd_imm(1, 2, 42),
        inst::iadd_reg(1, 2, 3),
        inst::mov_imm(1, 0xCAFEBABE),
        inst::mov_reg(1, 2),
    ];
    let program: Vec<u128> = fixtures.iter().copied().cycle().take(1000).collect();
    bench("translate 1000 inst", 20, 200, None, || {
        black_box(translate_module(black_box(&program), true, None, false));
    });
    bench("translate hints on", 20, 200, None, || {
        black_box(translate_module(black_box(&program), true, None, true));
    });
    let mut song = music();
    bench("audren music period", 20, 2000, None, || {
        sequence(&mut song);
        black_box(song.period());
    });
    let (w, h) = (1920, 1080);
    let pixels = vec![0x80u8; w * h * 4];
    let mut dst = vec![0u8; pixels.len()];
    bench("present+frame_read 1080p", 10, 200, None, || {
        present(w as u32, h as u32, &pixels);
        let (mut fw, mut fh) = (0, 0);
        unsafe { oboromi_frame_read(dst.as_mut_ptr(), dst.len(), &mut fw, &mut fh) };
        black_box(&dst);
    });
    let period = vec![0x1234i16; 2 * oboromi_core::audio::PERIOD as usize];
    let mut pulled = period.clone();
    bench("audio latency push->pull", 100, 10000, None, || {
        oboromi_core::audio::push(&period);
        oboromi_core::audio::pull(&mut pulled);
        black_box(&pulled);
    });
    bench("vk device init (cold)", 0, 1, None, || {
        let _ = surface_device();
    });
    let state = match surface_device() {
        Ok(state) => state,
        Err(e) => return println!("gpu paths skipped: {e:?}"),
    };
    let props = unsafe {
        state
            .instance
            .get_physical_device_properties(state.physical_device)
    };
    println!("device: {:?}", props.device_name_as_c_str().unwrap());
    let exact = exact_cooperative_hmma(state);
    println!("hmma: cooperative matrices for (k, f32 accumulator) {exact:?} of f16 tiles {:?}, loops otherwise", state.cooperative_f16_tiles);
    let (block, groups, chain) = ([128, 1, 1], [32, 1, 1], 16);
    let sources = [8, 9, 12, 13].map(|reg| inst::s2r(reg, 33));
    let hmmas = vec![inst::alu3(inst::HMMA | inst::MMA_16816 | inst::MMA_F32, 4, 8, 12, 4); chain];
    let store = [inst::mov_imm(2, GLOBAL_BASE as u32), inst::mov_imm(3, (GLOBAL_BASE >> 32) as u32), inst::stg(2, 0, 4), inst::stg(2, 4, 5), inst::stg(2, 8, 6), inst::stg(2, 12, 7), inst::exit()];
    let hmma_program = [&sources[..], &hmmas, &store].concat();
    let ops = f64::from(groups[0] * block[0] / 32) * chain as f64;
    let [exact, fast] = [false, true].map(|fast| {
        oboromi_set_tensor_mode(fast);
        let mut grid = Grid::new(state, &hmma_program, block, &[], &[(GLOBAL_BASE, &[0; 16])]);
        bench(if fast { "hmma fast" } else { "hmma exact" }, 2, 20, Some(state), || drop(black_box(grid.run(groups))))[1]
    });
    oboromi_set_tensor_mode(false);
    println!("hmma 16816.F32 gpu ns per warp op: exact {:.0}, fast {:.0}", exact * 1e3 / ops, fast * 1e3 / ops);
    let program = [inst::mov_imm(0, 42), inst::iadd_reg(1, 2, 3)];
    let words = translate_module(&program, true, Some(state), false).words();
    let ssbo = DescriptorType::STORAGE_BUFFER;
    let bindings = [(10, ssbo), (11, ssbo)];
    bench("pipeline create (cold)", 0, 1, None, || {
        drop(ComputePipeline::new(state, &words, &bindings).unwrap());
    });
    let julia = emit_module(Some(state), Stage::Compute { local_size: [16, 16, 1] }, |d| d.translate_program(&julia_program())).words();
    let ubo = DescriptorType::UNIFORM_BUFFER;
    let julia_bindings = [(SUPPORT_BINDING, ubo), (CBUF_BINDING, ubo), (GLOBAL_BINDING, ssbo)];
    let root = std::env::temp_dir().join(format!("oboromi-perf-{}", std::process::id()));
    let disk = Disk::open(&root, "perf", &props).unwrap();
    for start in ["cold", "from disk"] {
        let fresh = VkState::new().unwrap();
        if start != "cold" {
            disk.load(&fresh).unwrap();
        }
        bench(&format!("triangle pipeline {start}"), 0, 1, None, || drop(graphics::triangle_pipeline(&fresh).unwrap()));
        bench(&format!("julia pipeline {start}"), 0, 1, None, || drop(ComputePipeline::new(&fresh, &julia, &julia_bindings).unwrap()));
        disk.flush(&fresh).unwrap();
    }
    for start in ["cold", "warm"] {
        let fresh = Box::leak(Box::new(VkState::new().unwrap()));
        let mut gpu = triangle_guest(fresh, Some((&root, "engine")), 1920, 1080, &[([0.0, -0.8], [255; 4]), ([0.7, 0.4], [255; 4]), ([-0.7, 0.4], [255; 4])]).unwrap();
        bench(&format!("engine first frame {start}"), 0, 1, Some(fresh), || triangle_frame(&mut gpu, 1920, 1080, [1.0, 0.0, 0.0, 1.0], 0).unwrap());
    }
    let _ = std::fs::remove_dir_all(root);
    for (name, libraries) in [("triangle monolithic compile", false), ("triangle libraries + link", true)] {
        let mut fresh = VkState::new().unwrap();
        fresh.graphics_pipeline_library &= libraries;
        bench(name, 0, 1, None, || drop(graphics::triangle_pipeline(&fresh).unwrap()));
        if fresh.graphics_pipeline_library {
            bench("triangle link only", 0, 1, None, || drop(graphics::triangle_pipeline(&fresh).unwrap()));
        }
    }
    let mut pipeline = ComputePipeline::new(state, &words, &bindings).unwrap();
    let mut input = HostBuffer::new(state, STATE_WORDS as u64 * 4).unwrap();
    let output = HostBuffer::new(state, STATE_WORDS as u64 * 4).unwrap();
    pipeline.descriptors.bind(&[(10, &input), (11, &output)]);
    let mut out = [0u8; STATE_WORDS * 4];
    bench("dispatch+fence+readback", 50, 1000, Some(state), || {
        input.write(&[0; STATE_WORDS * 4]).unwrap();
        state.dispatch(&pipeline, [1, 1, 1]).unwrap();
        output.read(black_box(&mut out)).unwrap();
    });
    let frame = HostBuffer::new(state, pixels.len() as u64).unwrap();
    bench("present_buffer 1080p", 10, 200, Some(state), || {
        present_buffer(&frame, w as u32, h as u32).unwrap()
    });
    let mut triangle = graphics::triangle(state).unwrap();
    let mut triangle_pixels = vec![0; (graphics::SIZE * graphics::SIZE * 4) as usize];
    bench("draw+readback 64x64 triangle", 50, 1000, Some(state), || triangle(black_box(&mut triangle_pixels)).unwrap());
    bench("render_test_frame e2e", 2, 20, Some(state), || {
        render_test_frame().unwrap()
    });
    let t = Instant::now();
    render_demo_frame(0.25).unwrap();
    let julia_cold = since(t);
    let julia = bench("render_demo_frame e2e", 2, 100, Some(state), || {
        render_demo_frame(0.25).unwrap()
    });
    let julia_copy = bench("readback copy 1080p", 2, 100, None, || frame.read(&mut dst).unwrap())[0];
    let julia_present = bench("present 1080p", 2, 100, None, || present(w as u32, h as u32, &pixels))[0];
    let gradient = Kernel::load(KERNELS, "gradient").unwrap();
    let mut gradient = kernel_grid(state, &gradient, [120, 68, 1], [16, 16, 1], &[Arg::Buffer(&pixels)]).0;
    bench("cubin gradient 1080p", 2, 50, Some(state), || drop(black_box(gradient.run([120, 68, 1]))));
    let (mw, mh, crunch, scale) = (800u32, 600u32, 512u32, 3.2f32 / 800.0);
    let params = [mw.to_le_bytes(), mh.to_le_bytes(), crunch.to_le_bytes(), (-0.5 - 400.0 * scale).to_le_bytes(), (-300.0 * scale).to_le_bytes(), scale.to_le_bytes()].concat();
    let (image, mut copy) = (vec![0; (mw * mh * 4) as usize], vec![0; (mw * mh * 4) as usize]);
    let args: Vec<_> = [Arg::Buffer(&image)].into_iter().chain(params.chunks(4).map(Arg::Value)).collect();
    let mandelbrot = Kernel::load(BENCH, "mandelbrot").unwrap();
    let (mut mandelbrot_grid, mandelbrot_cold) = kernel_grid(state, &mandelbrot, [50, 38, 1], [16, 16, 1], &args);
    let mandelbrot_frame = bench("cubin mandelbrot 800x600", 2, 50, Some(state), || present(mw, mh, &mandelbrot_grid.run([50, 38, 1])[0]));
    let image_buffer = HostBuffer::new(state, image.len() as u64).unwrap();
    let mandelbrot_copy = bench("readback copy 800x600", 2, 100, None, || {
        image_buffer.read(&mut copy).unwrap();
        drop(black_box(copy.to_vec()));
    })[0];
    let mandelbrot_present = bench("present 800x600", 2, 100, None, || present(mw, mh, &image))[0];
    if std::env::var_os("OBOROMI_PERF_NATIVE").is_some() {
        versus(state, "mandelbrot", include_bytes!("../tests/fixtures/bench/mandelbrot.spv"), [50, 38, 1], [16, 16, 1], &args);
    }
    println!("breakdown, us");
    breakdown("julia frame", julia, julia_copy, julia_present, julia_cold - julia[0]);
    breakdown("mandelbrot frame", mandelbrot_frame, mandelbrot_copy, mandelbrot_present, mandelbrot_cold);
    bench("pushbuffer triangle 1080p", 2, 100, Some(state), || render_triangle_demo(0.25).unwrap());
    bench("demo post 1080p", 2, 100, Some(state), || render_post_frame(0.25).unwrap());
    bench("demo sprites 1080p", 2, 100, Some(state), || render_sprites_frame(0.25).unwrap());
    bench("demo particles 1080p", 2, 100, Some(state), || render_particles_frame(0.25).unwrap());
    let mut particles = demo_particles::Particles::new(state).unwrap();
    bench("particles update", 2, 100, Some(state), || particles.update(0.25).unwrap());
    bench("particles draw", 2, 100, Some(state), || particles.draw(0.25).unwrap());
    bench("particles present", 2, 100, Some(state), || particles.gpu.present(demo_particles::TARGET, 1920, 1080).unwrap());
    let mut quad = triangle_guest(state, None, 1920, 1080, &[([0.0; 2], [0; 4]); 3]).unwrap();
    let corners = [([-1.0, -1.0], [0, 0, 0, 0]), ([1.0, -1.0], [255, 0, 0, 0]), ([-1.0, 1.0], [0, 255, 0, 0]), ([1.0, 1.0], [255, 255, 0, 0])];
    quad.write(VERTICES, &vertex_bytes(&corners)).unwrap();
    quad.write(TEXTURES + 0x1000, &(0..64 * 64 * 4).map(|i| i as u8).collect::<Vec<_>>()).unwrap();
    texture_header(&mut quad, TEXTURES + 0x1000, 64, 64, 64 * 4).unwrap();
    bench("pushbuffer textured quad 1080p", 2, 100, Some(state), || textured_quad_frame(&mut quad, 1920, 1080, TARGET, 0).unwrap());
    bench("demo bounce 1080p", 2, 100, Some(state), || render_bounce_frame(0.25).unwrap());
    bench("demo boombox 1080p", 2, 100, Some(state), || demo::render_boombox_frame(0.25).unwrap());
    match demo::asset_dir("flighthelmet").exists() {
        true => { bench("demo helmet 1080p", 2, 100, Some(state), || demo::render_helmet_frame(0.25).unwrap()); }
        false => println!("demo helmet 1080p skipped: run demos/assets/fetch.sh"),
    }
    if std::env::var_os("OBOROMI_PERF_NATIVE").is_some() {
        println!("native equivalence, translated vs its GLSL twin through the same ComputePipeline");
        let (groups, iterations) = (1024u32, 64u32);
        let sums = vec![0; groups as usize * 256 * 4];
        let fma_values = [iterations.to_le_bytes(), 0.999f32.to_le_bytes(), 0.001f32.to_le_bytes()];
        let args: Vec<_> = [Arg::Buffer(&sums)].into_iter().chain(fma_values.iter().map(|value| Arg::Value(value))).collect();
        let fma = versus(state, "fma_chain", include_bytes!("../tests/fixtures/bench/fma_chain.spv"), [groups, 1, 1], [256, 1, 1], &args);
        let flops = 2.0 * f64::from(groups * 256) * f64::from(iterations * 256);
        let bytes = 64 << 20;
        let (source, sink, add) = (vec![0; bytes], vec![0; bytes], 1f32.to_le_bytes());
        let stream_args = [Arg::Buffer(&source), Arg::Buffer(&sink), Arg::Value(&add)];
        let stream = versus(state, "stream_add", include_bytes!("../tests/fixtures/bench/stream_add.spv"), [(bytes / 16 / 256) as u32, 1, 1], [256, 1, 1], &stream_args);
        let empty = versus(state, "empty", include_bytes!("../tests/fixtures/bench/empty.spv"), [1, 1, 1], [256, 1, 1], &[Arg::Buffer(&[0; 16])]);
        for (metric, [translated, glsl]) in [("GFLOP/s", fma.map(|us| flops / us / 1e3)), ("GB/s read+write", stream.map(|us| 2.0 * bytes as f64 / us / 1e3)), ("empty kernel gpu us", empty)] {
            println!("{metric:<30}translated {translated:.1}, glsl twin {glsl:.1}, ratio {:.2}", translated / glsl);
        }
    }
    let headless = ash::ext::headless_surface::Instance::new(&state.entry, &state.instance);
    let surface = unsafe { headless.create_headless_surface(&Default::default(), None) }.unwrap();
    attach_surface(surface, w as u32, h as u32).unwrap();
    bench("render_test_frame+swapchain", 2, 20, Some(state), || {
        render_test_frame().unwrap()
    });
}
