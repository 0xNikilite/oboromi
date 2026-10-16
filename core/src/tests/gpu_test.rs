use crate::gpu::sm86::Decoder;
use crate::gpu::spirv::Emitter;
use crate::gpu::spirv::capability::SHADER;
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
        let mut inst: u128 = 0x7810;
        inst |= ((rd as u128) & 0xff) << 16;
        inst |= ((ra as u128) & 0xff) << 24;
        inst |= ((imm32 as u128) & 0xffffffff) << 32;
        inst |= 0x7ffe0ffu128 << 64;
        inst
    }

    // iadd rd, ra, rb (src_type=1, opcode=0x210)
    pub fn iadd_reg(rd: u32, ra: u32, rb: u32) -> u128 {
        let mut inst: u128 = 0x7210;
        inst |= ((rd as u128) & 0xff) << 16;
        inst |= ((ra as u128) & 0xff) << 24;
        inst |= ((rb as u128) & 0xff) << 32;
        inst |= 0x7ffe0ffu128 << 64;
        inst
    }

    // iadd3 rd, ra, rb, rc (register form, key 0x210; rc must be 0xFF)
    pub fn iadd3_reg(rd: u32, ra: u32, rb: u32, rc: u32) -> u128 {
        let mut inst: u128 = 0x7210;
        inst |= ((rd as u128) & 0xff) << 16;
        inst |= ((ra as u128) & 0xff) << 24;
        inst |= ((rb as u128) & 0xff) << 32;
        inst |= ((rc as u128) & 0xff) << 64;
        inst |= 0x7ffe000u128 << 64;
        inst
    }

    // mov rd, imm32 (opcode=0x802)
    pub fn mov_imm(rd: u32, imm32: u32) -> u128 {
        let mut inst: u128 = 0x7802;
        inst |= 0xfu128 << 72;
        inst |= ((rd as u128) & 0xff) << 16;
        inst |= ((imm32 as u128) & 0xffffffff) << 32;
        inst
    }

    pub fn mov_reg(rd: u32, rb: u32) -> u128 {
        let mut inst: u128 = 0x7202;
        inst |= 0xfu128 << 72;
        inst |= ((rd as u128) & 0xff) << 16;
        inst |= ((rb as u128) & 0xff) << 32;
        inst
    }

    // kill (opcode=0x95b)
    pub fn kill(pred: u32, invert: bool) -> u128 {
        let mut inst: u128 = 0x95b | 0x7 << 87 | 0x3f << 110;
        inst |= ((pred as u128) & 0x7) << 12;
        if invert {
            inst |= 1 << 15;
        }
        inst
    }
}

fn translate_module(instructions: &[u128], register_io: bool) -> Emitter {
    let mut emitter = Emitter::new();
    emitter.emit_header();
    emitter.emit_capability(SHADER);
    emitter.emit_memory_model(0, 1);

    let mut decoder = Decoder::new(&mut emitter);
    decoder.init();

    // set up a function so translated instructions have a valid context
    let void_ty = decoder.get_type_void();
    let func_ty = decoder.ir.emit_type_function(void_ty, &[]);
    let func = decoder.ir.emit_function(void_ty, 0, func_ty);
    decoder.ir.emit_label();
    if register_io {
        decoder.emit_register_io(10, 9);
    }

    for &inst in instructions {
        decoder.translate(inst);
    }

    if register_io {
        decoder.emit_register_dump();
    }
    decoder.emit_entry_point(func);
    decoder.ir.emit_return();
    decoder.ir.emit_function_end();
    decoder.ir.finalize();
    emitter
}

fn run_translation_test(name: &str, instructions: &[u128]) -> TestResult {
    let start = Instant::now();
    println!("Running test: {} ({} instructions)", name, instructions.len());
    let result = catch_unwind(AssertUnwindSafe(|| {
        translate_module(instructions, false).validate()
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
        run_translation_test("IADD3 Register", &[inst::iadd3_reg(1, 2, 3, 0xFF)]),
        run_translation_test("MOV immediate", &[inst::mov_imm(1, 0xCAFEBABE)]),
        run_translation_test("MOV register", &[inst::mov_reg(1, 2)]),
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

#[test]
fn fixtures_match_nvdisasm() {
    let Some(nvdisasm) = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .chain(std::fs::read_dir("/usr/local").into_iter().flatten().flatten().map(|e| e.path().join("bin")))
        .map(|dir| dir.join("nvdisasm"))
        .find(|path| path.is_file())
    else {
        eprintln!("nvdisasm not found, skipping");
        return;
    };
    let bin = std::env::temp_dir().join(format!("oboromi-fixture-{}.bin", std::process::id()));
    let fixtures: &[(u128, &str)] = &[
        (inst::iadd_imm(1, 2, 42), "IADD3 R1, R2, 0x2a, RZ;"),
        (inst::iadd_reg(1, 2, 3), "IADD3 R1, R2, R3, RZ;"),
        (inst::iadd3_reg(1, 2, 3, 0xFF), "IADD3 R1, R2, R3, RZ;"),
        (inst::mov_imm(1, 0xCAFEBABE), "MOV R1, 0xcafebabe;"),
        (inst::mov_reg(1, 2), "MOV R1, R2;"),
        (inst::kill(7, false), "KILL;"),
    ];
    for &(word, expected) in fixtures {
        std::fs::write(&bin, word.to_le_bytes()).unwrap();
        let out = std::process::Command::new(&nvdisasm).args(["--binary", "SM86"]).arg(&bin).output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        let disasm = text.lines().find_map(|line| line.split("/*0000*/").nth(1)).unwrap_or_default();
        assert_eq!(disasm.trim(), expected, "{word:#x}");
    }
    std::fs::remove_file(bin).unwrap();
}

#[cfg(test)]
fn spirv_val(module: &Emitter, name: &str) {
    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, module.to_bytes()).unwrap();
    let spirv_val = std::process::Command::new("spirv-val")
        .args(["--target-env", "vulkan1.2"])
        .arg(&path)
        .output();
    match spirv_val {
        Ok(out) => assert!(out.status.success(), "{out:?}"),
        Err(e) => eprintln!("skipping, spirv-val unavailable: {e}"),
    }
}

#[test]
fn mov_immediate_passes_spirv_val() {
    let module = translate_module(&[inst::mov_imm(1, 0xCAFEBABE)], false);
    assert_eq!(module.words()[5..8], [2 << 16 | 17, SHADER, 3 << 16 | 14]);
    spirv_val(&module, "oboromi_mov_immediate.spv");
}

#[test]
fn translated_mov_and_iadd3_run_on_gpu() {
    use crate::gpu::compute::{ComputePipeline, HostBuffer};
    use crate::gpu::{VkError, VkState};
    use ash::vk::DescriptorType;
    let program = [inst::mov_imm(0, 42), inst::iadd_reg(1, 2, 3)];
    let module = translate_module(&program, true);
    spirv_val(&module, "oboromi_register_io.spv");
    let state = match VkState::new() {
        Ok(s) => s,
        Err(e @ (VkError::Loader | VkError::NoDevice)) => return eprintln!("skipping: {e:?}"),
        Err(e) => panic!("vulkan device creation failed: {e:?}"),
    };
    let mut regs = [0u32; 255];
    (regs[2], regs[3]) = (40, 2);
    let mut input = HostBuffer::new(&state, 255 * 4).unwrap();
    input.write(&regs.map(u32::to_le_bytes).concat());
    let mut output = HostBuffer::new(&state, 255 * 4).unwrap();
    output.write(&[0; 255 * 4]);
    let ssbo = DescriptorType::STORAGE_BUFFER;
    let bindings = [(10, ssbo), (9, ssbo)];
    let mut pipeline = ComputePipeline::new(&state, &module.words(), &bindings).unwrap();
    pipeline.bind(&[(10, &input), (9, &output)]);
    state.dispatch(&pipeline, [1, 1, 1]).unwrap();
    let mut out = [0u8; 12];
    output.read(&mut out);
    assert_eq!(out[..], [42u32, 42, 40].map(u32::to_le_bytes).concat());
}
