use super::{paging::PageTable, tlb::Tlb};

#[derive(Debug)]
/// Memory Management Unit (core implementation)
pub struct MMU {
    pub tlb: Tlb,
    pub page_table: PageTable,
}

impl MMU {
    /// Create a new MMU with empty TLB and page table
    pub fn new() -> Self {
        MMU {
            tlb: Tlb::new(64),
            page_table: PageTable::new(),
        }
    }

    /// Translate virtual address to physical address
    pub fn translate(&mut self, vaddr: u64) -> Option<u64> {
        // 1. Check TLB first
        if let Some(paddr) = self.tlb.lookup(vaddr) {
            return Some(paddr);
        }
        
        // 2. Page table walk
        let paddr = self.page_table.walk(vaddr)?;
        
        // 3. Update TLB
        self.tlb.add_entry(vaddr, paddr);
        
        Some(paddr)
    }
    
    /// Set identity mapping for a memory range
    pub fn set_identity_mapping(&mut self, start_page: usize, end_page: usize) {
        for i in start_page..end_page {
            if i < self.page_table.len() {
                // Mappa la pagina virtuale i alla stessa pagina fisica
                self.page_table.set_entry(i, (i * 4096) as u64);
            }
        }
    }
}