#[derive(Debug)]
/// Page Table implementation
pub struct PageTable {
    entries: Vec<Option<u64>>,
}

impl PageTable {
    /// Create new page table with identity mapping disabled
    pub fn new() -> Self {
        PageTable {
            entries: vec![None; 1 << 10],
        }
    }

    /// Perform page table walk
    pub fn walk(&self, vaddr: u64) -> Option<u64> {
        let vpn = (vaddr >> 12) & 0x3FF; // Virtual Page Number (4KB pages)
        let base = match self.entries.get(vpn as usize)? {
            Some(addr) => *addr,
            None => return None, // Page not mapped
        };
        
        // Physical address = page base + offset
        Some(base | (vaddr & 0xFFF))
    }

    /// Set a page table entry at index
    pub fn set_entry(&mut self, index: usize, value: u64) {
        if index < self.entries.len() {
            self.entries[index] = Some(value & !0xFFF);
        }
    }
    
    /// Get the number of entries in the page table
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}