//! Homebrew NRO executables (switchbrew NRO) started with the loader config of switchbrew Homebrew_ABI, with an
//! fsp-srv whose SD card is the `sdmc` directory next to the file.
use crate::cpu::{GuestRam, UnicornCPU};
use crate::guest::load_segments;
use crate::kernel::{CODE, Kernel};
use crate::nn::fs::FileSystemProxy;
use std::cell::RefCell;
use std::fs;
use std::io::{Error, ErrorKind, Result};
use std::path::Path;
use std::rc::Rc;

const PAGE: usize = 0x1000;
/// The kernel has no thread objects, so the main thread is the current-thread pseudo-handle.
const CURRENT_THREAD: u64 = 0xFFFF_8000;
const APPLET_APPLICATION: u64 = 0;
/// 22.5.0, the SystemVersion of the one firmware dump we checked.
const HOS_VERSION: u64 = 22 << 16 | 5 << 8;
/// `svc 0x07`: libnx returns to the loader through the entry's x30 instead of calling ExitProcess itself.
const EXIT_PROCESS: u32 = 0xD400_00E1;
const EXIT_AT: usize = 0x100;
const ARGV_AT: usize = 0x110;

fn invalid() -> Error {
    Error::new(ErrorKind::InvalidData, "not an NRO that fits guest memory")
}

fn word<const N: usize>(bytes: &[u8], at: usize) -> Result<usize> {
    let field = bytes.get(at..at + N).ok_or_else(invalid)?;
    Ok(field.iter().rev().fold(0, |value, &b| value << 8 | usize::from(b)))
}

/// ConfigEntry {key, flags, value[2]} records followed by the exit stub and the argv string.
fn config(base: u64, argv: &str) -> Vec<u8> {
    let entries = [[1, CURRENT_THREAD, 0], [5, 0, base + ARGV_AT as u64], [7, APPLET_APPLICATION, 0], [16, HOS_VERSION, 0], [0; 3]];
    let mut page: Vec<u8> = entries.iter().flatten().flat_map(|v| v.to_le_bytes()).collect();
    page.resize(EXIT_AT, 0);
    page.extend(EXIT_PROCESS.to_le_bytes());
    page.resize(ARGV_AT, 0);
    page.extend(argv.as_bytes());
    page.push(0);
    page
}

/// The RomFS in the asset section after the NRO's `size` bytes, if it has one.
fn romfs(nro: &[u8], size: usize) -> Rc<[u8]> {
    let asset = &nro[size.min(nro.len())..];
    let section = (word::<8>(asset, 0x28), word::<8>(asset, 0x30));
    let romfs = match section {
        (Ok(offset), Ok(len)) if asset.starts_with(b"ASET") => offset.checked_add(len).and_then(|end| asset.get(offset..end)),
        _ => None,
    };
    romfs.unwrap_or_default().into()
}

/// Loads the NRO at `path` at the start of the code region and points the kernel's thread at it.
pub fn load(ram: &GuestRam, cpu: &UnicornCPU, kernel: &mut Kernel, path: &Path) -> Result<()> {
    let nro = fs::read(path)?;
    if nro.get(0x10..0x14) != Some(b"NRO0") {
        return Err(invalid());
    }
    let segment = |i: usize| -> Result<(u64, &[u8])> {
        let (offset, size) = (word::<4>(&nro, 0x20 + 8 * i)?, word::<4>(&nro, 0x24 + 8 * i)?);
        Ok((offset as u64, nro.get(offset..offset.checked_add(size).ok_or_else(invalid)?).ok_or_else(invalid)?))
    };
    let [text, ro, data] = [segment(0)?, segment(1)?, segment(2)?];
    let bss_at = data.0 as usize + data.1.len();
    let bss = vec![0; word::<4>(&nro, 0x38)?];
    let config_at = (bss_at + bss.len()).next_multiple_of(PAGE);
    let name = path.file_name().map(|name| name.to_string_lossy()).unwrap_or_default();
    let config = config(CODE.base + config_at as u64, &format!("sdmc:/{name}"));
    let segments = [text, ro, data, (bss_at as u64, &bss), (config_at as u64, &config)];
    load_segments(ram, CODE.base, &segments).ok_or_else(invalid)?;

    kernel.start(cpu, CODE.base, (config_at + config.len()) as u64);
    cpu.set_x(0, CODE.base + config_at as u64);
    cpu.set_x(1, u64::MAX);
    cpu.set_x(30, CODE.base + (config_at + EXIT_AT) as u64);
    let sd = path.with_file_name("sdmc");
    fs::create_dir_all(&sd)?;
    let romfs = romfs(&nro, word::<4>(&nro, 0x18)?);
    kernel.sm.borrow_mut().0.insert("fsp-srv".into(), Rc::new(RefCell::new(FileSystemProxy { sd, romfs })));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::cpu_manager::CpuManager;

    // homebrew.nro is what scripts/build-guest.sh builds from guest/src/bin/homebrew.rs.
    #[test]
    fn a_homebrew_nro_reads_its_loader_config_and_writes_the_sd_card() {
        let dir = std::env::temp_dir().join(format!("oboromi-nro-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("homebrew.nro"), include_bytes!("../../tests/fixtures/guest/homebrew.nro")).unwrap();
        let cpus = CpuManager::new();
        let cpu = cpus.get_core(0).unwrap();
        let mut kernel = Kernel::default();
        load(&cpus.ram, cpu, &mut kernel, &dir.join("homebrew.nro")).unwrap();
        assert!(kernel.run(cpu), "guest stopped at {:#x} without ExitProcess", cpu.get_pc());
        assert_eq!(kernel.debug_output, ["sdmc:/homebrew.nro", "applet type 0"]);
        assert_eq!(fs::read(dir.join("sdmc/hello.txt")).unwrap(), b"hello from the sd card");
        fs::remove_dir_all(dir).unwrap();
    }
}
