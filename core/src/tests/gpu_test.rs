use crate::gpu::sm86::Decoder;
use crate::gpu::spirv::Emitter;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub message: String,
    pub duration: Duration,
}

// helpers to build sm86 instructions
mod inst {
    // iadd rd, ra, imm32 (src_type=4, opcode=0x810)
    pub fn iadd_imm(rd: u32, ra: u32, imm32: u32) -> u128 {
        let mut inst: u128 = 0x810;
        inst |= ((rd as u128) & 0xff) << 16;
        inst |= ((ra as u128) & 0xff) << 24;
        inst |= ((imm32 as u128) & 0xffffffff) << 32;
        inst |= 0xFFu128 << 64; // rc=0xFF to bypass assert
        inst
    }

    // iadd rd, ra, rb (src_type=1, opcode=0x210)
    pub fn iadd_reg(rd: u32, ra: u32, rb: u32) -> u128 {
        let mut inst: u128 = 0x210;
        inst |= ((rd as u128) & 0xff) << 16;
        inst |= ((ra as u128) & 0xff) << 24;
        inst |= ((rb as u128) & 0xff) << 32;
        inst |= 0xFFu128 << 64; // rc=0xFF to bypass assert
        inst
    }

    // iadd3 rd, ra, rb, rc (opcode=0x510, variant 0x1510 via bit91)
    pub fn iadd3_reg(rd: u32, ra: u32, rb: u32, rc: u32) -> u128 {
        let mut inst: u128 = 0x510;
        inst |= 1u128 << 91; // use 0x1510 variant to avoid iadd overlap
        inst |= ((rd as u128) & 0xff) << 16;
        inst |= ((ra as u128) & 0xff) << 24;
        inst |= ((rb as u128) & 0xff) << 32;
        inst |= ((rc as u128) & 0xff) << 64;
        inst
    }

    // iadd32i rd, ra, imm32 (opcode=0x410)
    pub fn iadd32i(rd: u32, ra: u32, imm32: u32) -> u128 {
        let mut inst: u128 = 0x410;
        inst |= ((rd as u128) & 0xff) << 16;
        inst |= ((ra as u128) & 0xff) << 24;
        inst |= ((imm32 as u128) & 0xffffffff) << 32;
        inst
    }

    // kill (opcode=0x8e0)
    pub fn kill(pred: u32, invert: bool) -> u128 {
        let mut inst: u128 = 0x8e0;
        inst |= ((pred as u128) & 0x7) << 12;
        if invert {
            inst |= 1 << 15;
        }
        inst
    }
}

fn run_translation_test(name: &str, instructions: &[u128]) -> TestResult {
    let start = Instant::now();
    println!("Running test: {} ({} instructions)", name, instructions.len());
    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut emitter = Emitter::new();
        emitter.emit_header();
        emitter.emit_capability(crate::gpu::spirv::capability::SHADER);
        emitter.emit_memory_model(0, 1);

        let mut decoder = Decoder::new(&mut emitter);
        decoder.init();

        // set up a function so translated instructions have a valid context
        let void_ty = decoder.get_type_void();
        let func_ty = decoder.ir.emit_type_function(void_ty, &[]);
        let _func = decoder.ir.emit_function(void_ty, 0, func_ty);
        decoder.ir.emit_label();

        for &inst in instructions {
            decoder.translate(inst);
        }

        decoder.ir.emit_return();
        decoder.ir.emit_function_end();
        decoder.ir.finalize();
        decoder.ir.validate();
    }));

    let duration = start.elapsed();
    match result {
        Ok(()) => TestResult {
            name: name.to_string(),
            passed: true,
            message: "PASS".to_string(),
            duration,
        },
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else {
                "unknown panic".to_string()
            };
            TestResult {
                name: name.to_string(),
                passed: false,
                message: format!("FAIL: {}", msg),
                duration,
            }
        }
    }
}

pub fn run_gpu_tests() -> Vec<String> {
    let mut results = Vec::new();
    let start_time = Instant::now();

    results.push("Starting GPU/SM86 Decoder Tests...".to_string());
    println!("Starting GPU/SM86 Decoder Tests...");

    let tests = vec![
        run_translation_test("IADD Immediate", &[inst::iadd_imm(1, 2, 42)]),
        run_translation_test("IADD Register", &[inst::iadd_reg(1, 2, 3)]),
        run_translation_test("IADD3 Register", &[inst::iadd3_reg(1, 2, 3, 4)]),
        run_translation_test("IADD32I", &[inst::iadd32i(1, 2, 100)]),
        run_translation_test("KILL PT", &[inst::kill(7, false)]),
    ];

    let mut passed = 0;
    for t in &tests {
        let icon = if t.passed { "Y" } else { "N" };
        let line = format!(
            "{} {} - {} ({:?})",
            icon, t.name, t.message, t.duration
        );
        println!("{}", line);
        results.push(line);
        if t.passed {
            passed += 1;
        }
    }

    let failed = tests.len() - passed;
    let total_time = start_time.elapsed();
    let summary = format!(
        "Total: {} ({}/{} passed) time {:?}",
        tests.len(),
        passed,
        failed,
        total_time
    );
    println!("{}", summary);
    results.push(summary);

    results
}
