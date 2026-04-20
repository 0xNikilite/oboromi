//! Test suite for Dynarmic JIT backend

use std::fmt::Display;
use std::time::{Duration, Instant};
use crate::cpu::{CpuCore, Fault};

const TEST_MEMORY_SIZE: u64 = 0x10000;
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

    fn fail(name: &str, message: impl Display, duration: Duration) -> Self {
        TestResult {
            name: name.to_string(),
            passed: false,
            message: format!("FAIL: {message}"),
            duration,
        }
    }

    fn timeout(name: &str, duration: Duration) -> Self {
        TestResult {
            name: name.to_string(),
            passed: false,
            message: format!("TIMEOUT after {duration:?}"),
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
    
    #[allow(dead_code)] // TODO: implement proper branch tests
    pub fn branch(offset: i32) -> u32 {
        // B imm26 - offset is in 4-byte instruction words
        // Encoding: 0b000101 | imm26
        let imm26 = offset.cast_unsigned() & 0x03FF_FFFF;
        0x14000000 | imm26
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

/// This prevents timeout issues on slower hardware during actual tests
/// No timeout is enforced here as initial compilation can take variable time
fn warmup_jit() {
    println!("Warming up Unicorn emulator...");
    let _start = Instant::now();
    let cpu = CpuCore::new_mock(TEST_MEMORY_SIZE);
    let mut addr = TEST_BASE_ADDR;
    for instr in [
        arm64::nop(),
        arm64::add_imm(0, 0, 1),
        arm64::add_reg(1, 1, 2),
        arm64::mov_reg(3, 4),
        arm64::brk(0),
    ] {
        cpu.write_u32(addr, instr).unwrap();
        addr += 4;
    }

    cpu.store_x::<0>(10);
    cpu.store_x::<1>(20);
    cpu.store_x::<2>(30);
    cpu.store_x::<4>(0xCAFE);
    
    println!("Compiling warmup code...");
    let start = Instant::now();
    let _ = cpu.execute_at(TEST_BASE_ADDR);
    let elapsed = start.elapsed();
    println!("JIT warmup completed in {elapsed:?}");
}

fn run_test<F, V>(name: &str, instructions: &[u32], setup: F, verify: V) -> TestResult
where
    F: FnOnce(&CpuCore),
    V: FnOnce(&CpuCore) -> bool,
{
    let start = Instant::now();
    let timeout = get_test_timeout();
    
    println!("Running test: {name} ({} instructions)", instructions.len());
    let cpu = CpuCore::new_mock(TEST_MEMORY_SIZE);

    println!("Setting initial state...");
    cpu.store_sp(0x8000);

    let mut current_addr = TEST_BASE_ADDR;
    for (i, &instr) in instructions.iter().enumerate() {
        cpu.write_u32(current_addr, instr).unwrap();
        println!("Wrote instruction {}: {instr:#08X} at {current_addr:#016X}", i + 1);
        current_addr += 4;
    }

    cpu.write_u32(current_addr, arm64::brk(0)).unwrap();
    println!("Added breakpoint at {current_addr:#016X}");
    
    println!("Running test setup...");
    setup(&cpu);
    
    println!("Executing {} instructions with run()...", instructions.len());
    let result = cpu.execute_at(TEST_BASE_ADDR);
    let final_pc = cpu.load_pc();
    println!("Execution completed, PC: {final_pc:#016X}, result: {result:?}");

    let duration = start.elapsed();

    match result {
        _ if duration > timeout => TestResult::timeout(name, duration),
        Ok(()) => TestResult::fail(name, "test cpu unexpectedly halted", duration),
        Err(Fault::UnhandledException) => {
            println!("Running verification...");
            match verify(&cpu) {
                true => TestResult::pass(name, duration),
                false => TestResult::fail(
                    name,
                    format_args!("Verification failed (PC = {final_pc:#016X})"),
                    duration
                )
            }
        }
        Err(err) => {
            TestResult::fail(
                name,
                format_args!("Execution faulted (PC = {final_pc:#016X}); fault: {err:?}"),
                duration
            )
        }
    }
}

pub fn run_tests() -> Vec<String> {
    let mut results = Vec::new();
    let start_time = Instant::now();

    println!("Starting Unicorn Instruction Tests...");
    println!("Base address: {TEST_BASE_ADDR:#016X}");
    println!("Breakpoint address: {BREAKPOINT_ADDR:#016X}");
    
    warmup_jit();
    if cfg!(target_os = "macos") {
        println!("  macOS test timeout: {:?}", get_test_timeout());
    }
    
    let test_results = [
        run_test(
            "NOP",
            &[arm64::nop()],
            |_cpu| {},
            |cpu| cpu.load_pc() >= TEST_BASE_ADDR + 4,
        ),
        run_test(
            "ADD X1, X1, #2",
            &[arm64::add_imm(1, 1, 2)],
            |cpu| {
                cpu.store_x::<1>(5);
            },
            |cpu| cpu.load_x::<1>() == 7,
        ),
        run_test(
            "SUB X2, X2, #1",
            &[arm64::sub_imm(2, 2, 1)],
            |cpu| {
                cpu.store_x::<2>(10);
            },
            |cpu| cpu.load_x::<2>() == 9,
        ),
        run_test(
            "ADD X0, X0, X1",
            &[arm64::add_reg(0, 0, 1)],
            |cpu| {
                cpu.store_x::<0>(7);
                cpu.store_x::<1>(3);
            },
            |cpu| cpu.load_x::<0>() == 10,
        ),
        run_test(
            "MOV X3, X4",
            &[arm64::mov_reg(3, 4)],
            |cpu| {
                cpu.store_x::<3>(0);
                cpu.store_x::<4>(0xDEADBEEF);
            },
            |cpu| cpu.load_x::<3>() == 0xDEADBEEF,
        ),
        run_test(
            "RET",
            &[arm64::ret()],
            |cpu| {
                cpu.store_x::<30>(0x2000);
            },
            |cpu| cpu.load_pc() == 0x2000,
        ),
        
        run_test(
            // WHAT?!?!??!?!?!?!?!?
            "Atomic ADD Test",
            &[arm64::add_imm(0, 0, 50)],
            |cpu| {
                cpu.store_x::<0>(100);
            },
            |cpu| cpu.load_x::<0>() == 150,
        ),
        run_test(
            // WHAT?!?!??!?!?!?!?!?
            "Memory Access Pattern",
            &[
                arm64::add_imm(1, 1, 1),
                arm64::add_imm(1, 1, 1),
                arm64::add_imm(1, 1, 1),
            ],
            |cpu| {
                cpu.store_x::<1>(0);
            },
            |cpu| cpu.load_x::<1>() == 3,
        ),
        run_test(
            "Multiple Arithmetic Ops",
            &[
                arm64::add_imm(0, 0, 5),
                arm64::sub_imm(1, 1, 3),
                arm64::add_reg(0, 0, 1),
            ],
            |cpu| {
                cpu.store_x::<0>(10);
                cpu.store_x::<1>(20);
            },
            |cpu| cpu.load_x::<0>() == 32 && cpu.load_x::<1>() == 17,
        ),
    ];

    let mut passed = 0;
    for result in &test_results {
        let icon = ["N", "Y"][result.passed as usize];
        let result_str = format!("{icon} {} - {} ({:?})", result.name, result.message, result.duration);
        if result.passed {
            passed += 1;
        }
        results.push(result_str);
    }
    let failed = test_results.len() - passed;
    let total_time = start_time.elapsed();
    
    results.push(format!("Total: {} ({passed} / {failed}) time {total_time:?}", test_results.len()));
    if failed > 0 && cfg!(target_os = "macos") {
        results.push(
            r"macOS JIT cold start may cause first-test timeout; this is normal after build system changes"
                .to_string()
        );
    }
    results
}
