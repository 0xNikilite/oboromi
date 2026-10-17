//! A GPU channel over guest RAM: pushbuffers run through the pusher, and each draw or compute launch translates the
//! guest's programs, mirrors the buffers and textures it reads and writes what it renders or stores back to guest
//! memory, so guest-physical pages stay the only identity of every resource.
use super::compute::{ComputePipeline, HostBuffer, HostImage};
use super::compute_class::{DescriptorPool, DispatchSnapshot};
use super::fifo::{Event, Pusher};
use super::graphics::{DrawState, GraphicsPipeline, RasterState, RenderTarget, TargetState, VertexLayout};
use super::memory::{Mapping, MemoryError, MemoryTracker, PAGE, PaRange, PageMap, Writer};
use super::pipeline_cache::{Disk, Lookup, PipelineBuilder, PipelinePolicy, hash};
use super::sm86::{CBUF_BINDING, GLOBAL_BINDING, ResourceKind, SUPPORT_BINDING, Stage, TEXTURE_BINDING, branch_target, emit_module};
use super::texture::TexturePool;
use super::threed::{Blend, ColourTarget, ConstantBuffer, DepthTarget, DrawSnapshot, MemoryLayout, PrimitiveTopology, ShaderProgram, VertexAttribute};
use super::{VkError, VkState};
use ash::vk;
use std::collections::{HashMap, hash_map::Entry};
use std::path::Path;
use std::time::Instant;

// SM 7.3 and later lead each graphics program with the 32-word SPHV4 header (NAK sph.rs), not the 20-word SPHV3 one.
const SPH_BYTES: usize = 0x80;
const SPH_VERSION: u32 = 4;
const VERTEX_PIPELINE: usize = 1;
const PIXEL_PIPELINE: usize = 5;
const SHADER_TYPE_VERTEX: u32 = 1;
const SHADER_TYPE_PIXEL: u32 = 5;
const A8B8G8R8: u8 = 0xd5;
// CLEAR_SURFACE bits (clc797.h): Z_ENABLE, and R, G, B and A of the target MRT_SELECT names.
const CLEAR_Z: u32 = 0x01;
const CLEAR_RGBA: u32 = 0x3c;

/// The fields of a version 4 shader program header (clc797sph.h) the translation depends on.
#[derive(Debug)]
struct ProgramHeader {
    sph_type: u32,
    version: u32,
    shader_type: u32,
    /// Bit n: generic attribute n is read.
    inputs: u32,
    /// Bit n: generic attribute n is written, by a type 1 header.
    outputs: u32,
    flat: u32,
    screen_linear: u32,
    colour_targets: u32,
    depth: bool,
}

impl ProgramHeader {
    fn new(sph: &[u8; SPH_BYTES]) -> Self {
        let field = |lo: usize, bits: usize| (lo..lo + bits).rev().fold(0, |v, i| v << 1 | u32::from(sph[i / 8] >> (i % 8) & 1));
        let generics = |used: &dyn Fn(usize) -> bool| (0..32).filter(|&i| used(i)).fold(0, |mask, i| mask | 1 << i);
        let pixel = field(0, 5) == 2;
        // A type 2 IMAP has two bits per component: 1 constant, 2 perspective, 3 screen linear.
        let pixel_mode = |i: usize, mode: &dyn Fn(u32) -> bool| pixel && (0..4).any(|c| mode(field(192 + 8 * i + 2 * c, 2)));
        Self {
            sph_type: field(0, 5),
            version: field(5, 5),
            shader_type: field(10, 4),
            inputs: generics(&|i| pixel_mode(i, &|m| m != 0) || !pixel && field(192 + 4 * i, 4) != 0),
            outputs: generics(&|i| !pixel && field(432 + 4 * i, 4) != 0),
            flat: generics(&|i| pixel_mode(i, &|m| m == 1)),
            screen_linear: generics(&|i| pixel_mode(i, &|m| m == 3)),
            colour_targets: if pixel { field(576, 32) } else { 0 },
            depth: pixel && field(609, 1) != 0,
        }
    }
}

struct Module {
    header: ProgramHeader,
    words: Vec<u32>,
    resources: Vec<ResourceKind>,
}

#[derive(PartialEq, Eq, Hash, Clone)]
enum PipelineKey {
    Draw {
        shaders: [u64; 2],
        streams: Vec<vk::VertexInputRate>,
        attributes: Vec<(u32, u32, vk::Format, u32)>,
        topology: vk::PrimitiveTopology,
        /// SET_BLEND, its equations and SET_CT_WRITE, unless VK_EXT_extended_dynamic_state3 makes them dynamic.
        blend: Option<(Option<Blend>, u8)>,
        depth: Option<vk::Format>,
    },
    Dispatch(u64),
}

enum Pipeline {
    Draw(GraphicsPipeline<'static>),
    Dispatch(ComputePipeline<'static>),
}

#[derive(PartialEq, Eq, Hash)]
struct TargetKey {
    ranges: Vec<PaRange>,
    format: u8,
    extent: (u32, u32),
    depth: Option<(u64, vk::Format)>,
}

/// A texture's guest pages and the TIC and TSC entries it was decoded from.
type TextureKey = (Vec<PaRange>, [u8; 32], [u8; 32]);

/// A colour target and the depth target drawn with it, which lives only on the host: nothing reads guest depth.
struct Target<'a> {
    render_target: RenderTarget<'a>,
    /// The generation of the target's pages when its image last matched them.
    generation: u64,
    depth_cleared: bool,
}

pub struct Gpu<'a> {
    state: &'static VkState,
    pub ram: Vec<u8>,
    pub space: PageMap,
    pub memory: MemoryTracker,
    pusher: Pusher,
    modules: HashMap<u64, Module>,
    pipelines: PipelineBuilder<'static, PipelineKey, Pipeline>,
    kernels: HashMap<u64, Vec<(u32, vk::DescriptorType)>>,
    /// Guest programs translated rather than read back from the disk cache.
    pub translated: usize,
    support: Option<HostBuffer<'a>>,
    targets: HashMap<TargetKey, Target<'a>>,
    mirrors: HashMap<Vec<PaRange>, (u64, Mapping<'a>)>,
    textures: HashMap<TextureKey, (u64, HostImage<'a>)>,
    pending_clear: Option<(ColourTarget, [f32; 4])>,
    pending_depth_clear: Option<(DepthTarget, f32)>,
    pixels: Vec<u8>,
    /// The report timestamp counts nanoseconds from here.
    started: Instant,
}

fn named(e: impl std::fmt::Debug) -> String {
    format!("{e:?}")
}

fn generation(memory: &MemoryTracker, ranges: &[PaRange]) -> u64 {
    ranges.iter().map(|r| memory.generation(r.start, r.end - r.start)).max().unwrap_or(0)
}

fn gather(space: &PageMap, ram: &[u8], va: u64, out: &mut [u8]) -> Result<(), MemoryError> {
    let mut rest = out;
    for range in space.resolve(va, rest.len() as u64)? {
        let (head, tail) = rest.split_at_mut((range.end - range.start) as usize);
        head.copy_from_slice(&ram[range.start as usize..range.end as usize]);
        rest = tail;
    }
    Ok(())
}

// SET_VERTEX_ATTRIBUTE_A NUMERICAL_TYPE values from clc797.h.
const SINT: u8 = 3;
const UINT: u8 = 4;
const FLOAT: u8 = 7;

fn vertex_format(attribute: VertexAttribute) -> Result<vk::Format, String> {
    use vk::Format as F;
    let unsupported = || Err(format!("vertex attribute widths {:#x} type {} swap {} is not implemented", attribute.size, attribute.kind, attribute.bgra));
    const R32_G32_B32_A32: u8 = 0x01;
    const R32_G32_B32: u8 = 0x02;
    const R16_G16_B16_A16: u8 = 0x03;
    const R32_G32: u8 = 0x04;
    const R16_G16_B16: u8 = 0x05;
    const R8_G8_B8_A8: u8 = 0x0a;
    const R16_G16: u8 = 0x0f;
    const R32: u8 = 0x12;
    const R8_G8_B8: u8 = 0x13;
    const R8_G8: u8 = 0x18;
    const R16: u8 = 0x1b;
    const R8: u8 = 0x1d;
    const A8B8G8R8: u8 = 0x2f;
    const A2B10G10R10: u8 = 0x30;
    const B10G11R11: u8 = 0x31;
    const G8R8: u8 = 0x32;
    // COMPONENT_BIT_WIDTHS (clc797.h) by the first member of the Vulkan family it reads as, and that family's size.
    let (family, members) = match (attribute.size, attribute.bgra) {
        (R32_G32_B32_A32, false) => (F::R32G32B32A32_UINT, 3),
        (R32_G32_B32, false) => (F::R32G32B32_UINT, 3),
        (R32_G32, false) => (F::R32G32_UINT, 3),
        (R32, false) => (F::R32_UINT, 3),
        (R16_G16_B16_A16, false) => (F::R16G16B16A16_UNORM, 7),
        (R16_G16_B16, false) => (F::R16G16B16_UNORM, 7),
        (R16_G16, false) => (F::R16G16_UNORM, 7),
        (R16, false) => (F::R16_UNORM, 7),
        (R8_G8_B8_A8 | A8B8G8R8, false) => (F::R8G8B8A8_UNORM, 6),
        (R8_G8_B8_A8 | A8B8G8R8, true) => (F::B8G8R8A8_UNORM, 6),
        (R8_G8_B8, false) => (F::R8G8B8_UNORM, 6),
        (R8_G8_B8, true) => (F::B8G8R8_UNORM, 6),
        (R8_G8 | G8R8, false) => (F::R8G8_UNORM, 6),
        (R8, false) => (F::R8_UNORM, 6),
        (A2B10G10R10, false) => (F::A2B10G10R10_UNORM_PACK32, 6),
        (A2B10G10R10, true) => (F::A2R10G10B10_UNORM_PACK32, 6),
        (B10G11R11, false) if attribute.kind == FLOAT => return Ok(F::B10G11R11_UFLOAT_PACK32),
        _ => return unsupported(),
    };
    // Vulkan orders a 32-bit family UINT, SINT, SFLOAT and the others UNORM, SNORM, USCALED, SSCALED, UINT, SINT and
    // for 16 bits SFLOAT, where NUMERICAL_TYPE counts SNORM, UNORM, SINT, UINT, USCALED, SSCALED, FLOAT from 1.
    let member = match (members, attribute.kind) {
        (3, UINT) => 0,
        (3, SINT) => 1,
        (3, FLOAT) => 2,
        (6 | 7, kind @ 1..=6) => [1, 0, 5, 4, 2, 3][kind as usize - 1],
        (7, FLOAT) => 6,
        _ => return unsupported(),
    };
    Ok(F::from_raw(family.as_raw() + member))
}

// SET_BLEND_COLOR_OP and the coefficient methods take OGL enums or D3D ones (clc797.h).
fn blend_op(op: u32) -> Result<vk::BlendOp, String> {
    use vk::BlendOp as B;
    Ok(match op {
        0x8006 | 1 => B::ADD,
        0x800a | 2 => B::SUBTRACT,
        0x800b | 3 => B::REVERSE_SUBTRACT,
        0x8007 | 4 => B::MIN,
        0x8008 | 5 => B::MAX,
        op => return Err(format!("blend op {op:#x} is not implemented")),
    })
}

fn blend_factor(coefficient: u32) -> Result<vk::BlendFactor, String> {
    use vk::BlendFactor as F;
    Ok(match coefficient {
        0x4000 | 1 => F::ZERO,
        0x4001 | 2 => F::ONE,
        0x4300 | 3 => F::SRC_COLOR,
        0x4301 | 4 => F::ONE_MINUS_SRC_COLOR,
        0x4302 | 5 => F::SRC_ALPHA,
        0x4303 | 6 => F::ONE_MINUS_SRC_ALPHA,
        0x4304 | 7 => F::DST_ALPHA,
        0x4305 | 8 => F::ONE_MINUS_DST_ALPHA,
        0x4306 | 9 => F::DST_COLOR,
        0x4307 | 0xa => F::ONE_MINUS_DST_COLOR,
        0x4308 | 0xb => F::SRC_ALPHA_SATURATE,
        0xc001 | 0xe => F::CONSTANT_COLOR,
        0xc002 | 0xf => F::ONE_MINUS_CONSTANT_COLOR,
        0xc003 => F::CONSTANT_ALPHA,
        0xc004 => F::ONE_MINUS_CONSTANT_ALPHA,
        coefficient => return Err(format!("blend coefficient {coefficient:#x} is not implemented")),
    })
}

fn blend_state(blend: Option<Blend>, write_mask: u8) -> Result<vk::PipelineColorBlendAttachmentState, String> {
    let state = vk::PipelineColorBlendAttachmentState::default().color_write_mask(vk::ColorComponentFlags::from_raw(write_mask.into()));
    let Some(Blend { colour: [colour_op, colour_source, colour_dest], alpha: [alpha_op, alpha_source, alpha_dest] }) = blend else { return Ok(state) };
    Ok(state
        .blend_enable(true)
        .color_blend_op(blend_op(colour_op)?)
        .src_color_blend_factor(blend_factor(colour_source)?)
        .dst_color_blend_factor(blend_factor(colour_dest)?)
        .alpha_blend_op(blend_op(alpha_op)?)
        .src_alpha_blend_factor(blend_factor(alpha_source)?)
        .dst_alpha_blend_factor(blend_factor(alpha_dest)?))
}

fn topology(topology: PrimitiveTopology) -> Result<vk::PrimitiveTopology, String> {
    use vk::PrimitiveTopology as T;
    Ok(match topology {
        PrimitiveTopology::Points => T::POINT_LIST,
        PrimitiveTopology::Lines => T::LINE_LIST,
        PrimitiveTopology::LineStrip => T::LINE_STRIP,
        PrimitiveTopology::Triangles => T::TRIANGLE_LIST,
        PrimitiveTopology::TriangleStrip => T::TRIANGLE_STRIP,
        PrimitiveTopology::TriangleFan => T::TRIANGLE_FAN,
        topology => return Err(format!("{topology:?} draws are not implemented")),
    })
}

fn colour_extent(ct: ColourTarget) -> Result<vk::Extent2D, String> {
    if ct.format != A8B8G8R8 || ct.layout != MemoryLayout::Pitch {
        return Err(format!("colour target format {:#x} with {:?} layout is not implemented", ct.format, ct.layout));
    }
    // A pitch-linear colour target's WIDTH holds its pitch in bytes (Ryujinx RtColorState.WidthOrStride).
    Ok(vk::Extent2D { width: ct.width / 4, height: ct.height })
}

fn raster_state(draw: &DrawSnapshot, extent: vk::Extent2D, topology: vk::PrimitiveTopology, blend: vk::PipelineColorBlendAttachmentState) -> RasterState {
    let ([sx, mut sy, _], [ox, mut oy, _]) = (draw.viewport.scale, draw.viewport.offset);
    let s = draw.scissor;
    let [x_min, x_max, mut y_min, mut y_max] = match s.enabled {
        true => [s.x_min, s.x_max, s.y_min, s.y_max].map(u32::from),
        false => [0, extent.width, 0, extent.height],
    };
    if draw.lower_left_origin {
        (sy, oy, y_min, y_max) = (-sy, extent.height as f32 - oy, extent.height.saturating_sub(y_max), extent.height.saturating_sub(y_min));
    }
    let scissor = vk::Rect2D {
        offset: vk::Offset2D { x: x_min as i32, y: y_min as i32 },
        extent: vk::Extent2D { width: x_max.saturating_sub(x_min), height: y_max.saturating_sub(y_min) },
    };
    // OGL_SET_CULL_FACE and OGL_SET_FRONT_FACE take GL enums, and SET_DEPTH_FUNC GL (0x200 + op) or D3D (1 + op) ones in Vulkan's order.
    let cull_face = match (draw.cull_enabled, draw.cull_face) {
        (false, _) => vk::CullModeFlags::NONE,
        (true, 0x404) => vk::CullModeFlags::FRONT,
        (true, 0x405) => vk::CullModeFlags::BACK,
        (true, _) => vk::CullModeFlags::FRONT_AND_BACK,
    };
    let depth_func = match draw.depth_func as i32 {
        f @ 0x200..=0x207 => vk::CompareOp::from_raw(f - 0x200),
        f @ 1..=8 => vk::CompareOp::from_raw(f - 1),
        _ => vk::CompareOp::ALWAYS,
    };
    RasterState {
        viewport: vk::Viewport {
            x: ox - sx,
            y: oy - sy,
            width: 2.0 * sx,
            height: 2.0 * sy,
            min_depth: draw.viewport.min_depth,
            max_depth: draw.viewport.max_depth,
        },
        scissor,
        cull_face,
        // Facing is judged on the target's rows as Vulkan does (NVK passes Vulkan's front face through with an upper-left
        // origin), which is GL's y-up judgement under a lower-left origin, so only FLIP_Y turns it (Ryujinx TriangleRastFlip).
        front_face: if (draw.front_face == 0x900) != draw.flip_y { vk::FrontFace::CLOCKWISE } else { vk::FrontFace::COUNTER_CLOCKWISE },
        topology,
        depth_test: draw.depth_test_enabled && draw.depth_target.is_some(),
        depth_write: draw.depth_write_enabled && draw.depth_target.is_some(),
        depth_func,
        blend_const: draw.blend_constant,
        blend,
    }
}

impl<'a> Gpu<'a> {
    /// Keeps translated programs and the pipeline cache under `cache`'s root and title when given.
    pub fn new(state: &'static VkState, ram_bytes: usize, cache: Option<(&Path, &str)>) -> Result<Self, VkError> {
        let disk = cache.and_then(|(root, title)| {
            let device = unsafe { state.instance.get_physical_device_properties(state.physical_device) };
            Disk::open(root, title, &device).ok()
        });
        Ok(Self {
            state,
            ram: vec![0; ram_bytes],
            space: PageMap::default(),
            memory: MemoryTracker::default(),
            pusher: Pusher::default(),
            modules: HashMap::new(),
            pipelines: PipelineBuilder::new(state, PipelinePolicy::Wait, disk)?,
            kernels: HashMap::new(),
            translated: 0,
            support: None,
            targets: HashMap::new(),
            mirrors: HashMap::new(),
            textures: HashMap::new(),
            pending_clear: None,
            pending_depth_clear: None,
            pixels: Vec::new(),
            started: Instant::now(),
        })
    }

    pub fn set_pipeline_policy(&mut self, policy: PipelinePolicy) {
        self.pipelines.policy = policy;
    }

    pub fn flush_caches(&self) -> std::io::Result<()> {
        self.pipelines.flush()
    }

    /// A CPU write of guest memory at `va`.
    pub fn write(&mut self, va: u64, bytes: &[u8]) -> Result<(), MemoryError> {
        self.store(va, bytes, Writer::Cpu)
    }

    pub fn read(&self, va: u64, out: &mut [u8]) -> Result<(), MemoryError> {
        gather(&self.space, &self.ram, va, out)
    }

    fn store(&mut self, va: u64, bytes: &[u8], writer: Writer) -> Result<(), MemoryError> {
        let ranges: Vec<_> = self.space.resolve(va, bytes.len() as u64)?.collect();
        self.memory.store(&mut self.ram, &ranges, bytes, writer);
        Ok(())
    }

    /// Runs GPFIFO entries to completion: every draw has landed in guest memory when this returns.
    pub fn submit(&mut self, entries: &[[u8; 8]]) -> Result<(), String> {
        let (space, ram) = (&self.space, &self.ram);
        let events = self.pusher.submit(entries.iter().copied(), |va, len| {
            let mut pieces = space.resolve(va, len as u64).map_err(named)?;
            match (pieces.next(), pieces.next()) {
                (Some(range), None) => Ok(&ram[range.start as usize..range.end as usize]),
                _ => Err(format!("pushbuffer at {va:#x} spans discontiguous pages")),
            }
        })?;
        for event in events {
            match event {
                Event::Draw(draw) => self.draw(&draw)?,
                Event::Dispatch(dispatch) => self.dispatch(&dispatch)?,
                Event::Report { addr, payload, one_word: true } | Event::SemaphoreRelease { addr, payload } => {
                    self.store(addr, &payload.to_le_bytes(), Writer::Gpu).map_err(named)?
                }
                Event::Report { addr, payload, one_word: false } => {
                    let report = [u64::from(payload), self.started.elapsed().as_nanos() as u64].map(u64::to_le_bytes).concat();
                    self.store(addr, &report, Writer::Gpu).map_err(named)?
                }
                Event::LoadConstantBuffer { addr, value } => self.store(addr, &value.to_le_bytes(), Writer::Gpu).map_err(named)?,
                Event::Clear(clear) if clear.value & !(CLEAR_Z | CLEAR_RGBA) == 0 && matches!(clear.value & CLEAR_RGBA, 0 | CLEAR_RGBA) => {
                    if clear.value & CLEAR_RGBA != 0 {
                        self.pending_clear = Some((clear.target, clear.colour));
                    }
                    if let Some(depth_target) = clear.depth_target.filter(|_| clear.value & CLEAR_Z != 0) {
                        self.pending_depth_clear = Some((depth_target, clear.depth));
                    }
                }
                Event::Clear(clear) => return Err(format!("CLEAR_SURFACE {:#x} is not implemented, only Z and RGBA of target 0", clear.value)),
                Event::SemaphoreAcquire { addr, payload, at_least } => {
                    let mut value = [0; 4];
                    gather(&self.space, &self.ram, addr, &mut value).map_err(named)?;
                    let value = u32::from_le_bytes(value);
                    if !(value == payload || at_least && value.wrapping_sub(payload) as i32 > 0) {
                        return Err(format!("semaphore acquire at {addr:#x} waits for {payload:#x} but memory holds {value:#x}, and nothing else runs to release it"));
                    }
                }
            }
        }
        match self.pending_clear.take() {
            Some((target, colour)) => self.fill(target, colour),
            None => Ok(()),
        }
    }

    /// Writes a clear no draw consumed into the target's guest memory, where the target's next draw loads it from.
    fn fill(&mut self, target: ColourTarget, colour: [f32; 4]) -> Result<(), String> {
        let extent = colour_extent(target)?;
        let texel = colour.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8);
        let bytes = texel.repeat((extent.width * extent.height) as usize);
        self.store(target.address, &bytes, Writer::Gpu).map_err(named)
    }

    /// Presents a pitch-linear RGBA8 surface from guest memory.
    pub fn present(&mut self, va: u64, width: u32, height: u32) -> Result<(), MemoryError> {
        self.pixels.resize(width as usize * height as usize * 4, 0);
        gather(&self.space, &self.ram, va, &mut self.pixels)?;
        crate::display::present(width, height, &self.pixels);
        Ok(())
    }

    // Nothing records a program's length, but compiled programs end with a branch to itself after their last EXIT.
    fn code(&self, address: u64) -> Result<Vec<u128>, String> {
        let mut code = Vec::new();
        while !code.last().is_some_and(|&inst: &u128| {
            let pc = code.len() as u32 - 1;
            inst >> 12 & 0xf == 7 && branch_target(pc, inst) == Some(pc)
        }) {
            let mut word = [0; 16];
            gather(&self.space, &self.ram, address + 16 * code.len() as u64, &mut word).map_err(named)?;
            code.push(u128::from_le_bytes(word));
        }
        Ok(code)
    }

    /// Bit n of `integer[0]` or `[1]` is set when vertex attribute n is fetched as an unsigned or signed integer.
    fn module(&mut self, program: ShaderProgram, shader_type: u32, integer: [u32; 2]) -> Result<u64, String> {
        let mut sph = [0; SPH_BYTES];
        gather(&self.space, &self.ram, program.address, &mut sph).map_err(named)?;
        let code = self.code(program.address + SPH_BYTES as u64)?;
        let header = ProgramHeader::new(&sph);
        let [uint, sint] = integer.map(|mask| mask & header.inputs);
        let key = hash((sph, &code, uint, sint));
        if self.modules.contains_key(&key) {
            return Ok(key);
        }
        if header.version != SPH_VERSION {
            return Err(format!("shader program at {:#x} has SPH version {}, only SPHV4 is implemented", program.address, header.version));
        }
        let stage = match (header.sph_type, header.shader_type) {
            (1, SHADER_TYPE_VERTEX) if shader_type == SHADER_TYPE_VERTEX => Stage::Vertex { uint, sint },
            (2, SHADER_TYPE_PIXEL) if shader_type == SHADER_TYPE_PIXEL && header.colour_targets == 0xf => {
                Stage::Fragment { flat: header.flat, screen_linear: header.screen_linear, depth: header.depth }
            }
            _ => return Err(format!("shader program at {:#x} with {header:?} is not implemented for shader type {shader_type}", program.address)),
        };
        let (words, resources) = self.translate(key, stage, &code);
        self.modules.insert(key, Module { header, words, resources });
        Ok(key)
    }

    fn translate(&mut self, key: u64, stage: Stage, code: &[u128]) -> (Vec<u32>, Vec<ResourceKind>) {
        let disk = self.pipelines.disk.as_ref();
        if let Some(saved) = disk.and_then(|disk| disk.spirv(key)) {
            return saved;
        }
        self.translated += 1;
        let mut resources = Vec::new();
        let words = emit_module(Some(self.state), stage, |decoder| {
            decoder.translate_program(code);
            resources = decoder.resources.iter().map(|r| r.kind).collect();
        })
        .words();
        if let Some(disk) = disk {
            let _ = disk.store_spirv(key, &words, &resources);
        }
        (words, resources)
    }

    /// Runs a QMD launch over one global window spanning all mapped guest memory, then stores the pages it changed.
    fn dispatch(&mut self, dispatch: &DispatchSnapshot) -> Result<(), String> {
        let code = self.code(dispatch.program_address)?;
        let key = hash((&code, dispatch.block));
        let mut words = Vec::new();
        if !self.kernels.contains_key(&key) {
            let resources;
            (words, resources) = self.translate(key, Stage::Compute { local_size: dispatch.block }, &code);
            let (ubo, ssbo) = (vk::DescriptorType::UNIFORM_BUFFER, vk::DescriptorType::STORAGE_BUFFER);
            let mut bindings = Vec::new();
            for kind in resources {
                let binding = match kind {
                    ResourceKind::Support => (SUPPORT_BINDING, ubo),
                    ResourceKind::Cbuf(slot) => (CBUF_BINDING + slot, ubo),
                    ResourceKind::Global | ResourceKind::Global64 => (GLOBAL_BINDING, ssbo),
                    ResourceKind::BuiltIn(_) | ResourceKind::Local | ResourceKind::Shared | ResourceKind::Shared64 | ResourceKind::Warp | ResourceKind::Schedule => continue,
                    kind => return Err(format!("{kind:?} in a compute dispatch is not implemented")),
                };
                if !bindings.contains(&binding) {
                    bindings.push(binding);
                }
            }
            self.kernels.insert(key, bindings);
        }
        let (state, pipeline_key) = (self.state, PipelineKey::Dispatch(key));
        self.pipelines
            .get_or_request(&pipeline_key, || {
                let bindings = self.kernels[&key].clone();
                move || ComputePipeline::new(state, &words, &bindings).map(Pipeline::Dispatch)
            })
            .map_err(named)?;
        let span = self.space.span().ok_or("a compute dispatch needs mapped guest memory")?;
        if span.end - span.start > 1 << 32 {
            return Err(format!("mapped guest memory {span:#x?} spans more than the 4 GiB a global window addresses"));
        }
        let mut buffers = Vec::new();
        for (binding, _) in self.kernels[&key].clone() {
            let ranges = match binding {
                GLOBAL_BINDING => self.mirror(span.start, span.end - span.start).map_err(|e| format!("the global window over all mapped guest memory: {e}"))?,
                SUPPORT_BINDING => continue,
                slot => {
                    let slot = slot - CBUF_BINDING;
                    let buffer = dispatch.constant_buffers[slot as usize].ok_or_else(|| format!("QMD constant buffer {slot} is not valid"))?;
                    self.mirror(buffer.address, buffer.size.into())?
                }
            };
            buffers.push((binding, ranges));
        }
        if self.support.is_none() {
            self.support = Some(HostBuffer::new(self.state, 16).map_err(named)?);
        }
        let support = self.support.as_mut().unwrap();
        support.write(&span.start.to_le_bytes()).map_err(named)?;
        let Lookup::Ready(Pipeline::Dispatch(pipeline)) = self.pipelines.wait(&pipeline_key).map_err(named)? else { unreachable!() };
        let bindings = &self.kernels[&key];
        let mut bound: Vec<_> = buffers.iter().map(|(binding, ranges)| (*binding, &self.mirrors[ranges].1.buffer)).collect();
        if bindings.iter().any(|&(binding, _)| binding == SUPPORT_BINDING) {
            bound.push((SUPPORT_BINDING, support));
        }
        pipeline.descriptors.bind(&bound);
        self.state.dispatch(pipeline, dispatch.grid).map_err(named)?;
        if let Some((_, window)) = buffers.iter().find(|&&(binding, _)| binding == GLOBAL_BINDING) {
            let (seen, mapping) = self.mirrors.get_mut(window).unwrap();
            self.pixels.resize((span.end - span.start) as usize, 0);
            mapping.buffer.read(&mut self.pixels).map_err(named)?;
            let mut bytes = &self.pixels[..];
            for range in window {
                for start in (range.start..range.end).step_by(PAGE as usize) {
                    let page = start..range.end.min(start + PAGE);
                    let (written, rest) = bytes.split_at((page.end - page.start) as usize);
                    if self.ram[page.start as usize..page.end as usize] != *written {
                        self.memory.store(&mut self.ram, &[page], written, Writer::Gpu);
                    }
                    bytes = rest;
                }
            }
            *seen = generation(&self.memory, window);
        }
        for release in dispatch.release_semaphores.iter().flatten() {
            self.store(release.address, &release.payload.to_le_bytes(), Writer::Gpu).map_err(named)?;
        }
        Ok(())
    }

    fn depth_format(&self, target: DepthTarget) -> Result<vk::Format, String> {
        // SET_ZT_FORMAT ZF32, Z24S8 and S8Z24 (clc797.h); the two 24-bit orders are the same Vulkan format.
        let format = match target.format {
            0x0a => vk::Format::D32_SFLOAT,
            0x14 | 0x16 => vk::Format::D24_UNORM_S8_UINT,
            format => return Err(format!("depth target format {format:#x} is not implemented, only ZF32, Z24S8 and S8Z24")),
        };
        let properties = unsafe { self.state.instance.get_physical_device_format_properties(self.state.physical_device, format) };
        match properties.optimal_tiling_features.contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT) {
            true => Ok(format),
            false => Err(format!("depth target format {:#x} needs {format:?}, which the device cannot render to", target.format)),
        }
    }

    /// The image for texture `handle` in the draw's pools, uploaded again whenever its guest pages change.
    fn texture(&mut self, draw: &DrawSnapshot, handle: u32) -> Result<TextureKey, String> {
        let entry = |pool: DescriptorPool, index: u32, name: &str| -> Result<[u8; 32], String> {
            if index > pool.maximum_index {
                return Err(format!("{name} {index} is past its pool's maximum index {}", pool.maximum_index));
            }
            let mut entry = [0; 32];
            gather(&self.space, &self.ram, pool.address + 32 * u64::from(index), &mut entry).map_err(named)?;
            Ok(entry)
        };
        let sampler_index = if draw.sampler_via_header { handle & 0xf_ffff } else { handle >> 20 };
        let (tic, tsc) = (entry(draw.texture_pool, handle & 0xf_ffff, "texture header")?, entry(draw.sampler_pool, sampler_index, "sampler")?);
        let pool = TexturePool { tic: &tic, tsc: &tsc };
        let (header, sampler) = (pool.texture(0)?, pool.sampler(0)?);
        let len = header.guest_bytes() as u64;
        let key = (self.space.resolve(header.address, len).map_err(named)?.collect::<Vec<_>>(), tic, tsc);
        let current = generation(&self.memory, &key.0);
        if self.textures.get(&key).is_none_or(|(seen, _)| *seen != current) {
            let mut texels = vec![0; len as usize];
            gather(&self.space, &self.ram, header.address, &mut texels).map_err(named)?;
            let image = HostImage::new(self.state, &header, &texels, &sampler).map_err(named)?;
            self.textures.insert(key.clone(), (current, image));
        }
        Ok(key)
    }

    /// Mirrors guest memory at `va` into a host buffer, rewritten when any of its pages changed, keyed by its ranges.
    fn mirror(&mut self, va: u64, len: u64) -> Result<Vec<PaRange>, String> {
        let ranges: Vec<_> = self.space.resolve(va, len).map_err(named)?.collect();
        let generation = generation(&self.memory, &ranges);
        match self.mirrors.get_mut(&ranges) {
            Some((seen, _)) if *seen == generation => {}
            Some((seen, mapping)) => {
                mapping.refresh(&self.ram).map_err(named)?;
                *seen = generation;
            }
            None => {
                let mapping = self.memory.upload(self.state, &self.ram, ranges.clone()).map_err(named)?;
                self.mirrors.insert(ranges.clone(), (generation, mapping));
            }
        }
        Ok(ranges)
    }

    fn draw(&mut self, draw: &DrawSnapshot) -> Result<(), String> {
        if let Some(i) = [0, 2, 3, 4].into_iter().find(|&i| draw.shader_programs[i].enabled) {
            return Err(format!("shader pipeline {i} is not implemented"));
        }
        let integer = [UINT, SINT].map(|kind| (0..32).filter(|&i| draw.vertex_attributes[i].kind == kind).fold(0, |mask, i| mask | 1 << i));
        let shaders = [
            self.module(draw.shader_programs[VERTEX_PIPELINE], SHADER_TYPE_VERTEX, integer)?,
            self.module(draw.shader_programs[PIXEL_PIPELINE], SHADER_TYPE_PIXEL, [0; 2])?,
        ];
        let [vs, fs] = shaders.map(|key| &self.modules[&key]);
        if fs.header.inputs & !vs.header.outputs != 0 {
            return Err(format!("pixel shader reads attributes {:#x} the vertex shader does not write", fs.header.inputs & !vs.header.outputs));
        }

        let (mut streams, mut attributes): (Vec<(u32, u32, vk::VertexInputRate)>, _) = (Vec::new(), Vec::new());
        for location in (0..32).filter(|location| vs.header.inputs >> location & 1 != 0) {
            let attribute = draw.vertex_attributes[location as usize];
            let binding = match streams.iter().position(|&(stream, ..)| stream == u32::from(attribute.stream)) {
                Some(binding) => binding,
                None => {
                    let stream = draw.vertex_streams[attribute.stream as usize];
                    if !stream.enabled {
                        return Err(format!("attribute {location} reads disabled vertex stream {}", attribute.stream));
                    }
                    let rate = match (stream.instanced, stream.frequency) {
                        (false, _) => vk::VertexInputRate::VERTEX,
                        (true, 1) => vk::VertexInputRate::INSTANCE,
                        (true, frequency) => return Err(format!("vertex stream {} advances every {frequency} instances, only every one is implemented", attribute.stream)),
                    };
                    streams.push((attribute.stream.into(), stream.stride.into(), rate));
                    streams.len() - 1
                }
            };
            attributes.push((location, binding as u32, vertex_format(attribute)?, attribute.offset.into()));
        }
        let mut cbufs: [Vec<(u32, ConstantBuffer)>; 2] = Default::default();
        let mut textures = Vec::new();
        for (stage, (module, pipeline)) in [(vs, VERTEX_PIPELINE), (fs, PIXEL_PIPELINE)].into_iter().enumerate() {
            let group = draw.shader_programs[pipeline].binding_group as usize;
            let bound = |slot: u32| draw.constant_buffers[group][slot as usize].ok_or_else(|| format!("constant buffer slot {slot} of binding group {group} is not bound"));
            let mut images = TEXTURE_BINDING..;
            for &kind in &module.resources {
                match kind {
                    ResourceKind::Cbuf(slot) => cbufs[stage].push((CBUF_BINDING + slot, bound(slot)?)),
                    ResourceKind::Texture2D { cbuf_bank, byte_offset } if pipeline == PIXEL_PIPELINE => {
                        let mut handle = [0; 4];
                        gather(&self.space, &self.ram, bound(cbuf_bank)?.address + u64::from(byte_offset), &mut handle).map_err(named)?;
                        textures.push((images.next().unwrap(), u32::from_le_bytes(handle)));
                    }
                    ResourceKind::BuiltIn(_) | ResourceKind::VertexInput { .. } | ResourceKind::Varying { .. } | ResourceKind::FragmentOutput { .. } => {}
                    kind => return Err(format!("{kind:?} in shader pipeline {pipeline} is not implemented")),
                }
            }
        }

        let ct = draw.colour_target;
        let extent = colour_extent(ct)?;
        let topology = topology(draw.topology)?;
        let pipeline_depth = draw.depth_target.map(|target| self.depth_format(target)).transpose()?;
        let key = PipelineKey::Draw {
            shaders,
            streams: streams.iter().map(|&(.., rate)| rate).collect(),
            attributes,
            topology,
            blend: self.state.extended_dynamic_state3.is_none().then_some((draw.blend, draw.colour_write_mask)),
            depth: pipeline_depth,
        };

        let vertex_ranges = streams
            .iter()
            .map(|&(stream, ..)| self.mirror(draw.vertex_streams[stream as usize].address, draw.vertex_streams[stream as usize].size))
            .collect::<Result<Vec<_>, _>>()?;
        let index_buffer = if draw.indexed {
            let ib = draw.index_buffer;
            let index_type = match ib.element_size {
                1 if self.state.index_type_uint8 => vk::IndexType::UINT8_KHR,
                1 => return Err("u8 index buffers need VK_KHR_index_type_uint8".into()),
                2 => vk::IndexType::UINT16,
                _ => vk::IndexType::UINT32,
            };
            Some((self.mirror(ib.address, u64::from(ib.element_size) * u64::from(draw.first_vertex + draw.vertex_count))?, index_type))
        } else {
            None
        };
        let mut cbuf_ranges: [Vec<(u32, Vec<PaRange>)>; 2] = Default::default();
        for (ranges, buffers) in cbuf_ranges.iter_mut().zip(&cbufs) {
            for &(binding, buffer) in buffers {
                ranges.push((binding, self.mirror(buffer.address, buffer.size.into())?));
            }
        }
        let textures = textures.into_iter().map(|(binding, handle)| Ok((binding, self.texture(draw, handle)?))).collect::<Result<Vec<_>, String>>()?;

        let PipelineKey::Draw { streams: rates, attributes, .. } = &key else { unreachable!() };
        let (state, blend) = (self.state, blend_state(draw.blend, draw.colour_write_mask)?);
        let lookup = self.pipelines.get_or_request(&key, || {
            let bindings: Vec<_> = (0..).zip(rates).map(|(binding, &input_rate)| vk::VertexInputBindingDescription { binding, stride: 0, input_rate }).collect();
            let attributes: Vec<_> = attributes
                .iter()
                .map(|&(location, binding, format, offset)| vk::VertexInputAttributeDescription { location, binding, format, offset })
                .collect();
            let ubo = vk::DescriptorType::UNIFORM_BUFFER;
            let [vs_bindings, mut fs_bindings] = cbuf_ranges.each_ref().map(|stage| stage.iter().map(|&(binding, _)| (binding, ubo)).collect::<Vec<_>>());
            fs_bindings.extend(textures.iter().map(|&(binding, _)| (binding, vk::DescriptorType::COMBINED_IMAGE_SAMPLER)));
            let [vs, fs] = shaders.map(|key| self.modules[&key].words.clone());
            move || {
                let layout = VertexLayout { bindings: &bindings, attributes: &attributes, topology };
                let target = TargetState { blend, depth: pipeline_depth };
                GraphicsPipeline::new(state, &vs, &fs, [&vs_bindings, &fs_bindings], &layout, &target).map(Pipeline::Draw)
            }
        });
        let Lookup::Ready(Pipeline::Draw(pipeline)) = lookup.map_err(named)? else { return Ok(()) };
        for (descriptors, ranges) in pipeline.descriptors.iter_mut().zip(&cbuf_ranges) {
            descriptors.bind(&ranges.iter().map(|(binding, ranges)| (*binding, &self.mirrors[ranges].1.buffer)).collect::<Vec<_>>());
        }
        pipeline.descriptors[1].bind_images(&textures.iter().map(|(binding, key)| (*binding, &self.textures[key].1)).collect::<Vec<_>>());

        let ranges: Vec<_> = self.space.resolve(ct.address, u64::from(extent.width * extent.height * 4)).map_err(named)?.collect();
        let current = generation(&self.memory, &ranges);
        let clear = self.pending_clear.take_if(|(target, _)| *target == ct).map(|(_, colour)| colour);
        let clear_depth = self.pending_depth_clear.take_if(|(target, _)| Some(*target) == draw.depth_target).map(|(_, depth)| depth);
        let depth = draw.depth_target.map(|target| target.address).zip(pipeline_depth);
        let target_key = TargetKey { ranges: ranges.clone(), format: ct.format, extent: (extent.width, extent.height), depth };
        let target = match self.targets.entry(target_key) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let render_target = RenderTarget::new(self.state, extent.width, extent.height, pipeline_depth).map_err(named)?;
                entry.insert(Target { render_target, generation: u64::MAX, depth_cleared: false })
            }
        };
        let load = clear.is_none() && target.generation != current;
        if load {
            self.pixels.resize((extent.width * extent.height * 4) as usize, 0);
            gather(&self.space, &self.ram, ct.address, &mut self.pixels).map_err(named)?;
            target.render_target.write(&self.pixels).map_err(named)?;
        }
        target.depth_cleared |= clear_depth.is_some();
        if let Some((address, _)) = depth.filter(|_| !target.depth_cleared) {
            return Err(format!("depth target at {address:#x} is drawn before any clear, and its contents only live on the host"));
        }
        let vertex_buffers: Vec<(&HostBuffer, u32)> =
            vertex_ranges.iter().zip(&streams).map(|(ranges, &(_, stride, _))| (&self.mirrors[ranges].1.buffer, stride)).collect();
        let draw_state = DrawState {
            vertex_buffers: &vertex_buffers,
            index_buffer: index_buffer.as_ref().map(|(ranges, index_type)| (&self.mirrors[ranges].1.buffer, *index_type)),
            vertex_count: draw.vertex_count,
            first_vertex: draw.first_vertex,
            first_instance: draw.first_instance,
            instance_count: draw.instance_count,
            clear,
            clear_depth,
            load,
            raster: raster_state(draw, extent, topology, blend),
        };
        self.state.draw(pipeline, &target.render_target, &draw_state).map_err(named)?;
        self.pixels.resize((extent.width * extent.height * 4) as usize, 0);
        target.render_target.read(&mut self.pixels).map_err(named)?;
        self.memory.store(&mut self.ram, &ranges, &self.pixels, Writer::Gpu);
        target.generation = generation(&self.memory, &ranges);
        Ok(())
    }
}
