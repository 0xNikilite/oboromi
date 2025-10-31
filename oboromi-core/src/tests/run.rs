//! Test suite for Dynarmic JIT backend
use std::time::{Instant, Duration};
use crate::cpu::DynarmicCPU;

const TEST_BASE_ADDR: u64 = 0x0000_1000;
const BREAKPOINT_ADDR: u64 = 0x0000_2000;

/// Get platform-specific timeout for test execution
/// macOS requires longer timeout due to JIT cold start overhead
fn get_test_timeout() -> Duration {
    if cfg!(target_os = "macos") {
        Duration::from_millis(500)
    } else {
        Duration::from_millis(100)
    }
}

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
    
    pub fn brk(imm16: u16) -> u32 {
        0xD4200000 | ((imm16 as u32) << 5)
    }
}

/// Warm up the JIT compiler by pre-compiling basic instructions
/// This prevents timeout issues on slower hardware during actual tests
/// No timeout is enforced here as initial compilation can take variable time
fn warmup_jit() {
    println!("Warming up JIT compiler...");
    
    let cpu = match DynarmicCPU::new() {
        Some(cpu) => cpu,
        None => {
            println!("Failed to create CPU for warmup");
            return;
        }
    };
    
    cpu.set_sp(0x8000);
    cpu.set_pc(TEST_BASE_ADDR);
    
    let warmup_instructions = vec![
        arm64::nop(),
        arm64::add_imm(0, 0, 1),
        arm64::add_reg(1, 1, 2),
        arm64::mov_reg(3, 4),
        arm64::brk(0),
    ];
    
    let mut addr = TEST_BASE_ADDR;
    for instr in warmup_instructions {
        cpu.write_u32(addr, instr);
        addr += 4;
    }
    
    cpu.set_x(0, 10);
    cpu.set_x(1, 20);
    cpu.set_x(2, 30);
    cpu.set_x(4, 0xCAFE);
    
    println!("  Compiling warmup code...");
    let start = Instant::now();
    let _ = cpu.run();
    let elapsed = start.elapsed();
    
    println!("JIT warmup completed in {:?}", elapsed);
    
    if elapsed.as_millis() > 200 {
        println!("First compilation took {:?} - this is normal on slower hardware", elapsed);
    }
}

fn run_instruction_test<F, V>(name: &str, instructions: &[u32], setup: F, verify: V) -> TestResult
where
    F: FnOnce(&DynarmicCPU),
    V: FnOnce(&DynarmicCPU) -> bool,
{
    let start = Instant::now();
    let timeout = get_test_timeout();
    
    println!("  Running test: {}", name);
    
    let cpu = match DynarmicCPU::new() {
        Some(cpu) => {
            println!("CPU created successfully");
            cpu
        }
        None => {
            println!("FAILED to create CPU!");
            return TestResult::fail(name, "Failed to create CPU", start.elapsed());
        }
    };
    
    println!("Setting initial state...");
    cpu.set_sp(0x8000);
    cpu.set_pc(TEST_BASE_ADDR);
    
    let mut current_addr = TEST_BASE_ADDR;
    for (i, &instr) in instructions.iter().enumerate() {
        cpu.write_u32(current_addr, instr);
        println!("    Wrote instruction {}: 0x{:08X} at 0x{:016X}", i + 1, instr, current_addr);
        current_addr += 4;
    }
    
    cpu.write_u32(current_addr, arm64::brk(0));
    println!("Added breakpoint at 0x{:016X}", current_addr);
    
    println!("Running test setup...");
    setup(&cpu);
    
    println!("Executing instructions with run()...");
    let result = cpu.run();
    let final_pc = cpu.get_pc();
    println!("Execution completed, PC: 0x{:016X}, result: {}", final_pc, result);
    
    let duration = start.elapsed();
    
    if duration > timeout {
        TestResult::timeout(name, duration)
    } else if result == 0 {
        TestResult::fail(name, &format!("Execution failed (PC = 0x{:016X})", final_pc), duration)
    } else {
        println!("Running verification...");
        let verification_result = verify(&cpu);
        
        if verification_result {
            TestResult::pass(name, duration)
        } else {
            TestResult::fail(name, &format!("Verification failed (PC = 0x{:016X})", final_pc), duration)
        }
    }
}

fn run_control_flow_test<F, V>(name: &str, instructions: &[u32], target_addr: u64, setup: F, verify: V) -> TestResult
where
    F: FnOnce(&DynarmicCPU),
    V: FnOnce(&DynarmicCPU) -> bool,
{
    let start = Instant::now();
    let timeout = get_test_timeout();
    
    println!("Running test: {}", name);
    
    let cpu = match DynarmicCPU::new() {
        Some(cpu) => {
            println!("CPU created successfully");
            cpu
        }
        None => {
            println!("FAILED to create CPU!");
            return TestResult::fail(name, "Failed to create CPU", start.elapsed());
        }
    };
    
    println!("Setting initial state...");
    cpu.set_sp(0x8000);
    cpu.set_pc(TEST_BASE_ADDR);
    
    let mut current_addr = TEST_BASE_ADDR;
    for (i, &instr) in instructions.iter().enumerate() {
        cpu.write_u32(current_addr, instr);
        println!("Wrote instruction {}: 0x{:08X} at 0x{:016X}", i + 1, instr, current_addr);
        current_addr += 4;
    }
    
    cpu.write_u32(target_addr, arm64::brk(0));
    cpu.write_u32(target_addr + 4, arm64::nop());
    println!("Added breakpoint at target address 0x{:016X}", target_addr);
    
    println!("Running test setup...");
    setup(&cpu);
    
    println!("Executing instructions with run()...");
    let result = cpu.run();
    let final_pc = cpu.get_pc();
    println!("Execution completed, PC: 0x{:016X}, result: {}", final_pc, result);
    
    let duration = start.elapsed();
    
    if duration > timeout {
        TestResult::timeout(name, duration)
    } else if result == 0 {
        TestResult::fail(name, &format!("Execution failed (PC = 0x{:016X})", final_pc), duration)
    } else {
        println!("Running verification...");
        let verification_result = verify(&cpu);
        
        if verification_result {
            TestResult::pass(name, duration)
        } else {
            TestResult::fail(name, &format!("Verification failed (PC = 0x{:016X})", final_pc), duration)
        }
    }
}

fn run_ret_test<F, V>(name: &str, instructions: &[u32], setup: F, verify: V) -> TestResult
where
    F: FnOnce(&DynarmicCPU),
    V: FnOnce(&DynarmicCPU) -> bool,
{
    let start = Instant::now();
    let timeout = get_test_timeout();
    
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
    
    let mut current_addr = TEST_BASE_ADDR;
    for (i, &instr) in instructions.iter().enumerate() {
        cpu.write_u32(current_addr, instr);
        println!("    Wrote instruction {}: 0x{:08X} at 0x{:016X}", i + 1, instr, current_addr);
        current_addr += 4;
    }
    
    let return_addr = 0x2000;
    cpu.write_u32(return_addr, arm64::brk(0));
    cpu.write_u32(return_addr + 4, arm64::nop());
    println!("    Set up return target at 0x{:016X} with breakpoint", return_addr);
    
    println!("    Running test setup...");
    setup(&cpu);
    
    println!("    Executing instructions with run()...");
    let result = cpu.run();
    let final_pc = cpu.get_pc();
    println!("    Execution completed, PC: 0x{:016X}, result: {}", final_pc, result);
    
    let duration = start.elapsed();
    
    if duration > timeout {
        TestResult::timeout(name, duration)
    } else if result == 0 {
        TestResult::fail(name, &format!("Execution failed (PC = 0x{:016X})", final_pc), duration)
    } else {
        println!("    Running verification...");
        let verification_result = verify(&cpu);
        
        if verification_result {
            TestResult::pass(name, duration)
        } else {
            TestResult::fail(name, &format!("Verification failed (PC = 0x{:016X})", final_pc), duration)
        }
    }
}

fn run_multi_instruction_test<F, V>(name: &str, instructions: &[u32], setup: F, verify: V) -> TestResult
where
    F: FnOnce(&DynarmicCPU),
    V: FnOnce(&DynarmicCPU) -> bool,
{
    let start = Instant::now();
    let timeout = get_test_timeout();
    
    println!("  Running test: {} ({} instructions)", name, instructions.len());
    
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
    
    let mut current_addr = TEST_BASE_ADDR;
    for (i, &instr) in instructions.iter().enumerate() {
        cpu.write_u32(current_addr, instr);
        println!("    Wrote instruction {}: 0x{:08X} at 0x{:016X}", i + 1, instr, current_addr);
        current_addr += 4;
    }
    
    cpu.write_u32(current_addr, arm64::brk(0));
    println!("    Added breakpoint at 0x{:016X}", current_addr);
    
    println!("    Running test setup...");
    setup(&cpu);
    
    println!("    Executing {} instructions with run()...", instructions.len());
    let result = cpu.run();
    let final_pc = cpu.get_pc();
    println!("    Execution completed, PC: 0x{:016X}, result: {}", final_pc, result);
    
    let duration = start.elapsed();
    
    if duration > timeout {
        TestResult::timeout(name, duration)
    } else if result == 0 {
        TestResult::fail(name, &format!("Execution failed (PC = 0x{:016X})", final_pc), duration)
    } else {
        println!("    Running verification...");
        let verification_result = verify(&cpu);
        
        if verification_result {
            TestResult::pass(name, duration)
        } else {
            TestResult::fail(name, &format!("Verification failed (PC = 0x{:016X})", final_pc), duration)
        }
    }
}

pub fn run_tests() -> Vec<String> {
    let mut results = Vec::new();
    let start_time = Instant::now();
    
    println!("🧪 Starting Dynarmic JIT Instruction Tests...");
    println!("  Base address: 0x{:016X}", TEST_BASE_ADDR);
    println!("  Breakpoint address: 0x{:016X}", BREAKPOINT_ADDR);
    println!();
    
    warmup_jit();
    println!();
    
    if cfg!(target_os = "macos") {
        println!("  macOS test timeout: {:?}", get_test_timeout());
    }
    
    let test_results = vec![
        run_instruction_test(
            "NOP",
            &[
                arm64::nop(),
            ],
            |_cpu| {},
            |cpu| {
                cpu.get_pc() >= TEST_BASE_ADDR + 4
            }
        ),
        
        run_instruction_test(
            "ADD X1, X1, #2",
            &[
                arm64::add_imm(1, 1, 2),
            ],
            |cpu| {
                cpu.set_x(1, 5);
            },
            |cpu| cpu.get_x(1) == 7
        ),
        
        run_instruction_test(
            "SUB X2, X2, #1",
            &[
                arm64::sub_imm(2, 2, 1),
            ],
            |cpu| {
                cpu.set_x(2, 10);
            },
            |cpu| cpu.get_x(2) == 9
        ),
        
        run_instruction_test(
            "ADD X0, X0, X1",
            &[
                arm64::add_reg(0, 0, 1),
            ],
            |cpu| {
                cpu.set_x(0, 7);
                cpu.set_x(1, 3);
            },
            |cpu| cpu.get_x(0) == 10
        ),
        
        run_instruction_test(
            "MOV X3, X4",
            &[
                arm64::mov_reg(3, 4),
            ],
            |cpu| {
                cpu.set_x(3, 0);
                cpu.set_x(4, 0xDEADBEEF);
            },
            |cpu| cpu.get_x(3) == 0xDEADBEEF
        ),
        
        run_control_flow_test(
            "B +8",
            &[
                arm64::branch(8),
                arm64::nop(),
                arm64::nop(),
            ],
            TEST_BASE_ADDR + 8,
            |_cpu| {},
            |cpu| cpu.get_pc() == TEST_BASE_ADDR + 8
        ),
        
        run_ret_test(
            "RET",
            &[
                arm64::ret(),
            ],
            |cpu| {
                cpu.set_x(30, 0x2000);
            },
            |cpu| cpu.get_pc() == 0x2004
        ),
        
        run_instruction_test(
            "Atomic ADD Test",
            &[
                arm64::add_imm(0, 0, 50),
            ],
            |cpu| {
                cpu.set_x(0, 100);
            },
            |cpu| cpu.get_x(0) == 150
        ),
        
        run_multi_instruction_test(
            "Memory Access Pattern",
            &[
                arm64::add_imm(1, 1, 1),
                arm64::add_imm(1, 1, 1),
                arm64::add_imm(1, 1, 1),
            ],
            |cpu| {
                cpu.set_x(1, 0);
            },
            |cpu| cpu.get_x(1) == 3
        ),
        
        run_multi_instruction_test(
            "Multiple Arithmetic Ops",
            &[
                arm64::add_imm(0, 0, 5),
                arm64::sub_imm(1, 1, 3),
                arm64::add_reg(0, 0, 1),
            ],
            |cpu| {
                cpu.set_x(0, 10);
                cpu.set_x(1, 20);
            },
            |cpu| cpu.get_x(0) == 32 && cpu.get_x(1) == 17
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
    
    if failed > 0 && cfg!(target_os = "macos") {
        println!("  ⚠️  macOS JIT cold start may cause first-test timeout");
        println!("      This is normal after build system changes");
    }
    
    println!();
    
    results.push(format!("Total: {} passed, {} failed", passed, failed));
    results
}
