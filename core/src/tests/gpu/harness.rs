use crate::gpu::compute::{ComputePipeline, HostBuffer, HostImage};
use crate::gpu::cubin::{Arg, Kernel};
use crate::gpu::sm86::{self, CBUF_BINDING, Decoder, GLOBAL_BINDING, ResourceKind, STATE_WORDS, SUPPORT_BINDING, TEXTURE_BINDING};
use crate::gpu::spirv::Emitter;
use crate::gpu::texture::TexturePool;
use crate::gpu::{VkError, VkState};
use ash::vk::{self, DescriptorType};
use std::ops::Range;

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
    sm86::emit_module(device, [lanes, 1, 1], |decoder| {
        if register_io {
            decoder.emit_register_io(10, 11);
        }
        body(decoder);
        if register_io {
            decoder.emit_register_dump();
        }
    })
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

fn vk_state() -> Option<VkState> {
    match VkState::new() {
        Ok(s) => Some(s),
        Err(e @ (VkError::Loader | VkError::NoDevice)) => {
            eprintln!("skipping: {e:?}");
            None
        }
        Err(e) => panic!("vulkan device creation failed: {e:?}"),
    }
}

pub const GLOBAL_BASE: u64 = 0x2_0000_1000;
pub const PRED: usize = 254;
pub const UREG: usize = PRED + 1;
pub const UPRED: usize = STATE_WORDS - 2;
pub const LIVE: usize = STATE_WORDS - 1;

pub fn run_regs(
    instructions: &[u128],
    seed: &[u32],
    cbuf0: &[u32],
    global: &[u32],
) -> Option<(Vec<u32>, Vec<u32>)> {
    let (mut lanes, global) = run_lanes(instructions, &[seed.to_vec()], cbuf0, global)?;
    Some((lanes.remove(0), global))
}

pub fn run(program: &[u128], seed: &[(usize, u32)]) -> Option<Vec<u32>> {
    let mut words = vec![0; UPRED + 1];
    for &(slot, value) in seed {
        words[slot] = value;
    }
    run_regs(program, &words, &[], &[]).map(|(regs, _)| regs)
}

pub fn run_lanes(
    instructions: &[u128],
    seeds: &[Vec<u32>],
    cbuf0: &[u32],
    global: &[u32],
) -> Option<(Vec<Vec<u32>>, Vec<u32>)> {
    run_body(|decoder| decoder.translate_program(instructions), seeds, cbuf0, global)
}

fn pad_to_warps(seeds: &[Vec<u32>]) -> Vec<u32> {
    let idle = vec![0; STATE_WORDS];
    let mut words = Vec::new();
    for lane in 0..seeds.len().next_multiple_of(32) {
        let seed = seeds.get(lane).unwrap_or(&idle);
        let start = words.len();
        words.extend(seed.iter().copied().chain(std::iter::repeat(0)).take(STATE_WORDS));
        if seed.len() <= LIVE {
            words[start + LIVE] = 1;
        }
    }
    words
}

pub fn run_body(
    body: impl FnOnce(&mut Decoder),
    seeds: &[Vec<u32>],
    cbuf0: &[u32],
    global: &[u32],
) -> Option<(Vec<Vec<u32>>, Vec<u32>)> {
    run_textured(body, seeds, cbuf0, global, &Textures::default())
}

/// Stands in for guest state the harness does not map: the handle every bound texture and surface reads from its cbuf
/// word, the TIC/TSC pools, and the texel bytes at each address a header can point to.
#[derive(Default)]
pub struct Textures<'a> {
    pub handle: u32,
    pub pool: TexturePool<'a>,
    pub memory: &'a [(u64, &'a [u8])],
}

/// Runs `body` as one workgroup of whole warps with an invocation per seed, the state words of invocation i seeded
/// from `seeds[i]`, every cbuf slot holding `cbuf0`, global memory at `GLOBAL_BASE` holding `global` and every texture or
/// surface the body declares resolved through `textures`, returning every seeded invocation's state words and the
/// global words, or `None` when there is no Vulkan device.
pub fn run_textured(
    body: impl FnOnce(&mut Decoder),
    seeds: &[Vec<u32>],
    cbuf0: &[u32],
    global: &[u32],
    textures: &Textures,
) -> Option<(Vec<Vec<u32>>, Vec<u32>)> {
    let state = vk_state()?;
    let words = pad_to_warps(seeds);
    let mut resources = Vec::new();
    let module = emit_module(true, Some(&state), (words.len() / STATE_WORDS) as u32, |decoder| {
        body(decoder);
        resources = decoder.resources.clone();
    });
    spirv_val(&module);
    let float64 = module.words().windows(2).any(|words| words == [2 << 16 | 17, 10]);
    if float64 && !state.shader_float64 {
        eprintln!("skipping: Vulkan device lacks shaderFloat64");
        return None;
    }
    let bytes = |words: &[u32]| {
        words
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .collect::<Vec<_>>()
    };
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
    let image = |format: Option<vk::Format>| {
        let header = textures.pool.texture(textures.handle).unwrap();
        assert!(format.is_none_or(|f| f == header.format), "a {:?} header cannot back a {format:?} surface", header.format);
        let texels = textures.memory.iter().find(|(address, _)| *address == header.address).expect("no texels at the header address").1;
        HostImage::new(&state, &header, texels, &textures.pool.sampler(textures.handle).unwrap()).unwrap()
    };
    let image_types = resources.iter().filter_map(|r| match r.kind {
        ResourceKind::Texture2D { .. } => Some((DescriptorType::COMBINED_IMAGE_SAMPLER, None)),
        ResourceKind::Surface2D { format, .. } => Some((DescriptorType::STORAGE_IMAGE, Some(format))),
        _ => None,
    });
    let images: Vec<(u32, DescriptorType, HostImage)> = (TEXTURE_BINDING..).zip(image_types).map(|(b, (ty, format))| (b, ty, image(format))).collect();
    let cbufs: Vec<u32> = resources.iter().filter_map(|r| match r.kind {
        ResourceKind::Cbuf(slot) if slot > 0 => Some(CBUF_BINDING + slot),
        _ => None,
    }).collect();
    let buffers = [(10, ssbo), (11, ssbo), (GLOBAL_BINDING, ssbo), (SUPPORT_BINDING, ubo), (CBUF_BINDING, ubo)];
    let bindings: Vec<_> = buffers.into_iter().chain(cbufs.iter().map(|&binding| (binding, ubo))).chain(images.iter().map(|&(binding, ty, _)| (binding, ty))).collect();
    let mut pipeline = ComputePipeline::new(&state, &module.words(), &bindings).unwrap();
    let mut buffers = vec![(10, &input), (11, &output), (GLOBAL_BINDING, &memory), (SUPPORT_BINDING, &support), (CBUF_BINDING, &cbuf)];
    buffers.extend(cbufs.iter().map(|&binding| (binding, &cbuf)));
    pipeline.bind(&buffers);
    pipeline.bind_images(&images.iter().map(|(binding, _, image)| (*binding, image)).collect::<Vec<_>>());
    state.dispatch(&pipeline, [1, 1, 1]).unwrap();
    let read = |buf: &HostBuffer, len: usize| -> Vec<u32> {
        let mut out = vec![0u8; len * 4];
        buf.read(&mut out).unwrap();
        out.chunks(4)
            .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
            .collect()
    };
    let lanes: Vec<Vec<u32>> = read(&output, words.len()).chunks(STATE_WORDS).take(seeds.len()).map(<[u32]>::to_vec).collect();
    assert!(lanes.iter().all(|lane| lane[LIVE] < 2), "a lane was left waiting on a barrier");
    Some((lanes, read(&memory, global.len())))
}

/// A translated program launched the way a driver does, without register io: each of `banks` as the (bank, bytes) of a
/// constant bank and global memory holding each region's bytes at its guest address.
pub struct Grid<'a> {
    state: &'a VkState,
    pipeline: ComputePipeline<'a>,
    global: HostBuffer<'a>,
    _support: HostBuffer<'a>,
    _cbufs: Vec<(u32, HostBuffer<'a>)>,
    memory: Vec<u8>,
    spans: Vec<Range<usize>>,
}

impl<'a> Grid<'a> {
    pub fn new(state: &'a VkState, instructions: &[u128], block: [u32; 3], banks: &[(u32, Vec<u8>)], regions: &[(u64, &[u8])]) -> Self {
        let module = sm86::emit_module(Some(state), block, |decoder| decoder.translate_program(instructions));
        spirv_val(&module);
        let base = regions.iter().map(|&(address, _)| address).min().unwrap() & !0xf;
        let end = regions.iter().map(|&(address, bytes)| address + bytes.len() as u64).max().unwrap();
        let spans: Vec<_> = regions.iter().map(|&(address, bytes)| (address - base) as usize..(address - base) as usize + bytes.len()).collect();
        let mut memory = vec![0; (end - base) as usize];
        for (span, region) in spans.iter().zip(regions) {
            memory[span.clone()].copy_from_slice(region.1);
        }
        let mut global = HostBuffer::new(state, memory.len() as u64).unwrap();
        global.write(&memory).unwrap();
        let buffer = |bytes: &[u8]| {
            let mut buffer = HostBuffer::new(state, 16 + bytes.len() as u64).unwrap();
            buffer.write(bytes).unwrap();
            buffer
        };
        let support = buffer(&base.to_le_bytes());
        let cbufs: Vec<_> = banks.iter().map(|(bank, bytes)| (CBUF_BINDING + bank, buffer(bytes))).collect();
        let (ssbo, ubo) = (DescriptorType::STORAGE_BUFFER, DescriptorType::UNIFORM_BUFFER);
        let bindings: Vec<_> = [(GLOBAL_BINDING, ssbo), (SUPPORT_BINDING, ubo)].into_iter().chain(cbufs.iter().map(|&(binding, _)| (binding, ubo))).collect();
        let mut pipeline = ComputePipeline::new(state, &module.words(), &bindings).unwrap();
        pipeline.bind(&[(GLOBAL_BINDING, &global), (SUPPORT_BINDING, &support)].into_iter().chain(cbufs.iter().map(|(binding, buffer)| (*binding, buffer))).collect::<Vec<_>>());
        Self { state, pipeline, global, _support: support, _cbufs: cbufs, memory, spans }
    }

    /// Dispatches `groups` workgroups and returns every region's bytes after them.
    pub fn run(&mut self, groups: [u32; 3]) -> Vec<Vec<u8>> {
        self.state.dispatch(&self.pipeline, groups).unwrap();
        self.global.read(&mut self.memory).unwrap();
        self.spans.iter().map(|span| self.memory[span.clone()].to_vec()).collect()
    }
}

/// Launches `instructions` as `grid` workgroups of `block` invocations with cbuf slot 0 holding `cbuf0`, returning every
/// region's bytes after the launch, or `None` when there is no Vulkan device.
pub fn run_grid(instructions: &[u128], block: [u32; 3], grid: [u32; 3], cbuf0: &[u8], regions: &[(u64, &[u8])]) -> Option<Vec<Vec<u8>>> {
    let state = vk_state()?;
    Some(Grid::new(&state, instructions, block, &[(0, cbuf0.to_vec())], regions).run(grid))
}

/// Launches kernel `name` of `cubin` the way cuLaunchKernel does with `args` in parameter order, each buffer placed in
/// global memory from `GLOBAL_BASE` up, returning the buffers after the launch, or `None` when there is no Vulkan device.
pub fn run_kernel(cubin: &[u8], name: &str, grid: [u32; 3], block: [u32; 3], args: &[Arg]) -> Option<Vec<Vec<u8>>> {
    let kernel = Kernel::load(cubin, name).unwrap();
    let (banks, regions) = kernel.launch(grid, block, args, GLOBAL_BASE);
    let state = vk_state()?;
    Some(Grid::new(&state, &kernel.words, block, &banks, &regions).run(grid))
}
