#![allow(dead_code)]

use crate::mmu::{Memory, MemoryError, Permission};
use byteorder::{ByteOrder, LittleEndian};

pub const R_AARCH64_RELATIVE: u32 = 1027;
pub const DT_NULL: i64 = 0;
pub const DT_RELA: i64 = 7;
pub const DT_RELASZ: i64 = 8;

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("File is too small to contain an NRO header")]
    TooSmall,

    #[error("Invalid NRO magic: expected 'NRO0', got {0:?}")]
    InvalidMagic([u8; 4]),

    #[error("Memory mapping error: {0}")]
    Memory(#[from] MemoryError),
}

#[derive(Debug, Clone)]
pub struct NroHeader {
    pub version: u32,
    pub size: u32,
    pub flags: u32,
    pub text_offset: u32,
    pub text_size: u32,
    pub rodata_offset: u32,
    pub rodata_size: u32,
    pub data_offset: u32,
    pub data_size: u32,
    pub bss_size: u32,
    pub relocations_applied: usize,
}

pub struct NroLoader;

impl NroLoader {
    /// Parses an NRO0 header from raw file bytes.
    pub fn parse_header(bytes: &[u8]) -> Result<NroHeader, LoadError> {
        if bytes.len() < 0x80 {
            return Err(LoadError::TooSmall);
        }

        let magic = &bytes[0x10..0x14];
        if magic != b"NRO0" {
            let mut arr = [0u8; 4];
            arr.copy_from_slice(magic);
            return Err(LoadError::InvalidMagic(arr));
        }

        Ok(NroHeader {
            version: LittleEndian::read_u32(&bytes[0x14..0x18]),
            size: LittleEndian::read_u32(&bytes[0x18..0x1C]),
            flags: LittleEndian::read_u32(&bytes[0x1C..0x20]),
            text_offset: LittleEndian::read_u32(&bytes[0x20..0x24]),
            text_size: LittleEndian::read_u32(&bytes[0x24..0x28]),
            rodata_offset: LittleEndian::read_u32(&bytes[0x28..0x2C]),
            rodata_size: LittleEndian::read_u32(&bytes[0x2C..0x30]),
            data_offset: LittleEndian::read_u32(&bytes[0x30..0x34]),
            data_size: LittleEndian::read_u32(&bytes[0x34..0x38]),
            bss_size: LittleEndian::read_u32(&bytes[0x38..0x3C]),
            relocations_applied: 0,
        })
    }

    /// Loads an NRO executable into virtual memory, configures segments, and applies MOD0 relocations.
    pub fn load_into_memory(
        bytes: &[u8],
        base_addr: u64,
        mmu: &mut Memory,
    ) -> Result<NroHeader, LoadError> {
        let mut header = Self::parse_header(bytes)?;

        // 1. Map and write .text segment (Read + Write + Execute)
        let text_start = header.text_offset as usize;
        let text_end = text_start + header.text_size as usize;
        if text_end <= bytes.len() && header.text_size > 0 {
            mmu.map_range(
                base_addr,
                header.text_size as usize,
                Permission::READ_WRITE_EXECUTE,
            );
            mmu.write_bytes(base_addr, &bytes[text_start..text_end])?;
        }

        // 2. Map and write .rodata segment
        let rodata_addr = base_addr + header.rodata_offset as u64;
        let ro_start = header.rodata_offset as usize;
        let ro_end = ro_start + header.rodata_size as usize;
        if ro_end <= bytes.len() && header.rodata_size > 0 {
            mmu.map_range(
                rodata_addr,
                header.rodata_size as usize,
                Permission::READ_WRITE,
            );
            mmu.write_bytes(rodata_addr, &bytes[ro_start..ro_end])?;
        }

        // 3. Map and write .data segment + .bss (Read + Write)
        let data_addr = base_addr + header.data_offset as u64;
        let data_start = header.data_offset as usize;
        let data_end = data_start + header.data_size as usize;
        let total_data_size = (header.data_size + header.bss_size) as usize;

        if data_end <= bytes.len() && total_data_size > 0 {
            mmu.map_range(data_addr, total_data_size, Permission::READ_WRITE);
            mmu.write_bytes(data_addr, &bytes[data_start..data_end])?;
        }

        // 4. Parse MOD0 header and apply dynamic R_AARCH64_RELATIVE relocations if present
        header.relocations_applied = Self::apply_mod0_relocations(base_addr, mmu);

        Ok(header)
    }

    /// Loads an uncompressed Nintendo Shared Object (`NSO0`) executable into virtual memory
    /// (such as Switch 2 `RO` / `FS` system modules and retail ExeFS `main` binaries).
    pub fn load_nso_into_memory(
        bytes: &[u8],
        base_addr: u64,
        mmu: &mut Memory,
    ) -> Result<NroHeader, LoadError> {
        if bytes.len() < 0x100 {
            return Err(LoadError::TooSmall);
        }
        let magic = &bytes[0x00..0x04];
        if magic != b"NSO0" {
            let mut arr = [0u8; 4];
            arr.copy_from_slice(magic);
            return Err(LoadError::InvalidMagic(arr));
        }

        let text_file_off = LittleEndian::read_u32(&bytes[0x10..0x14]) as usize;
        let text_mem_off = LittleEndian::read_u32(&bytes[0x14..0x18]) as u64;
        let text_size = LittleEndian::read_u32(&bytes[0x18..0x1C]) as usize;

        let ro_file_off = LittleEndian::read_u32(&bytes[0x20..0x24]) as usize;
        let ro_mem_off = LittleEndian::read_u32(&bytes[0x24..0x28]) as u64;
        let ro_size = LittleEndian::read_u32(&bytes[0x28..0x2C]) as usize;

        let data_file_off = LittleEndian::read_u32(&bytes[0x30..0x34]) as usize;
        let data_mem_off = LittleEndian::read_u32(&bytes[0x34..0x38]) as u64;
        let data_size = LittleEndian::read_u32(&bytes[0x38..0x3C]) as usize;
        let bss_size = LittleEndian::read_u32(&bytes[0x3C..0x40]) as usize;

        if text_file_off + text_size <= bytes.len() && text_size > 0 {
            mmu.map_range(
                base_addr + text_mem_off,
                text_size,
                Permission::READ_WRITE_EXECUTE,
            );
            mmu.write_bytes(
                base_addr + text_mem_off,
                &bytes[text_file_off..text_file_off + text_size],
            )?;
        }

        if ro_file_off + ro_size <= bytes.len() && ro_size > 0 {
            mmu.map_range(base_addr + ro_mem_off, ro_size, Permission::READ_WRITE);
            mmu.write_bytes(
                base_addr + ro_mem_off,
                &bytes[ro_file_off..ro_file_off + ro_size],
            )?;
        }

        let total_data = data_size + bss_size;
        if data_file_off + data_size <= bytes.len() && total_data > 0 {
            mmu.map_range(base_addr + data_mem_off, total_data, Permission::READ_WRITE);
            if data_size > 0 {
                mmu.write_bytes(
                    base_addr + data_mem_off,
                    &bytes[data_file_off..data_file_off + data_size],
                )?;
            }
        }

        let relocations_applied = Self::apply_mod0_relocations(base_addr, mmu);

        Ok(NroHeader {
            version: LittleEndian::read_u32(&bytes[0x04..0x08]),
            size: bytes.len() as u32,
            flags: LittleEndian::read_u32(&bytes[0x0C..0x10]),
            text_offset: text_mem_off as u32,
            text_size: text_size as u32,
            rodata_offset: ro_mem_off as u32,
            rodata_size: ro_size as u32,
            data_offset: data_mem_off as u32,
            data_size: data_size as u32,
            bss_size: bss_size as u32,
            relocations_applied,
        })
    }

    /// Inspects the MOD0 structure at `*(base_addr + 4)` and applies `.rela.dyn` relocations.
    pub fn apply_mod0_relocations(base_addr: u64, mmu: &mut Memory) -> usize {
        let mod0_rel = match mmu.read_u32(base_addr + 4) {
            Ok(off) if off > 0 => off as u64,
            _ => return 0,
        };

        let mod0_addr = base_addr + mod0_rel;
        let mut magic = [0u8; 4];
        if mmu.read_bytes(mod0_addr, &mut magic).is_err() || &magic != b"MOD0" {
            return 0;
        }

        let dyn_rel = mmu.read_u32(mod0_addr + 4).unwrap_or(0) as i32 as i64;
        let dyn_addr = (mod0_addr as i64 + dyn_rel) as u64;

        let mut rela_offset: Option<u64> = None;
        let mut rela_size: u64 = 0;

        // Walk Elf64_Dyn entries (16 bytes each: d_tag i64, d_val u64)
        for i in 0..128 {
            let entry_addr = dyn_addr + (i * 16) as u64;
            let d_tag = match mmu.read_u64(entry_addr) {
                Ok(t) => t as i64,
                Err(_) => break,
            };
            if d_tag == DT_NULL {
                break;
            }
            let d_val = mmu.read_u64(entry_addr + 8).unwrap_or(0);
            match d_tag {
                DT_RELA => rela_offset = Some(d_val),
                DT_RELASZ => rela_size = d_val,
                _ => {}
            }
        }

        let rela_base = match rela_offset {
            Some(off) => base_addr + off,
            None => return 0,
        };

        let num_entries = (rela_size / 24) as usize;
        let mut applied = 0;

        for i in 0..num_entries {
            let entry_addr = rela_base + (i * 24) as u64;
            let r_offset = mmu.read_u64(entry_addr).unwrap_or(0);
            let r_info = mmu.read_u64(entry_addr + 8).unwrap_or(0);
            let r_addend = mmu.read_u64(entry_addr + 16).unwrap_or(0) as i64;

            let r_type = (r_info & 0xFFFF_FFFF) as u32;
            if r_type == R_AARCH64_RELATIVE {
                let target_va = base_addr + r_offset;
                let relocated_val = (base_addr as i64 + r_addend) as u64;
                if mmu.write_u64(target_va, relocated_val).is_ok() {
                    applied += 1;
                }
            }
        }

        applied
    }

    /// Helper to assemble a minimal valid NRO binary with custom text bytes for testing.
    pub fn build_synthetic_nro(text_code: &[u8]) -> Vec<u8> {
        let mut binary = vec![0u8; 0x80 + text_code.len()];

        // 0x00: Branch instruction B #0x80 (0x14000020)
        LittleEndian::write_u32(&mut binary[0x00..0x04], 0x14000020);
        // 0x04: MOD0 offset
        LittleEndian::write_u32(&mut binary[0x04..0x08], 0x00000000);

        // 0x10: Magic "NRO0"
        binary[0x10..0x14].copy_from_slice(b"NRO0");
        // 0x14: Version = 0
        LittleEndian::write_u32(&mut binary[0x14..0x18], 0);
        // 0x18: Total size
        let total_len = binary.len() as u32;
        LittleEndian::write_u32(&mut binary[0x18..0x1C], total_len);
        // 0x1C: Flags
        LittleEndian::write_u32(&mut binary[0x1C..0x20], 0);

        // 0x20: .text offset = 0x00 (includes NRO start header)
        LittleEndian::write_u32(&mut binary[0x20..0x24], 0x00);
        // 0x24: .text size
        let text_len = binary.len() as u32;
        LittleEndian::write_u32(&mut binary[0x24..0x28], text_len);

        // Copy text code at 0x80
        binary[0x80..0x80 + text_code.len()].copy_from_slice(text_code);

        binary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mod0_relocations() {
        let mut mmu = Memory::new();
        let base_addr = 0x80000000u64;
        mmu.map_range(base_addr, 0x2000, Permission::READ_WRITE_EXECUTE);

        // Set MOD0 offset at base + 0x04 -> points to 0x100
        mmu.write_u32(base_addr + 4, 0x100).unwrap();
        // Write "MOD0" at base + 0x100 and dynamic offset +0x20 -> 0x120
        mmu.write_bytes(base_addr + 0x100, b"MOD0").unwrap();
        mmu.write_u32(base_addr + 0x104, 0x20).unwrap();

        // Write .dynamic table at base + 0x120:
        // Entry 0: DT_RELA (7) = 0x200
        mmu.write_u64(base_addr + 0x120, DT_RELA as u64).unwrap();
        mmu.write_u64(base_addr + 0x128, 0x200).unwrap();
        // Entry 1: DT_RELASZ (8) = 24 (1 entry)
        mmu.write_u64(base_addr + 0x130, DT_RELASZ as u64).unwrap();
        mmu.write_u64(base_addr + 0x138, 24).unwrap();
        // Entry 2: DT_NULL (0)
        mmu.write_u64(base_addr + 0x140, 0).unwrap();

        // Write Elf64_Rela entry at base + 0x200:
        // r_offset = 0x300, r_info = R_AARCH64_RELATIVE (1027), r_addend = 0x450
        mmu.write_u64(base_addr + 0x200, 0x300).unwrap();
        mmu.write_u64(base_addr + 0x208, R_AARCH64_RELATIVE as u64).unwrap();
        mmu.write_u64(base_addr + 0x210, 0x450).unwrap();

        let count = NroLoader::apply_mod0_relocations(base_addr, &mut mmu);
        assert_eq!(count, 1);

        // Verify pointer at base + 0x300 was relocated to base_addr + 0x450 (0x80000450)
        assert_eq!(mmu.read_u64(base_addr + 0x300).unwrap(), 0x80000450);
    }
}
