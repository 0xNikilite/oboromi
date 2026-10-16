use crate::gpu::compute::{ComputePipeline, HostBuffer};
use crate::gpu::sm86::{CBUF_BINDING, Decoder, GLOBAL_BINDING, STATE_WORDS, SUPPORT_BINDING};
use crate::gpu::spirv::Emitter;
use crate::gpu::spirv::capability::SHADER;
use crate::gpu::{VkError, VkState};
use ash::vk::DescriptorType;

pub fn translate_module(
    instructions: &[u128],
    register_io: bool,
    device: Option<&VkState>,
) -> Emitter {
    emit_module(register_io, device, 1, |decoder| decoder.translate_program(instructions))
}

pub fn emit_module(
    register_io: bool,
    device: Option<&VkState>,
    lanes: u32,
    body: impl FnOnce(&mut Decoder),
) -> Emitter {
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
        decoder.emit_register_io(10, 11);
    }

    body(&mut decoder);

    if register_io {
        decoder.emit_register_dump();
    }
    decoder.emit_entry_point(func, device, lanes);
    decoder.ir.emit_return();
    decoder.ir.emit_function_end();
    decoder.ir.finalize();
    emitter
}

pub fn spirv_val(module: &Emitter) {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let child = Command::new("spirv-val")
        .args(["--target-env", "vulkan1.2"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(e) => return eprintln!("skipping, spirv-val unavailable: {e}"),
    };
    child.stdin.take().unwrap().write_all(&module.to_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "{out:?}");
}

pub const GLOBAL_BASE: u64 = 0x2_0000_1000;
pub const PRED: usize = 254;
pub const UREG: usize = PRED + 1;
pub const UPRED: usize = STATE_WORDS - 1;

pub fn run_regs(
    instructions: &[u128],
    seed: &[u32],
    cbuf0: &[u32],
    global: &[u32],
) -> Option<(Vec<u32>, Vec<u32>)> {
    let (mut lanes, global) = run_lanes(instructions, &[seed.to_vec()], cbuf0, global)?;
    Some((lanes.remove(0), global))
}

pub fn run_lanes(
    instructions: &[u128],
    seeds: &[Vec<u32>],
    cbuf0: &[u32],
    global: &[u32],
) -> Option<(Vec<Vec<u32>>, Vec<u32>)> {
    run_body(|decoder| decoder.translate_program(instructions), seeds, cbuf0, global)
}

/// Runs `body` as one workgroup with an invocation per seed, the state words of invocation i seeded from `seeds[i]`,
/// cbuf slot 0 holding `cbuf0` and global memory at `GLOBAL_BASE` holding `global`, returning every invocation's
/// state words and the global words, or `None` when there is no Vulkan device.
pub fn run_body(
    body: impl FnOnce(&mut Decoder),
    seeds: &[Vec<u32>],
    cbuf0: &[u32],
    global: &[u32],
) -> Option<(Vec<Vec<u32>>, Vec<u32>)> {
    let state = match VkState::new() {
        Ok(s) => s,
        Err(e @ (VkError::Loader | VkError::NoDevice)) => {
            eprintln!("skipping: {e:?}");
            return None;
        }
        Err(e) => panic!("vulkan device creation failed: {e:?}"),
    };
    let module = emit_module(true, Some(&state), seeds.len() as u32, body);
    spirv_val(&module);
    let bytes = |words: &[u32]| {
        words
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .collect::<Vec<_>>()
    };
    let words: Vec<u32> = seeds
        .iter()
        .flat_map(|seed| seed.iter().copied().chain(std::iter::repeat(0)).take(STATE_WORDS))
        .collect();
    let size = 4 * words.len() as u64;
    let mut input = HostBuffer::new(&state, size).unwrap();
    input.write(&bytes(&words)).unwrap();
    let output = HostBuffer::new(&state, size).unwrap();
    let mut cbuf = HostBuffer::new(&state, 16 + 4 * cbuf0.len() as u64).unwrap();
    cbuf.write(&bytes(cbuf0)).unwrap();
    let mut memory = HostBuffer::new(&state, 4 * global.len().max(1) as u64).unwrap();
    memory.write(&bytes(global)).unwrap();
    let mut support = HostBuffer::new(&state, 16).unwrap();
    support.write(&GLOBAL_BASE.to_le_bytes()).unwrap();
    let (ssbo, ubo) = (DescriptorType::STORAGE_BUFFER, DescriptorType::UNIFORM_BUFFER);
    let bindings = [(10, ssbo), (11, ssbo), (GLOBAL_BINDING, ssbo), (SUPPORT_BINDING, ubo), (CBUF_BINDING, ubo)];
    let mut pipeline = ComputePipeline::new(&state, &module.words(), &bindings).unwrap();
    pipeline.bind(&[(10, &input), (11, &output), (GLOBAL_BINDING, &memory), (SUPPORT_BINDING, &support), (CBUF_BINDING, &cbuf)]);
    state.dispatch(&pipeline, [1, 1, 1]).unwrap();
    let read = |buf: &HostBuffer, len: usize| -> Vec<u32> {
        let mut out = vec![0u8; len * 4];
        buf.read(&mut out).unwrap();
        out.chunks(4)
            .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
            .collect()
    };
    let lanes = read(&output, words.len()).chunks(STATE_WORDS).map(<[u32]>::to_vec).collect();
    Some((lanes, read(&memory, global.len())))
}
