//! Translates every guest shader program in a game's files into the title's disk cache before the game runs:
//! `cargo run -p oboromi-core --example precompile -- <title> <dir-or-file>...`.
use oboromi_core::gpu::bnsh::{self, Program};
use oboromi_core::gpu::{device, pipeline_cache::Disk};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

fn files(path: &Path, out: &mut Vec<PathBuf>) {
    match std::fs::read_dir(path) {
        Ok(entries) => entries.flatten().for_each(|entry| files(&entry.path(), out)),
        Err(_) => out.push(path.to_path_buf()),
    }
}

// The key `Gpu::module` gives a program whose vertex attributes are all fetched as floats.
fn key(program: &Program) -> u64 {
    let mut hasher = DefaultHasher::new();
    match program.stage {
        oboromi_core::gpu::sm86::Stage::Compute { local_size } => (&program.words(), local_size).hash(&mut hasher),
        _ => {
            let sph: [u8; 0x80] = std::array::from_fn(|i| (program.sph[i / 4] >> (8 * (i % 4))) as u8);
            (sph, &program.words(), 0u32, 0u32).hash(&mut hasher);
        }
    }
    hasher.finish()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [title, paths @ ..] = &args[..] else { return eprintln!("usage: precompile <title> <dir-or-file>...") };
    let state = device().expect("no Vulkan device");
    let props = unsafe { state.instance.get_physical_device_properties(state.physical_device) };
    let home = || std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache"));
    let root = std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from).or_else(home).unwrap_or_else(std::env::temp_dir).join("oboromi");
    let disk = Disk::open(&root, title, &props).expect("cache directory");
    std::panic::set_hook(Box::new(|_| {}));
    let mut all = Vec::new();
    paths.iter().for_each(|path| files(Path::new(path), &mut all));
    for path in all {
        let Ok(file) = std::fs::read(&path) else { continue };
        let mut found = bnsh::programs(&file);
        if found.is_empty() {
            found = bnsh::scan(&file);
        }
        let (mut translated, mut cached, mut failed) = (0, 0, 0);
        for program in &found {
            let key = key(program);
            if disk.spirv(key).is_some() {
                cached += 1;
            } else if let Some((words, resources, stores_global)) = bnsh::translate(Some(state), program) {
                translated += 1;
                cached += usize::from(disk.store_spirv(key, &words, &resources, stores_global).is_ok());
            } else {
                failed += 1;
            }
        }
        println!("{}: found {} translated {translated} cached {cached} failed {failed}", path.display(), found.len());
    }
}
