//! Test suite for Dynarmic JIT backend

use std::time::{Instant, Duration};
use crate::cpu::DynarmicCPU;

const TEST_BASE_ADDR: u64 = 0x0000_1000;
const TEST_TIMEOUT: Duration = Duration::from_millis(100);

/// Test result
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub message: String,
    pub duration: Duration,
}

impl TestResult {
    fn pass(name: &str, duration: Duration) -> Self {
        TestResult {
            name: name.to_string(),
            passed: true,
            message: "PASS".to_string(),
            duration,
        }
    }
    
    fn fail(name: &str, message: &str, duration: Duration) -> Self {
        TestResult {
            name: name.to_string(),
            passed: false,
            message: format!("FAIL: {}", message),
            duration,
        }
    }
    
    fn timeout(name: &str, duration: Duration) -> Self {
        TestResult {
            name: name.to_string(),
            passed: false,
            message: format!("TIMEOUT after {:?}", duration),
            duration,
        }
    }
}

/// ARM64 instruction encoding helpers
mod arm64 {
    /// Encode ADD immediate instruction
    pub fn add_imm(rd: u8, rn: u8, imm12: u16) -> u32 {
        0x91000000 | ((imm12 as u32) << 10) | ((rn as u32) << 5) | (rd as u32)
    }
    
    /// Encode SUB immediate instruction
    pub fn sub_imm(rd: u8, rn: u8, imm12: u16) -> u32 {
        0xD1000000 | ((imm12 as u32) << 10) | ((rn as u32) << 5) | (rd as u32)
    }
    
    /// Encode ADD register instruction
    pub fn add_reg(rd: u8, rn: u8, rm: u8) -> u32 {
        0x8B000000 | ((rm as u32) << 16) | ((rn as u32) << 5) | (rd as u32)
    }
    
    /// Encode MOV register instruction
    pub fn mov_reg(rd: u8, rm: u8) -> u32 {
        0xAA0003E0 | ((rm as u32) << 16) | (rd as u32)
    }
    
    /// Encode branch instruction
    pub fn branch(offset: i32) -> u32 {
        let imm26 = (offset >> 2) & 0x3FFFFFF;
        0x14000000 | (imm26 as u32)
    }
    
    /// Encode RET instruction
    pub fn ret() -> u32 {
        0xD65F03C0
    }
    
    /// Encode NOP instruction
    pub fn nop() -> u32 {
        0xD503201F
    }
}

/// Run single test case
fn run_single_test<F, V>(name: &str, setup: F, verify: V) -> TestResult
where
    F: FnOnce(&DynarmicCPU),
    V: FnOnce(&DynarmicCPU) -> bool,
{
    let start = Instant::now();
    
    match DynarmicCPU::new() {
        Some(cpu) => {
            // Set initial state
            cpu.set_sp(0x8000);
            cpu.set_pc(TEST_BASE_ADDR);
            
            // Run test setup
            setup(&cpu);
            
            // NEW: Execute single instruction using step()
            let result = cpu.step();
            let duration = start.elapsed();
            
            // Handle timeout
            if duration > TEST_TIMEOUT {
                TestResult::timeout(name, duration)
            } else if result == 0 {
                TestResult::fail(name, "Execution failed", duration)
            } else if verify(&cpu) {
                TestResult::pass(name, duration)
            } else {
                TestResult::fail(name, "Verification failed", duration)
            }
        }
        None => TestResult::fail(name, "Failed to create CPU", start.elapsed()),
    }
}

/// Run all instruction tests
pub fn run_tests() -> Vec<String> {
    let mut results = Vec::new();
    let start_time = Instant::now();
    
    println!("🧪 Starting Dynarmic JIT Instruction Tests...");
    println!("  Base address: 0x{:016X}", TEST_BASE_ADDR);
    println!();
    
    // Test cases
    let test_results = vec![
        // NOP instruction test
        run_single_test(
            "NOP",
            |cpu| cpu.write_u32(TEST_BASE_ADDR, arm64::nop()),
            |cpu| cpu.get_pc() == TEST_BASE_ADDR + 4
        ),
        
        // ADD immediate test
        run_single_test(
            "ADD X1, X1, #2",
            |cpu| {
                cpu.set_x(1, 5);
                cpu.write_u32(TEST_BASE_ADDR, arm64::add_imm(1, 1, 2));
            },
            |cpu| cpu.get_x(1) == 7 && cpu.get_pc() == TEST_BASE_ADDR + 4
        ),
        
        // SUB immediate test
        run_single_test(
            "SUB X2, X2, #1",
            |cpu| {
                cpu.set_x(2, 10);
                cpu.write_u32(TEST_BASE_ADDR, arm64::sub_imm(2, 2, 1));
            },
            |cpu| cpu.get_x(2) == 9 && cpu.get_pc() == TEST_BASE_ADDR + 4
        ),
        
        // ADD register test
        run_single_test(
            "ADD X0, X0, X1",
            |cpu| {
                cpu.set_x(0, 7);
                cpu.set_x(1, 3);
                cpu.write_u32(TEST_BASE_ADDR, arm64::add_reg(0, 0, 1));
            },
            |cpu| cpu.get_x(0) == 10 && cpu.get_pc() == TEST_BASE_ADDR + 4
        ),
        
        // MOV register test
        run_single_test(
            "MOV X3, X4",
            |cpu| {
                cpu.set_x(3, 0);
                cpu.set_x(4, 0xDEADBEEF);
                cpu.write_u32(TEST_BASE_ADDR, arm64::mov_reg(3, 4));
            },
            |cpu| cpu.get_x(3) == 0xDEADBEEF && cpu.get_pc() == TEST_BASE_ADDR + 4
        ),
        
        // Branch test
        run_single_test(
            "B +8",
            |cpu| cpu.write_u32(TEST_BASE_ADDR, arm64::branch(8)),
            |cpu| cpu.get_pc() == TEST_BASE_ADDR + 8
        ),
        
        // RET instruction test
        run_single_test(
            "RET",
            |cpu| {
                cpu.set_x(30, 0x2000); // Link register
                cpu.write_u32(TEST_BASE_ADDR, arm64::ret());
            },
            |cpu| cpu.get_pc() == 0x2000
        ),
    ];
    
    // Process results
    let mut passed = 0;
    let mut failed = 0;
    
    for result in &test_results {
        let icon = if result.passed { "✅" } else { "❌" };
        let time_str = format!("{:?}", result.duration);
        
        println!("  {} {} - {} ({})", 
            icon, 
            result.name, 
            result.message,
            time_str
        );
        
        if result.passed {
            passed += 1;
        } else {
            failed += 1;
        }
        
        results.push(format!("{}: {} ({})", 
            result.name, 
            result.message,
            time_str
        ));
    }
    
    let total_time = start_time.elapsed();
    
    // Summary
    println!();
    println!("📊 Test Summary:");
    println!("  Total tests: {}", test_results.len());
    println!("  Passed: {} ✅", passed);
    println!("  Failed: {} ❌", failed);
    println!("  Total time: {:?}", total_time);
    println!();
    
    results.push(format!("Total: {} passed, {} failed", passed, failed));
    results
}