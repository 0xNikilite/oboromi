#[derive(Debug, Clone, Copy)]
/// TLB Entry structure
pub struct TlbEntry {
    pub vaddr: u64,     // Virtual address (page aligned)
    pub paddr: u64,     // Physical address (page aligned)
    pub valid: bool,    // Entry is valid
}

/// Translation Lookaside Buffer
#[derive(Debug)]
pub struct Tlb {
    entries: Vec<TlbEntry>,
    capacity: usize,
}

impl Tlb {
    /// Create new TLB with specified capacity
    pub fn new(capacity: usize) -> Self {
        Tlb {
            entries: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Lookup virtual address in TLB
    pub fn lookup(&self, vaddr: u64) -> Option<u64> {
        let aligned_vaddr = vaddr & !0xFFF; // Page align
        
        for entry in &self.entries {
            if entry.valid && entry.vaddr == aligned_vaddr {
                return Some(entry.paddr | (vaddr & 0xFFF)); // Add offset
            }
        }
        None
    }

    /// Add new entry to TLB (FIFO replacement)
    pub fn add_entry(&mut self, vaddr: u64, paddr: u64) {
        let aligned_vaddr = vaddr & !0xFFF; // Page align
        let aligned_paddr = paddr & !0xFFF; // Page align

        // Remove oldest entry if full
        if self.entries.len() >= self.capacity {
            self.entries.remove(0);
        }

        self.entries.push(TlbEntry {
            vaddr: aligned_vaddr,
            paddr: aligned_paddr,
            valid: true,
        });
    }
}