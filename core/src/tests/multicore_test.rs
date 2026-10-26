#[cfg(test)]
mod tests {
    use crate::cpu::cpu_manager::CpuManager;

    // the real 12GB can't be committed on every machine
    const TEST_MEMORY_SIZE: u64 = 16 * 1024 * 1024;

    #[test]
    fn test_multicore_initialization() {
        println!("Initializing 8-core CPU Manager...");
        let manager = CpuManager::with_memory_size(TEST_MEMORY_SIZE);

        assert_eq!(manager.cores.len(), 8, "Should have 8 cores");
        assert_eq!(manager.shared_memory.len() as u64, TEST_MEMORY_SIZE, "Memory size should match the request");
    }

    #[test]
    fn test_shared_memory_access() {
        println!("Testing shared memory between cores...");
        let manager = CpuManager::with_memory_size(TEST_MEMORY_SIZE);

        let core0 = manager.get_core(0).expect("Core 0 missing");
        let core1 = manager.get_core(1).expect("Core 1 missing");

        // Write value using Core 0
        let test_addr = 0x1000;
        let test_val = 0xDEADBEEF;
        println!("Core 0 writing {:#x} to {:#x}", test_val, test_addr);
        core0.write_u32(test_addr, test_val);

        // Read value using Core 1
        let read_val = core1.read_u32(test_addr);
        println!("Core 1 read {:#x} from {:#x}", read_val, test_addr);

        assert_eq!(read_val, test_val, "Core 1 should see value written by Core 0");
    }
}
