//! Renders a demo offline at fixed time steps, frame i at i / fps seconds, as `frame_%04d.ppm` in a directory:
//! `cargo run --release -p oboromi-core --example capture -- <demo> [--seconds 8] [--fps 60] --out <dir>`, then
//! `ffmpeg -framerate 60 -i <dir>/frame_%04d.ppm -c:v libx264 -crf 20 -pix_fmt yuv420p -movflags +faststart demo.mp4`.
use oboromi_core::ffi::oboromi_frame_read;
use oboromi_core::gpu::{demo, demo_dungeon, demo_particles, demo_post, demo_sprites, frame};

fn render(demo: &str, seconds: f32) -> Result<(), String> {
    match demo {
        "dungeon" => demo_dungeon::render_dungeon_frame(seconds),
        "julia" => frame::render_demo_frame(seconds).map_err(|e| format!("{e:?}")),
        "triangle" => frame::render_triangle_demo(seconds),
        "sprites" => demo_sprites::render_sprites_frame(seconds),
        "post" => demo_post::render_post_frame(seconds),
        "boombox" => demo::render_boombox_frame(seconds),
        "raytraced" => demo::render_raytraced_frame(seconds),
        "helmet" => demo::render_helmet_frame(seconds),
        "particles" => demo_particles::render_particles_frame(seconds),
        _ => Err(format!("unknown demo {demo}")),
    }

}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |name: &str| args.iter().position(|arg| arg == name).map(|i| args[i + 1].clone());
    let seconds: f32 = flag("--seconds").map_or(8.0, |s| s.parse().unwrap());
    let fps: f32 = flag("--fps").map_or(60.0, |s| s.parse().unwrap());
    let out = std::path::PathBuf::from(flag("--out").expect("--out <dir>"));
    std::fs::create_dir_all(&out).unwrap();
    let mut pixels = Vec::new();
    let mut timings = Vec::new();
    let mut audio = flag("--audio-out").map(|p| std::fs::File::create(p).unwrap());
    for i in 0..(seconds * fps).round() as usize {
        let start = std::time::Instant::now();
        render(&args[0], (i + usize::from(args[0] == "dungeon")) as f32 / fps).unwrap();
        timings.push(start.elapsed().as_secs_f64() * 1000.0);
        if let Some(audio) = audio.as_mut() {
            use std::io::Write;
            let mut samples = vec![0; oboromi_core::audio::queued() * 2];
            let count = oboromi_core::audio::pull(&mut samples);
            audio.write_all(&samples[..count * 2].iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>()).unwrap();
        }
        let (mut width, mut height) = (0, 0);
        pixels.resize(unsafe { oboromi_frame_read(std::ptr::null_mut(), 0, &mut width, &mut height) }, 0);
        unsafe { oboromi_frame_read(pixels.as_mut_ptr(), pixels.len(), &mut width, &mut height) };
        let rgb: Vec<u8> = pixels.chunks(4).flat_map(|pixel| &pixel[..3]).copied().collect();
        std::fs::write(out.join(format!("frame_{i:04}.ppm")), [format!("P6\n{width} {height}\n255\n").as_bytes(), &rgb].concat()).unwrap();
    }
    timings.sort_by(f64::total_cmp);
    eprintln!("frames={} median_ms={:.3} p95_ms={:.3}", timings.len(), timings[timings.len() / 2], timings[(timings.len() - 1) * 95 / 100]);
}
