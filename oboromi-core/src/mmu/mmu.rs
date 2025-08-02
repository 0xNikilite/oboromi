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

    /// Translate virtual address to physical address with permissions
    pub fn translate(&mut self, vaddr: u64) -> Option<(u64, bool, bool)> {
        // 1. Check TLB first
        if let Some(result) = self.tlb.lookup(vaddr) {
            return Some(result);
        }
        
        // 2. Page table walk
        let (paddr, readable, writable) = self.page_table.walk(vaddr)?;
        
        // 3. Update TLB
        self.tlb.add_entry(vaddr, paddr, readable, writable);
        
        Some((paddr, readable, writable))
    }
    
    /// Set identity mapping for a memory range with permissions
    pub fn set_identity_mapping(&mut self, start_page: usize, page_count: usize) {
        for i in 0..page_count {
            let page_index = start_page + i;
            self.page_table.set_entry(
                page_index,
                (page_index * 4096) as u64,
                true,  // Readable
                true   // Writable
            );
        }
    }
    
    /// Map a single page from virtual to physical address with permissions
    pub fn map_page(&mut self, vaddr: u64, paddr: u64, readable: bool, writable: bool) {
        let vpn = (vaddr >> 12) as usize;
        let aligned_paddr = paddr & !0xFFF;
        self.page_table.set_entry(vpn, aligned_paddr, readable, writable);
        self.tlb.invalidate(vaddr);
    }
}