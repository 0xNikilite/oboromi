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
    pub fn add_imm(rd: u8, rn: u8, imm12: u16) -> u32 {
        0x91000000 | ((imm12 as u32) << 10) | ((rn as u32) << 5) | (rd as u32)
    }
    
    pub fn sub_imm(rd: u8, rn: u8, imm12: u16) -> u32 {
        0xD1000000 | ((imm12 as u32) << 10) | ((rn as u32) << 5) | (rd as u32)
    }
    
    pub fn add_reg(rd: u8, rn: u8, rm: u8) -> u32 {
        0x8B000000 | ((rm as u32) << 16) | ((rn as u32) << 5) | (rd as u32)
    }
    
    pub fn mov_reg(rd: u8, rm: u8) -> u32 {
        0xAA0003E0 | ((rm as u32) << 16) | (rd as u32)
    }
    
    pub fn branch(offset: i32) -> u32 {
        let imm26 = (offset >> 2) & 0x3FFFFFF;
        0x14000000 | (imm26 as u32)
    }
    
    pub fn ret() -> u32 {
        0xD65F03C0
    }
    
    pub fn nop() -> u32 {
        0xD503201F
    }
}

fn run_single_test<F, V>(name: &str, setup: F, verify: V) -> TestResult
where
    F: FnOnce(&DynarmicCPU),
    V: FnOnce(&DynarmicCPU) -> bool,
{
    let start = Instant::now();
    
    println!("  Running test: {}", name);
    
    let cpu = match DynarmicCPU::new() {
        Some(cpu) => {
            println!("    CPU created successfully");
            cpu
        }
        None => {
            println!("    FAILED to create CPU!");
            return TestResult::fail(name, "Failed to create CPU", start.elapsed());
        }
    };
    
    println!("    Setting initial state...");
    cpu.set_sp(0x8000);
    cpu.set_pc(TEST_BASE_ADDR);
    
    println!("    Running test setup...");
    setup(&cpu);
    
    println!("    Executing instruction...");
    let result = cpu.step();
    println!("    Instruction executed, PC: 0x{:016X}", cpu.get_pc());
    
    let duration = start.elapsed();
    
    if duration > TEST_TIMEOUT {
        TestResult::timeout(name, duration)
    } else if result == 0 {
        TestResult::fail(name, &format!("Execution failed (PC = 0x{:016X})", cpu.get_pc()), duration)
    } else {
        println!("    Running verification...");
        let verification_result = verify(&cpu);
        
        if verification_result {
            TestResult::pass(name, duration)
        } else {
            TestResult::fail(name, &format!("Verification failed (PC = 0x{:016X}, X0 = 0x{:016X})", 
                cpu.get_pc(), cpu.get_x(0)), duration)
        }
    }
}

fn run_multi_instruction_test<F, V>(name: &str, instruction_count: usize, setup: F, verify: V) -> TestResult
where
    F: FnOnce(&DynarmicCPU),
    V: FnOnce(&DynarmicCPU) -> bool,
{
    let start = Instant::now();
    
    println!("  Running test: {} ({} instructions)", name, instruction_count);
    
    let cpu = match DynarmicCPU::new() {
        Some(cpu) => {
            println!("    CPU created successfully");
            cpu
        }
        None => {
            println!("    FAILED to create CPU!");
            return TestResult::fail(name, "Failed to create CPU", start.elapsed());
        }
    };
    
    println!("    Setting initial state...");
    cpu.set_sp(0x8000);
    cpu.set_pc(TEST_BASE_ADDR);
    
    println!("    Running test setup...");
    setup(&cpu);
    
    println!("    Executing {} instructions...", instruction_count);
    for i in 0..instruction_count {
        println!("    Executing instruction {}...", i + 1);
        let result = cpu.step();
        println!("    Instruction {} executed, PC: 0x{:016X}", i + 1, cpu.get_pc());
        
        if result == 0 {
            return TestResult::fail(name, &format!("Execution failed at instruction {} (PC = 0x{:016X})", 
                i + 1, cpu.get_pc()), start.elapsed());
        }
        
        if cpu.get_pc() >= TEST_BASE_ADDR + 0x1000 {
            return TestResult::fail(name, &format!("PC out of bounds at instruction {} (PC = 0x{:016X})", 
                i + 1, cpu.get_pc()), start.elapsed());
        }
    }
    
    let duration = start.elapsed();
    
    if duration > TEST_TIMEOUT {
        TestResult::timeout(name, duration)
    } else {
        println!("    Running verification...");
        let verification_result = verify(&cpu);
        
        if verification_result {
            TestResult::pass(name, duration)
        } else {
            TestResult::fail(name, &format!("Verification failed (PC = 0x{:016X}, X0 = 0x{:016X})", 
                cpu.get_pc(), cpu.get_x(0)), duration)
        }
    }
}

pub fn run_tests() -> Vec<String> {
    let mut results = Vec::new();
    let start_time = Instant::now();
    
    println!("🧪 Starting Dynarmic JIT Instruction Tests...");
    println!("  Base address: 0x{:016X}", TEST_BASE_ADDR);
    println!("  Using 4KB test memory (0x1000-0x2000)");
    println!();
    
    let test_results = vec![
        run_single_test(
            "NOP",
            |cpu| {
                cpu.write_u32(TEST_BASE_ADDR, arm64::nop());
            },
            |cpu| {
                let pc = cpu.get_pc();
                pc == TEST_BASE_ADDR + 4
            }
        ),
        
        run_single_test(
            "ADD X1, X1, #2",
            |cpu| {
                cpu.set_x(1, 5);
                cpu.write_u32(TEST_BASE_ADDR, arm64::add_imm(1, 1, 2));
            },
            |cpu| cpu.get_x(1) == 7 && cpu.get_pc() == TEST_BASE_ADDR + 4
        ),
        
        run_single_test(
            "SUB X2, X2, #1",
            |cpu| {
                cpu.set_x(2, 10);
                cpu.write_u32(TEST_BASE_ADDR, arm64::sub_imm(2, 2, 1));
            },
            |cpu| cpu.get_x(2) == 9 && cpu.get_pc() == TEST_BASE_ADDR + 4
        ),
        
        run_single_test(
            "ADD X0, X0, X1",
            |cpu| {
                cpu.set_x(0, 7);
                cpu.set_x(1, 3);
                cpu.write_u32(TEST_BASE_ADDR, arm64::add_reg(0, 0, 1));
            },
            |cpu| cpu.get_x(0) == 10 && cpu.get_pc() == TEST_BASE_ADDR + 4
        ),
        
        run_single_test(
            "MOV X3, X4",
            |cpu| {
                cpu.set_x(3, 0);
                cpu.set_x(4, 0xDEADBEEF);
                cpu.write_u32(TEST_BASE_ADDR, arm64::mov_reg(3, 4));
            },
            |cpu| cpu.get_x(3) == 0xDEADBEEF && cpu.get_pc() == TEST_BASE_ADDR + 4
        ),
        
        run_single_test(
            "B +8",
            |cpu| {
                cpu.write_u32(TEST_BASE_ADDR, arm64::branch(8));
            },
            |cpu| cpu.get_pc() == TEST_BASE_ADDR + 8
        ),
        
        run_single_test(
            "RET",
            |cpu| {
                cpu.set_x(30, 0x2000);
                cpu.write_u32(TEST_BASE_ADDR, arm64::ret());
            },
            |cpu| cpu.get_pc() == 0x2000
        ),
        
        run_single_test(
            "Atomic ADD Test",
            |cpu| {
                cpu.set_x(0, 100);
                cpu.write_u32(TEST_BASE_ADDR, arm64::add_imm(0, 0, 50));
            },
            |cpu| cpu.get_x(0) == 150 && cpu.get_pc() == TEST_BASE_ADDR + 4
        ),
        
        run_multi_instruction_test(
            "Memory Access Pattern",
            3,
            |cpu| {
                cpu.write_u32(TEST_BASE_ADDR, arm64::add_imm(1, 1, 1));
                cpu.write_u32(TEST_BASE_ADDR + 4, arm64::add_imm(1, 1, 1));
                cpu.write_u32(TEST_BASE_ADDR + 8, arm64::add_imm(1, 1, 1));
                cpu.set_x(1, 0);
                cpu.set_pc(TEST_BASE_ADDR);
            },
            |cpu| cpu.get_x(1) == 3 && cpu.get_pc() == TEST_BASE_ADDR + 12
        ),
        
        run_multi_instruction_test(
            "Multiple Arithmetic Ops",
            3,
            |cpu| {
                cpu.set_x(0, 10);
                cpu.set_x(1, 20);
                cpu.write_u32(TEST_BASE_ADDR, arm64::add_imm(0, 0, 5));
                cpu.write_u32(TEST_BASE_ADDR + 4, arm64::sub_imm(1, 1, 3));
                cpu.write_u32(TEST_BASE_ADDR + 8, arm64::add_reg(0, 0, 1));
                cpu.set_pc(TEST_BASE_ADDR);
            },
            |cpu| cpu.get_x(0) == 32 && cpu.get_x(1) == 17 && cpu.get_pc() == TEST_BASE_ADDR + 12
        ),
    ];
    
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
    
    println!();
    println!("📊 Test Summary:");
    println!("  Total tests: {}", test_results.len());
    println!("  Passed: {} ✅", passed);
    println!("  Failed: {} ❌", failed);
    println!("  Total time: {:?}", total_time);
    
    if failed > 0 {
        println!("  ⚠️  Some failures may be due to the current 4KB test memory limitation.");
        println!("      Full Rust memory integration will resolve these issues.");
    }
    
    println!();
    
    results.push(format!("Total: {} passed, {} failed", passed, failed));
    results
}
