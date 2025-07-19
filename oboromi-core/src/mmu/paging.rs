use std::collections::HashMap;

#[derive(Debug)]
/// Page Table implementation with sparse entries
pub struct PageTable {
    entries: HashMap<usize, u64>,
}

impl PageTable {
    /// Create new empty page table
    pub fn new() -> Self {
        PageTable {
            entries: HashMap::new(),
        }
    }

    /// Perform page table walk
    pub fn walk(&self, vaddr: u64) -> Option<u64> {
        let vpn = (vaddr >> 12) as usize; // Virtual Page Number (4KB pages)
        let base = self.entries.get(&vpn)?;
        
        // Physical address = page base + offset
        Some(base | (vaddr & 0xFFF))
    }

    /// Set a page table entry at index
    pub fn set_entry(&mut self, index: usize, value: u64) {
        self.entries.insert(index, value & !0xFFF);
    }
}
