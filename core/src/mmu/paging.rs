use std::collections::HashMap;

#[derive(Debug)]
/// Page Table implementation with sparse entries
pub struct PageTable {
    entries: HashMap<usize, (u64, u64)>, // (base_address, permissions)
}

impl PageTable {
    /// Create new empty page table
    pub fn new() -> Self {
        PageTable {
            entries: HashMap::new(),
        }
    }

    /// Perform page table walk with permissions
    pub fn walk(&self, vaddr: u64) -> Option<(u64, bool, bool)> {
        let vpn = (vaddr >> 12) as usize;
        let (base, permissions) = self.entries.get(&vpn)?;
        
        // Physical address = page base + offset
        Some((
            base | (vaddr & 0xFFF),
            permissions & (1 << 62) != 0,
            permissions & (1 << 63) != 0
        ))
    }

    /// Set a page table entry with permissions
    pub fn set_entry(&mut self, index: usize, value: u64, readable: bool, writable: bool) {
        let mut perm_flags = 0u64;
        if readable { perm_flags |= 1 << 62; }
        if writable { perm_flags |= 1 << 63; }
        
        self.entries.insert(index, (value, perm_flags));
    }

    /// Get pointer to page table data
    pub fn as_ptr(&self) -> *const std::ffi::c_void {
        self as *const _ as *const std::ffi::c_void
    }
}