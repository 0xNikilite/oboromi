//! Renders a demo offline at fixed time steps, frame i at i / fps seconds, as `frame_%04d.ppm` in a directory:
//! `cargo run --release -p oboromi-core --example capture -- <demo> [--seconds 8] [--fps 60] --out <dir>`, then
//! `ffmpeg -framerate 60 -i <dir>/frame_%04d.ppm -c:v libx264 -crf 20 -pix_fmt yuv420p -movflags +faststart demo.mp4`.
use oboromi_core::ffi::oboromi_frame_read;
use oboromi_core::gpu::{demo, demo_post, demo_sprites, frame};

fn render(demo: &str, seconds: f32) -> Result<(), String> {
    match demo {
        "julia" => frame::render_demo_frame(seconds).map_err(|e| format!("{e:?}")),
        "triangle" => frame::render_triangle_demo(seconds),
        "sprites" => demo_sprites::render_sprites_frame(seconds),
        "post" => demo_post::render_post_frame(seconds),
        "boombox" => demo::render_boombox_frame(seconds),
        "helmet" => demo::render_helmet_frame(seconds),
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
    for i in 0..(seconds * fps).round() as usize {
        render(&args[0], i as f32 / fps).unwrap();
        let (mut width, mut height) = (0, 0);
        pixels.resize(unsafe { oboromi_frame_read(std::ptr::null_mut(), 0, &mut width, &mut height) }, 0);
        unsafe { oboromi_frame_read(pixels.as_mut_ptr(), pixels.len(), &mut width, &mut height) };
        let rgb: Vec<u8> = pixels.chunks(4).flat_map(|pixel| &pixel[..3]).copied().collect();
        std::fs::write(out.join(format!("frame_{i:04}.ppm")), [format!("P6\n{width} {height}\n255\n").as_bytes(), &rgb].concat()).unwrap();
    }
}
