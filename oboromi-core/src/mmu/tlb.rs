#[derive(Debug, Clone, Copy)]
/// TLB Entry structure
pub struct TlbEntry {
    pub vaddr: u64,     // Virtual address (page aligned)
    pub paddr: u64,     // Physical address (page aligned)
    pub valid: bool,    // Entry is valid
    pub readable: bool, // Page is readable
    pub writable: bool, // Page is writable
}

impl TlbEntry {
    /// Create new TLB entry with permissions
    pub fn new(vaddr: u64, paddr: u64, readable: bool, writable: bool) -> Self {
        TlbEntry {
            vaddr,
            paddr,
            valid: true,
            readable,
            writable,
        }
    }
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

    /// Lookup virtual address in TLB with permissions
    pub fn lookup(&self, vaddr: u64) -> Option<(u64, bool, bool)> {
        let aligned_vaddr = vaddr & !0xFFF;
        
        for entry in &self.entries {
            if entry.valid && entry.vaddr == aligned_vaddr {
                return Some((
                    entry.paddr | (vaddr & 0xFFF),
                    entry.readable,
                    entry.writable
                ));
            }
        }
        None
    }

    /// Add new entry to TLB with permissions (FIFO replacement)
    pub fn add_entry(&mut self, vaddr: u64, paddr: u64, readable: bool, writable: bool) {
        let aligned_vaddr = vaddr & !0xFFF;
        let aligned_paddr = paddr & !0xFFF;

        if self.entries.len() >= self.capacity {
            self.entries.remove(0);
        }

        self.entries.push(TlbEntry::new(
            aligned_vaddr,
            aligned_paddr,
            readable,
            writable
        ));
    }

    /// Invalidate TLB entry for a virtual address
    pub fn invalidate(&mut self, vaddr: u64) {
        let aligned_vaddr = vaddr & !0xFFF;
        self.entries.retain(|entry| entry.vaddr != aligned_vaddr);
    }
}