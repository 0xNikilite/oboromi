//! Replay an Ampere command list from saved guest memory.
use oboromi_core::gpu::State;
use std::{
    env,
    ffi::OsStr,
    fs,
    io::{self, Write},
    path::Path,
    process::ExitCode,
};

const USAGE: &str =
    "usage: replay_gpfifo ENTRIES.bin READBACK_VA SIZE MAP_VA FILE [MAP_VA FILE ...]";

fn number(value: &OsStr) -> Result<u64, String> {
    let value = value
        .to_str()
        .ok_or_else(|| format!("invalid integer {value:?}"))?;
    let parsed = match value.strip_prefix("0x") {
        Some(hex) => u64::from_str_radix(hex, 16),
        None => value.parse(),
    };
    parsed.map_err(|_| format!("invalid unsigned integer {value:?}"))
}

fn load(path: &OsStr) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("{}: {e}", Path::new(path).display()))
}

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() < 5 || !args[3..].chunks_exact(2).remainder().is_empty() {
        return Err(USAGE.into());
    }
    let va = number(&args[1])?;
    let size = usize::try_from(number(&args[2])?).map_err(|_| "readback too large")?;
    let bytes = load(&args[0])?;
    if bytes.len() % 8 != 0 {
        return Err("truncated 64-bit GPFIFO entry".into());
    }
    let entries: Vec<_> = bytes
        .chunks_exact(8)
        .map(|chunk| u64::from_le_bytes(chunk.try_into().unwrap()))
        .collect();
    let mut gpu = State::new();
    for pair in args[3..].chunks_exact(2) {
        gpu.address_space
            .map(number(&pair[0])?, load(&pair[1])?)
            .map_err(|e| format!("mapping {:?} at {:?}: {e:?}", pair[1], pair[0]))?;
    }
    gpu.submit_gpfifo(&entries).map_err(|e| {
        format!(
            "entry {}, pushbuffer VA {:#x}: {:?}",
            e.entry, e.pushbuffer_va, e.fault
        )
    })?;
    let output = gpu
        .address_space
        .read(va, size)
        .map_err(|e| format!("readback: {e:?}"))?;
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(output)
        .and_then(|()| stdout.flush())
        .map_err(|e| e.to_string())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
