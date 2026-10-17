//! Latency of the hot paths in microseconds: `MESA_SHADER_CACHE_DISABLE=true cargo run --release -p oboromi-core --example perf`,
//! where the variable keeps Mesa's own shader cache from warming the cold pipeline rows.
use ash::vk::DescriptorType;
use oboromi_core::display::{attach_surface, present, present_buffer};
use oboromi_core::ffi::oboromi_frame_read;
use oboromi_core::gpu::compute::{ComputePipeline, HostBuffer};
use oboromi_core::gpu::pipeline_cache::Disk;
use oboromi_core::gpu::sm86::{CBUF_BINDING, GLOBAL_BINDING, STATE_WORDS, SUPPORT_BINDING, Stage, emit_module};
use oboromi_core::audio::demo_music::{music, sequence};
use oboromi_core::gpu::{VkState, demo, demo_bounce::render_bounce_frame, demo_particles::render_particles_frame, demo_post::render_post_frame, demo_sprites::render_sprites_frame, surface_device, frame::{julia_program, render_demo_frame, render_test_frame, render_triangle_demo}};
use oboromi_core::tests::gpu::engine::{TARGET, TEXTURES, VERTICES, texture_header, textured_quad_frame, triangle_frame, triangle_guest, vertex_bytes};
use oboromi_core::tests::gpu::{graphics, harness::translate_module};
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
    println!("{name:<30}{iters:>6}{p50:>11.1}{p95:>11.1}{p99:>11.1}{max:>11.1}");
}

fn main() {
    println!("path                               n     p50 us     p95 us     p99 us     max us");
    // KILL is left out: OpKill terminates the block, so nothing may follow it.
    let fixtures = [
        inst::iadd_imm(1, 2, 42),
        inst::iadd_reg(1, 2, 3),
        inst::mov_imm(1, 0xCAFEBABE),
        inst::mov_reg(1, 2),
    ];
    let program: Vec<u128> = fixtures.iter().copied().cycle().take(1000).collect();
    bench("translate 1000 inst", 20, 200, || {
        black_box(translate_module(black_box(&program), true, None));
    });
    let mut song = music();
    bench("audren music period", 20, 2000, || {
        sequence(&mut song);
        black_box(song.period());
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
    let period = vec![0x1234i16; 2 * oboromi_core::audio::PERIOD as usize];
    let mut pulled = period.clone();
    bench("audio latency push->pull", 100, 10000, || {
        oboromi_core::audio::push(&period);
        oboromi_core::audio::pull(&mut pulled);
        black_box(&pulled);
    });
    bench("vk device init (cold)", 0, 1, || {
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
    let program = [inst::mov_imm(0, 42), inst::iadd_reg(1, 2, 3)];
    let words = translate_module(&program, true, Some(state)).words();
    let ssbo = DescriptorType::STORAGE_BUFFER;
    let bindings = [(10, ssbo), (11, ssbo)];
    bench("pipeline create (cold)", 0, 1, || {
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
        bench(&format!("triangle pipeline {start}"), 0, 1, || drop(graphics::triangle_pipeline(&fresh).unwrap()));
        bench(&format!("julia pipeline {start}"), 0, 1, || drop(ComputePipeline::new(&fresh, &julia, &julia_bindings).unwrap()));
        disk.flush(&fresh).unwrap();
    }
    for start in ["cold", "warm"] {
        let fresh = Box::leak(Box::new(VkState::new().unwrap()));
        let mut gpu = triangle_guest(fresh, Some((&root, "engine")), 1920, 1080, &[([0.0, -0.8], [255; 4]), ([0.7, 0.4], [255; 4]), ([-0.7, 0.4], [255; 4])]).unwrap();
        bench(&format!("engine first frame {start}"), 0, 1, || triangle_frame(&mut gpu, 1920, 1080, [1.0, 0.0, 0.0, 1.0], 0).unwrap());
    }
    let _ = std::fs::remove_dir_all(root);
    for (name, libraries) in [("triangle monolithic compile", false), ("triangle libraries + link", true)] {
        let mut fresh = VkState::new().unwrap();
        fresh.graphics_pipeline_library &= libraries;
        bench(name, 0, 1, || drop(graphics::triangle_pipeline(&fresh).unwrap()));
        if fresh.graphics_pipeline_library {
            bench("triangle link only", 0, 1, || drop(graphics::triangle_pipeline(&fresh).unwrap()));
        }
    }
    let mut pipeline = ComputePipeline::new(state, &words, &bindings).unwrap();
    let mut input = HostBuffer::new(state, STATE_WORDS as u64 * 4).unwrap();
    let output = HostBuffer::new(state, STATE_WORDS as u64 * 4).unwrap();
    pipeline.descriptors.bind(&[(10, &input), (11, &output)]);
    let mut out = [0u8; STATE_WORDS * 4];
    bench("dispatch+fence+readback", 50, 1000, || {
        input.write(&[0; STATE_WORDS * 4]).unwrap();
        state.dispatch(&pipeline, [1, 1, 1]).unwrap();
        output.read(black_box(&mut out)).unwrap();
    });
    let frame = HostBuffer::new(state, pixels.len() as u64).unwrap();
    bench("present_buffer 1080p", 10, 200, || {
        present_buffer(&frame, w as u32, h as u32).unwrap()
    });
    let mut triangle = graphics::triangle(state).unwrap();
    let mut pixels = vec![0; (graphics::SIZE * graphics::SIZE * 4) as usize];
    bench("draw+readback 64x64 triangle", 50, 1000, || triangle(black_box(&mut pixels)).unwrap());
    bench("render_test_frame e2e", 2, 20, || {
        render_test_frame().unwrap()
    });
    bench("render_demo_frame e2e", 2, 100, || {
        render_demo_frame(0.25).unwrap()
    });
    bench("pushbuffer triangle 1080p", 2, 100, || render_triangle_demo(0.25).unwrap());
    bench("demo post 1080p", 2, 100, || render_post_frame(0.25).unwrap());
    bench("demo sprites 1080p", 2, 100, || render_sprites_frame(0.25).unwrap());
    bench("demo particles 1080p", 2, 100, || render_particles_frame(0.25).unwrap());
    let mut quad = triangle_guest(state, None, 1920, 1080, &[([0.0; 2], [0; 4]); 3]).unwrap();
    let corners = [([-1.0, -1.0], [0, 0, 0, 0]), ([1.0, -1.0], [255, 0, 0, 0]), ([-1.0, 1.0], [0, 255, 0, 0]), ([1.0, 1.0], [255, 255, 0, 0])];
    quad.write(VERTICES, &vertex_bytes(&corners)).unwrap();
    quad.write(TEXTURES + 0x1000, &(0..64 * 64 * 4).map(|i| i as u8).collect::<Vec<_>>()).unwrap();
    texture_header(&mut quad, TEXTURES + 0x1000, 64, 64, 64 * 4).unwrap();
    bench("pushbuffer textured quad 1080p", 2, 100, || textured_quad_frame(&mut quad, 1920, 1080, TARGET, 0).unwrap());
    bench("demo bounce 1080p", 2, 100, || render_bounce_frame(0.25).unwrap());
    bench("demo boombox 1080p", 2, 100, || demo::render_boombox_frame(0.25).unwrap());
    match demo::asset_dir("flighthelmet").exists() {
        true => bench("demo helmet 1080p", 2, 100, || demo::render_helmet_frame(0.25).unwrap()),
        false => println!("demo helmet 1080p skipped: run demos/assets/fetch.sh"),
    }
    let headless = ash::ext::headless_surface::Instance::new(&state.entry, &state.instance);
    let surface = unsafe { headless.create_headless_surface(&Default::default(), None) }.unwrap();
    attach_surface(surface, w as u32, h as u32).unwrap();
    bench("render_test_frame+swapchain", 2, 20, || {
        render_test_frame().unwrap()
    });
}
