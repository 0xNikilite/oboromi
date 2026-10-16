//! Latency baseline of the hot paths: `cargo run --release -p oboromi-core --example perf [-- --json]`.
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

struct Row {
    name: &'static str,
    us: Vec<f64>,
    per_op: f64,
    unit: &'static str,
}

impl Row {
    fn pct(&self, p: f64) -> f64 {
        self.us[((p * self.us.len() as f64).ceil() as usize).clamp(1, self.us.len()) - 1]
    }
}

fn bench(
    name: &'static str,
    warmup: usize,
    iters: usize,
    per_op: f64,
    unit: &'static str,
    mut f: impl FnMut(),
) -> Row {
    (0..warmup).for_each(|_| f());
    let mut us: Vec<f64> = (0..iters)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed().as_secs_f64() * 1e6
        })
        .collect();
    us.sort_by(f64::total_cmp);
    Row {
        name,
        us,
        per_op,
        unit,
    }
}

fn gpu_rows(rows: &mut Vec<Row>) -> String {
    let t = Instant::now();
    let state = match device() {
        Ok(state) => state,
        Err(e) => return format!("skipped: {e:?}"),
    };
    rows.push(Row {
        name: "vk device init (cold)",
        us: vec![t.elapsed().as_secs_f64() * 1e6],
        per_op: 0.0,
        unit: "",
    });
    let props = unsafe {
        state
            .instance
            .get_physical_device_properties(state.physical_device)
    };
    let words = translate_module(&[inst::mov_imm(0, 42), inst::iadd_reg(1, 2, 3)], true).words();
    let ssbo = DescriptorType::STORAGE_BUFFER;
    let t = Instant::now();
    let mut pipeline = ComputePipeline::new(state, &words, &[(10, ssbo), (11, ssbo)]).unwrap();
    rows.push(Row {
        name: "pipeline create (cold)",
        us: vec![t.elapsed().as_secs_f64() * 1e6],
        per_op: 0.0,
        unit: "",
    });
    let mut input = HostBuffer::new(state, STATE_WORDS as u64 * 4).unwrap();
    let output = HostBuffer::new(state, STATE_WORDS as u64 * 4).unwrap();
    pipeline.bind(&[(10, &input), (11, &output)]);
    let mut out = [0u8; STATE_WORDS * 4];
    rows.push(bench(
        "dispatch+fence+readback",
        50,
        1000,
        1.0,
        "dispatch/s",
        || {
            input.write(&[0; STATE_WORDS * 4]).unwrap();
            state.dispatch(&pipeline, [1, 1, 1]).unwrap();
            output.read(black_box(&mut out)).unwrap();
        },
    ));
    drop((input, output, pipeline));
    let name = props
        .device_name_as_c_str()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    rows.push(bench(
        "render_test_frame e2e",
        2,
        20,
        1.0,
        "frame/s",
        || render_test_frame().unwrap(),
    ));
    name
}

fn main() {
    let json = std::env::args().any(|a| a == "--json");
    // KILL is left out: OpKill terminates the block, so nothing may follow it.
    let fixtures = [
        inst::iadd_imm(1, 2, 42),
        inst::iadd_reg(1, 2, 3),
        inst::iadd3_reg(1, 2, 3, 0xFF),
        inst::mov_imm(1, 0xCAFEBABE),
        inst::mov_reg(1, 2),
    ];
    let program: Vec<u128> = fixtures.iter().copied().cycle().take(1000).collect();
    let mut rows = vec![bench(
        "translate 1000 inst",
        20,
        200,
        1000.0,
        "inst/s",
        || {
            black_box(translate_module(black_box(&program), true));
        },
    )];
    let (w, h) = (1920, 1080);
    let pixels = vec![0x80u8; w * h * 4];
    let mut dst = vec![0u8; pixels.len()];
    rows.push(bench(
        "present+frame_read 1080p",
        10,
        200,
        pixels.len() as f64 / 1e6,
        "MB/s",
        || {
            present(w as u32, h as u32, &pixels);
            let (mut fw, mut fh) = (0, 0);
            unsafe { oboromi_frame_read(dst.as_mut_ptr(), dst.len(), &mut fw, &mut fh) };
            black_box(&dst);
        },
    ));
    let device = gpu_rows(&mut rows);

    let mut table = format!(
        "device: {device}\n{:<26}{:>6}{:>11}{:>11}{:>11}{:>11}  throughput@p50\n",
        "path", "n", "p50 us", "p95 us", "p99 us", "max us"
    );
    let mut entries = Vec::new();
    for r in &rows {
        let [p50, p95, p99, max] = [0.5, 0.95, 0.99, 1.0].map(|p| r.pct(p));
        let tput = r.per_op * 1e6 / p50;
        let tput_col = if r.unit.is_empty() {
            String::new()
        } else {
            format!("{tput:.1} {}", r.unit)
        };
        table += &format!(
            "{:<26}{:>6}{p50:>11.1}{p95:>11.1}{p99:>11.1}{max:>11.1}  {tput_col}\n",
            r.name,
            r.us.len()
        );
        entries.push(format!(
            r#"{{"name":{:?},"n":{},"p50_us":{p50:.1},"p95_us":{p95:.1},"p99_us":{p99:.1},"max_us":{max:.1},"throughput":{tput:.1},"unit":{:?}}}"#,
            r.name, r.us.len(), r.unit
        ));
    }
    if json {
        eprint!("{table}");
        println!(r#"{{"device":{device:?},"rows":[{}]}}"#, entries.join(","));
    } else {
        print!("{table}");
    }
}
