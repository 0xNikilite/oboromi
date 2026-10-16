use crate::gpu::compute::{ComputePipeline, HostBuffer};
use crate::gpu::sm86::{Decoder, STATE_WORDS};
use crate::gpu::spirv::Emitter;
use crate::gpu::spirv::capability::SHADER;
use crate::gpu::{VkError, VkState};
use ash::vk::DescriptorType;

pub fn translate_module(instructions: &[u128], register_io: bool) -> Emitter {
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

    decoder.translate_program(instructions);

    if register_io {
        decoder.emit_register_dump();
    }
    decoder.emit_entry_point(func);
    decoder.ir.emit_return();
    decoder.ir.emit_function_end();
    decoder.ir.finalize();
    emitter
}

pub fn spirv_val(module: &Emitter) {
    let name = std::thread::current()
        .name()
        .unwrap_or("main")
        .replace("::", "_");
    let path = std::env::temp_dir().join(format!("oboromi_{name}.spv"));
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

/// Runs `instructions` once with the state words seeded from `seed` and cbuf slot 0 holding `cbuf0`,
/// returning every state word, or `None` when there is no Vulkan device.
pub fn run_regs(instructions: &[u128], seed: &[u32], cbuf0: &[u32]) -> Option<Vec<u32>> {
    let module = translate_module(instructions, true);
    spirv_val(&module);
    let state = match VkState::new() {
        Ok(s) => s,
        Err(e @ (VkError::Loader | VkError::NoDevice)) => {
            eprintln!("skipping: {e:?}");
            return None;
        }
        Err(e) => panic!("vulkan device creation failed: {e:?}"),
    };
    let bytes = |words: &[u32]| {
        words
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .collect::<Vec<_>>()
    };
    let mut words = seed.to_vec();
    words.resize(STATE_WORDS, 0);
    let size = (STATE_WORDS * 4) as u64;
    let mut input = HostBuffer::new(&state, size).unwrap();
    input.write(&bytes(&words));
    let output = HostBuffer::new(&state, size).unwrap();
    let mut cbuf = HostBuffer::new(&state, 16 + 4 * cbuf0.len() as u64).unwrap();
    cbuf.write(&bytes(cbuf0));
    let ssbo = DescriptorType::STORAGE_BUFFER;
    let bindings = [(10, ssbo), (9, ssbo), (1, DescriptorType::UNIFORM_BUFFER)];
    let mut pipeline = ComputePipeline::new(&state, &module.words(), &bindings).unwrap();
    pipeline.bind(&[(10, &input), (9, &output), (1, &cbuf)]);
    state.dispatch(&pipeline, [1, 1, 1]).unwrap();
    let mut out = vec![0u8; STATE_WORDS * 4];
    output.read(&mut out);
    Some(
        out.chunks(4)
            .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
            .collect(),
    )
}
