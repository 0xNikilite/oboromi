//! Renders a demo offline at fixed time steps, frame i at i / fps seconds, as `frame_%04d.ppm` in a directory:
//! `cargo run --release -p oboromi-core --example capture -- <demo> [--seconds 8] [--fps 60] --out <dir>`, then
//! `ffmpeg -framerate 60 -i <dir>/frame_%04d.ppm -c:v libx264 -crf 20 -pix_fmt yuv420p -movflags +faststart demo.mp4`.
use oboromi_core::audio::{CHANNELS, PERIOD, RATE, demo_music, pull, queued};
use oboromi_core::ffi::oboromi_frame_read;
use oboromi_core::gpu::{demo, demo_bounce, demo_dungeon, demo_particles, demo_post, demo_sprites, frame};
use std::io::{Seek, Write};

fn render(demo: &str, seconds: f32) -> Result<(), String> {
    match demo {
        "dungeon" => demo_dungeon::render_dungeon_frame(seconds),
        "julia" => frame::render_demo_frame(seconds).map_err(|e| format!("{e:?}")),
        "triangle" => frame::render_triangle_demo(seconds),
        "sprites" => demo_sprites::render_sprites_frame(seconds),
        "post" => demo_post::render_post_frame(seconds),
        "boombox" => demo::render_boombox_frame(seconds),
        "raytraced" => demo::render_raytraced_frame(seconds),
        "raytraced-off" => { demo::set_raytracing_enabled(false); demo::render_raytraced_frame(seconds) },
        "helmet" => demo::render_helmet_frame(seconds),
        "particles" => demo_particles::render_particles_frame(seconds),
        "bounce" => demo_bounce::render_bounce_frame(seconds),
        "music" => {
            demo_music::play_music(seconds);
            Ok(())
        }
        _ => Err(format!("unknown demo {demo}")),
    }
}

struct Wav {
    file: std::fs::File,
    frames: usize,
}

impl Wav {
    fn new(path: impl AsRef<std::path::Path>) -> Self {
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(b"RIFF\0\0\0\0WAVEfmt \x10\0\0\0\x01\0\x02\0\x80\xbb\0\0\0\xee\x02\0\x04\0\x10\0data\0\0\0\0").unwrap();
        Self { file, frames: 0 }
    }

    fn drain(&mut self) {
        let frames = queued();
        let mut samples = vec![0; frames * CHANNELS as usize];
        assert_eq!(pull(&mut samples), frames);
        self.file
            .write_all(
                &samples
                    .iter()
                    .flat_map(|sample| sample.to_le_bytes())
                    .collect::<Vec<_>>(),
            )
            .unwrap();
        self.frames += frames;
    }

    fn finish(mut self, expected: usize) {
        assert!(
            self.frames.abs_diff(expected) <= PERIOD as usize,
            "{} audio frames for {expected} expected",
            self.frames
        );
        let bytes = (self.frames * CHANNELS as usize * size_of::<i16>()) as u32;
        self.file.seek(std::io::SeekFrom::Start(4)).unwrap();
        self.file.write_all(&(36 + bytes).to_le_bytes()).unwrap();
        self.file.seek(std::io::SeekFrom::Start(40)).unwrap();
        self.file.write_all(&bytes.to_le_bytes()).unwrap();
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |name: &str| {
        args.iter()
            .position(|arg| arg == name)
            .map(|i| args[i + 1].clone())
    };
    if args[0] == "dungeon" {
        demo_dungeon::set_dungeon_capture();
        demo_dungeon::render::PROFILE.store(args.iter().any(|a|a=="--profile"), std::sync::atomic::Ordering::Relaxed);
        if let Some(debug) = flag("--water-debug") {
            let bit=match debug.as_str(){"normal"=>16,"reflection"=>32,"refraction"=>64,_=>panic!("--water-debug normal|reflection|refraction")};
            demo_dungeon::render::DISABLED.fetch_or(bit, std::sync::atomic::Ordering::Relaxed);
        }
        if let Some(disabled) = flag("--disable") {
            let mask=disabled.split(',').map(|effect|match effect {"fire"=>1,"water"=>2,"bloom"=>4,"fog"=>8,_=>panic!("unknown effect {effect}")}).fold(0,|a,b|a|b);
            demo_dungeon::render::DISABLED.fetch_or(mask, std::sync::atomic::Ordering::Relaxed);
        }
        if let Some(quality) = flag("--quality") {
            assert!(quality == "realtime" || quality == "showcase", "--quality realtime|showcase");
            demo_dungeon::render::SHOWCASE.store(quality == "showcase", std::sync::atomic::Ordering::Relaxed);
            demo_dungeon::render::RAYS.store(quality == "showcase", std::sync::atomic::Ordering::Relaxed);
        }
        if let Some(rays) = flag("--raytracing") { demo_dungeon::render::RAYS.store(rays == "on", std::sync::atomic::Ordering::Relaxed); }
        if let Some(view) = flag("--view") { demo_dungeon::VIEW.store(view.parse().unwrap(), std::sync::atomic::Ordering::Relaxed); }
    }
    let seconds: f32 = flag("--seconds").map_or(8.0, |s| s.parse().unwrap());
    let start: f32 = flag("--start").map_or(0.0, |s| s.parse().unwrap());
    let fps: f32 = flag("--fps").map_or(60.0, |s| s.parse().unwrap());
    let out = std::path::PathBuf::from(flag("--out").expect("--out <dir>"));
    std::fs::create_dir_all(&out).unwrap();
    let mut wav = flag("--audio").map(Wav::new);
    let audio_only = args.iter().any(|arg| arg == "--audio-only");
    if let Some(solo) = flag("--solo") {
        demo_dungeon::set_dungeon_audio_solo(&solo);
    }
    let mut pixels = Vec::new();
    let mut timings = Vec::new();
    let frames = (seconds * fps).round() as usize;
    for i in 0..frames {
        let tick = std::time::Instant::now();
        let seconds = start + (i + usize::from(args[0] == "dungeon")) as f32 / fps;
        if audio_only && args[0] == "dungeon" {
            demo_dungeon::render_dungeon_audio_frame(seconds).unwrap();
        } else {
            render(&args[0], seconds).unwrap();
        }
        timings.push(tick.elapsed().as_secs_f64() * 1000.0);
        if args.iter().any(|a| a == "--profile") {
            eprintln!("dungeon_cpu,{i},{:.4}", timings.last().unwrap());
        }
        if let Some(wav) = &mut wav { wav.drain(); }
        if audio_only {
            continue;
        }
        let (mut width, mut height) = (0, 0);
        pixels.resize(
            unsafe { oboromi_frame_read(std::ptr::null_mut(), 0, &mut width, &mut height) },
            0,
        );
        unsafe { oboromi_frame_read(pixels.as_mut_ptr(), pixels.len(), &mut width, &mut height) };
        let rgb: Vec<u8> = pixels
            .chunks(4)
            .flat_map(|pixel| &pixel[..3])
            .copied()
            .collect();
        std::fs::write(
            out.join(format!("frame_{i:04}.ppm")),
            [format!("P6\n{width} {height}\n255\n").as_bytes(), &rgb].concat(),
        )
        .unwrap();
    }
    if let Some(wav) = wav {
        wav.finish((frames as f64 * f64::from(RATE) / f64::from(fps)).round() as usize);
    }
    if timings.len() > 1 {
        timings.remove(0);
        timings.sort_by(f64::total_cmp);
        eprintln!("frame_ms,median,{:.3},p95,{:.3}", timings[timings.len() / 2], timings[(timings.len() * 95 / 100).min(timings.len() - 1)]);
    }
}
