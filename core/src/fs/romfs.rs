#![allow(dead_code)]

use crate::mmu::{Memory, MemoryError};
use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
pub enum VfsError {
    #[error("File not found in VFS: {0}")]
    NotFound(String),

    #[error("Invalid VFS file handle: {0}")]
    InvalidHandle(u32),

    #[error("Memory error during VFS transfer: {0}")]
    Memory(#[from] MemoryError),
}

/// Horizon OS Virtual Filesystem (RomFS, SDMC, and SaveData).
pub struct VirtualFileSystem {
    /// Maps normalized virtual paths (e.g. "romfs:/title.txt") to file byte contents
    files: HashMap<String, Vec<u8>>,
    /// Maps open file handles to virtual paths
    open_handles: HashMap<u32, String>,
    next_handle: u32,
    pub total_bytes_read: u64,
    pub total_bytes_written: u64,
}

impl Default for VirtualFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualFileSystem {
    pub fn new() -> Self {
        let mut vfs = Self {
            files: HashMap::new(),
            open_handles: HashMap::new(),
            next_handle: 1,
            total_bytes_read: 0,
            total_bytes_written: 0,
        };

        // Populate built-in Switch 2 RomFS metadata assets
        vfs.mount_file(
            "romfs:/title.txt",
            b"Ounce-Core Switch 2 Tech Demo (Tegra T239 Ampere)",
        );
        vfs.mount_file(
            "romfs:/shader/default_vs.glsl",
            b"#version 460\nuniform mat4 u_mvp;\nvoid main() { gl_Position = u_mvp * vec4(pos, 1.0); }",
        );

        vfs
    }

    /// Mounts or overwrites a virtual file in the VFS.
    pub fn mount_file(&mut self, path: &str, data: &[u8]) {
        self.files.insert(path.to_string(), data.to_vec());
    }

    /// Opens a file by virtual path and returns a VFS file handle.
    pub fn open_file(&mut self, path: &str) -> Result<u32, VfsError> {
        if !self.files.contains_key(path) {
            return Err(VfsError::NotFound(path.to_string()));
        }
        let handle = self.next_handle;
        self.next_handle += 1;
        self.open_handles.insert(handle, path.to_string());
        Ok(handle)
    }

    /// Returns the byte length of an open file handle.
    pub fn get_file_size(&self, handle: u32) -> Result<usize, VfsError> {
        let path = self
            .open_handles
            .get(&handle)
            .ok_or(VfsError::InvalidHandle(handle))?;
        let data = self
            .files
            .get(path)
            .ok_or_else(|| VfsError::NotFound(path.clone()))?;
        Ok(data.len())
    }

    /// Reads bytes from a VFS file directly into guest virtual memory.
    pub fn read_file_to_guest(
        &mut self,
        path: &str,
        file_offset: usize,
        guest_dst_addr: u64,
        max_len: usize,
        mmu: &mut Memory,
    ) -> Result<usize, VfsError> {
        let data = self
            .files
            .get(path)
            .ok_or_else(|| VfsError::NotFound(path.to_string()))?;

        if file_offset >= data.len() {
            return Ok(0);
        }

        let available = (data.len() - file_offset).min(max_len);
        let slice = &data[file_offset..file_offset + available];
        mmu.write_bytes(guest_dst_addr, slice)?;
        self.total_bytes_read += available as u64;

        Ok(available)
    }

    /// Writes bytes from guest virtual memory into a writable VFS file (e.g., `save:/profile.sav`).
    pub fn write_file_from_guest(
        &mut self,
        path: &str,
        guest_src_addr: u64,
        len: usize,
        mmu: &Memory,
    ) -> Result<usize, VfsError> {
        let mut buf = vec![0u8; len];
        mmu.read_bytes(guest_src_addr, &mut buf)?;
        self.files.insert(path.to_string(), buf);
        self.total_bytes_written += len as u64;
        Ok(len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mmu::Permission;

    #[test]
    fn test_vfs_mount_and_read_to_guest() {
        let mut vfs = VirtualFileSystem::new();
        let mut mmu = Memory::new();
        let dst_addr = 0x80050000u64;
        mmu.map_range(dst_addr, 4096, Permission::READ_WRITE);

        vfs.mount_file("romfs:/hello.bin", b"Switch2RomFS");
        let n = vfs
            .read_file_to_guest("romfs:/hello.bin", 0, dst_addr, 64, &mut mmu)
            .unwrap();
        assert_eq!(n, 12);

        let mut buf = [0u8; 12];
        mmu.read_bytes(dst_addr, &mut buf).unwrap();
        assert_eq!(&buf, b"Switch2RomFS");
    }
}
