//! The pbr scenes (demos/scenes/pbr-boombox.md, pbr-flighthelmet.md): a converted glTF asset in guest memory, drawn
//! from a host-written pushbuffer by the NVIDIA driver's SM86 build of demos/shaders/pbr.vert and pbr.frag, one draw per
//! primitive with its material's pointer table.
use super::engine::{Geometry, Gpu};
use super::frame::cache_root;
use super::memory::PAGE;
use super::{VkState, device};
use crate::tests::gpu::engine::{bytes, entry, inc, non_inc};
use crate::tests::gpu_test::inst::bra;
use std::path::Path;
use std::sync::{Mutex, atomic::{AtomicBool, Ordering}};

const BASE: u64 = 0x1000_0000;
const PUSHBUFFER: u64 = BASE;
const FRAME_SEGMENT: u64 = BASE + 0x8000;
const VERTEX_PROGRAM: u64 = BASE + 0x1_0000;
const PIXEL_PROGRAM: u64 = BASE + 0x4_0000;
const VERTEX_TABLE: u64 = BASE + 0x2_0000;
const VERTEX_UNIFORMS: u64 = VERTEX_TABLE + 0x100;
const MATERIALS: u64 = BASE + 0x2_1000;
const MATERIAL_STRIDE: u64 = 0x100;
const MATERIAL_UNIFORMS: u64 = 0x80;
const TIC: u64 = BASE + 0x3_0000;
const TSC: u64 = BASE + 0x3_8000;
const REPORT: u64 = BASE + 0x3_f000;
const VERTICES: u64 = BASE + 0x6_0000;
// The depth target lives on the host only, so its VA needs no pages.
const DEPTH: u64 = 0x2000_0000;
const ALIGN: u64 = 0x1_0000;
const EYE: [f32; 3] = [0.0, 0.35, 3.5];
const TLAS: u64 = BASE + 0x2_f000;
const RAY_CONSTANTS: u64 = TLAS + 0x800;
const IDENTITY: Matrix = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
const ROOM_EYE: [f32; 3] = [0.0, 1.0, 5.5];
const ROOM_MATERIALS: [([u8; 3], f32, f32); 8] = [
    ([28, 31, 38], 0.75, 0.18), ([150, 18, 24], 0.0, 0.72), ([20, 112, 52], 0.0, 0.72),
    ([218, 205, 174], 0.0, 0.72), ([226, 215, 188], 0.0, 0.8), ([205, 218, 232], 1.0, 0.06),
    ([120, 195, 220], 0.2, 0.42),
    ([80, 190, 255], 0.0, 0.25),
];

// The driver's bank 0x1d: TEX handle k at +4k in GLSL declaration order, the binding 1 block's address at +0x18
// (pbr.vert and pbr.frag SASS: `TEX ... 0x1d, 0x4`, `ULDC.64 UR4, c[0x1d][0x18]`).
const POINTER_BANK: u32 = 0x1d;
const UNIFORM_POINTER: usize = 0x18 / 4;
const SLOTS: [&str; 5] = ["baseColor", "normal", "metallicRoughness", "occlusion", "emissive"];

const SET_OBJECT: u32 = 0x0000;
const SET_VERTEX_STREAM_SIZE_A: u32 = 0x0600;
const SET_COLOR_TARGET_A: u32 = 0x0800;
const SET_VIEWPORT_SCALE_X: u32 = 0x0a00;
const SET_VIEWPORT_CLIP_HORIZONTAL: u32 = 0x0c00;
const SET_COLOR_CLEAR_VALUE: u32 = 0x0d80;
const SET_SCISSOR_ENABLE: u32 = 0x0e00;
const SET_ZT_A: u32 = 0x0fe0;
const SET_VERTEX_ATTRIBUTE_A: u32 = 0x1160;
const SET_DEPTH_TEST: u32 = 0x12cc;
const SET_DEPTH_WRITE: u32 = 0x12e8;
const SET_DEPTH_FUNC: u32 = 0x130c;
const SET_BLEND_CONST_RED: u32 = 0x131c;
const SET_BLEND_COLOR_OP: u32 = 0x1340;
const SET_BLEND: u32 = 0x1360;
const SET_ZT_SELECT: u32 = 0x1538;
const SET_TEX_SAMPLER_POOL_A: u32 = 0x155c;
const SET_TEX_HEADER_POOL_A: u32 = 0x1574;
const END: u32 = 0x1614;
const BEGIN: u32 = 0x1618;
const SET_INDEX_BUFFER_A: u32 = 0x17c8;
const SET_INDEX_BUFFER_E: u32 = 0x17d8;
const SET_INDEX_BUFFER_F: u32 = 0x17dc;
const DRAW_INDEX_BUFFER: u32 = 0x17e0;
const CLEAR_SURFACE: u32 = 0x19d0;
const SET_CT_WRITE: u32 = 0x1a00;
const SET_REPORT_SEMAPHORE_A: u32 = 0x1b00;
const SET_VERTEX_STREAM_A_FORMAT: u32 = 0x1c00;
const SET_PIPELINE_SHADER: u32 = 0x2000;
const SET_CONSTANT_BUFFER_SELECTOR_A: u32 = 0x2380;
const LOAD_CONSTANT_BUFFER: u32 = 0x2390;
const BIND_GROUP_CONSTANT_BUFFER: u32 = 0x2410;

/// A minimal reader for the converter's manifest.json: numbers, strings without escapes, arrays and objects.
enum Json {
    Number(f64),
    Text(String),
    List(Vec<Json>),
    Map(Vec<(String, Json)>),
    Other,
}

impl Json {
    fn parse(text: &mut &str) -> Option<Json> {
        *text = text.trim_start();
        let open = text.chars().next()?;
        if let Some(rest) = text.strip_prefix('"') {
            let (value, rest) = rest.split_once('"')?;
            *text = rest;
            return Some(Json::Text(value.into()));
        }
        if !matches!(open, '{' | '[') {
            let end = text.find([',', '}', ']']).unwrap_or(text.len());
            let value = text[..end].trim().parse().map_or(Json::Other, Json::Number);
            *text = &text[end..];
            return Some(value);
        }
        *text = &text[1..];
        let mut items = Vec::new();
        loop {
            *text = text.trim_start().trim_start_matches(',').trim_start();
            if let Some(rest) = text.strip_prefix(['}', ']']) {
                *text = rest;
                break;
            }
            let key = match open {
                '{' => {
                    let Json::Text(key) = Json::parse(text)? else { return None };
                    *text = text.trim_start().strip_prefix(':')?;
                    key
                }
                _ => String::new(),
            };
            items.push((key, Json::parse(text)?));
        }
        Some(if open == '{' { Json::Map(items) } else { Json::List(items.into_iter().map(|(_, value)| value).collect()) })
    }

    fn get(&self, key: &str) -> Result<&Json, String> {
        match self {
            Json::Map(items) => items.iter().find(|(k, _)| k == key).map(|(_, value)| value).ok_or_else(|| format!("manifest has no {key}")),
            _ => Err(format!("manifest {key} is not inside an object")),
        }
    }

    fn list(&self) -> &[Json] {
        if let Json::List(items) = self { items } else { &[] }
    }

    fn number(&self) -> f32 {
        if let Json::Number(n) = self { *n as f32 } else { 0.0 }
    }

    fn text(&self) -> &str {
        if let Json::Text(s) = self { s } else { "" }
    }
}

pub type Matrix = [[f32; 4]; 4];

pub fn multiply(a: Matrix, b: Matrix) -> Matrix {
    std::array::from_fn(|column| std::array::from_fn(|row| (0..4).map(|k| a[k][row] * b[column][k]).sum()))
}

fn split(va: u64) -> [u32; 2] {
    [(va >> 32) as u32, va as u32]
}

// A bindless cbuf handle: the address >> 4 in bits 0..45 and the size >> 4 above (NVK nvk_bindless_cbuf).
fn uniform_handle(va: u64, size: u32) -> [u32; 2] {
    [(va >> 4) as u32, (va >> 36) as u32 | (size >> 4) << 13]
}

fn floats(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn vertex(out: &mut Vec<u8>, position: [f32; 3], normal: [f32; 3]) {
    let tangent = if normal[0].abs() > 0.9 { [0.0, 0.0, 1.0] } else { [1.0, 0.0, 0.0] };
    out.extend(floats(&[position[0], position[1], position[2], normal[0], normal[1], normal[2], tangent[0], tangent[1], tangent[2], 1.0, 0.0, 0.0]));
}

fn quad(vertices: &mut Vec<u8>, indices: &mut Vec<u32>, corners: [[f32; 3]; 4], normal: [f32; 3]) -> [u32; 2] {
    let base = (vertices.len() / 48) as u32;
    for corner in corners { vertex(vertices, corner, normal); }
    let first = indices.len() as u32;
    indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    [first, 6]
}

fn sphere(vertices: &mut Vec<u8>, indices: &mut Vec<u32>, centre: [f32; 3], radius: f32) -> [u32; 2] {
    const RINGS: u32 = 16;
    const SEGMENTS: u32 = 32;
    let base = (vertices.len() / 48) as u32;
    for ring in 0..=RINGS {
        let latitude = std::f32::consts::PI * ring as f32 / RINGS as f32;
        for segment in 0..=SEGMENTS {
            let longitude = std::f32::consts::TAU * segment as f32 / SEGMENTS as f32;
            let normal = [latitude.sin() * longitude.cos(), latitude.cos(), latitude.sin() * longitude.sin()];
            vertex(vertices, std::array::from_fn(|i| centre[i] + normal[i] * radius), normal);
        }
    }
    let first = indices.len() as u32;
    for ring in 0..RINGS { for segment in 0..SEGMENTS {
        let a = base + ring * (SEGMENTS + 1) + segment;
        let b = a + SEGMENTS + 1;
        indices.extend([a, b, a + 1, a + 1, b, b + 1]);
    }}
    [first, indices.len() as u32 - first]
}

fn align(va: u64) -> u64 {
    va.next_multiple_of(ALIGN)
}

pub fn asset_dir(asset: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../demos/assets").join(asset)
}

pub fn supports_raytraced(state: &VkState) -> bool {
    use ash::vk;
    let mut subgroup = vk::PhysicalDeviceSubgroupProperties::default();
    let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut subgroup);
    unsafe { state.instance.get_physical_device_properties2(state.physical_device, &mut properties) };
    state.acceleration_structure.is_some()
        && subgroup.supported_stages.contains(vk::ShaderStageFlags::FRAGMENT)
        && subgroup.supported_operations.contains(vk::SubgroupFeatureFlags::ARITHMETIC | vk::SubgroupFeatureFlags::BALLOT | vk::SubgroupFeatureFlags::SHUFFLE)
}

pub struct Scene<'a> {
    pub gpu: Gpu<'a>,
    pub target: u64,
    width: u32,
    height: u32,
    /// first index, index count and material of each primitive.
    draws: Vec<[u32; 3]>,
    /// each material's blend op, source and destination factors and constant colour, drawn over the depth of
    /// what is already there without writing it.
    blends: Vec<Option<Vec<u32>>>,
    /// guest address of each texture, for scenes that rewrite theirs.
    pub textures: Vec<u64>,
    /// Model space centre and the scale that makes the asset two units across.
    fit: ([f32; 3], f32),
    geometry: Vec<Geometry>,
    traced: bool,
    room: bool,
    material_count: usize,
    model_indices: u32,
    light_first: u32,
    light_y: f32,
    frame: u32,
}

impl<'a> Scene<'a> {
    #[cfg(test)]
    pub(crate) fn replace_programs(&mut self, programs: [&[u8]; 2]) -> Result<(), String> {
        for (va, program) in [VERTEX_PROGRAM, PIXEL_PROGRAM].into_iter().zip(programs) {
            self.gpu.write(va, &[program, &bra(-16).to_le_bytes()].concat()).map_err(|e| format!("{e:?}"))?;
        }
        Ok(())
    }

    /// Lays the asset in demos/assets/`asset` out in guest memory and runs the init segment.
    pub fn new(state: &'static VkState, cache: Option<(&Path, &str)>, asset: &str, width: u32, height: u32) -> Result<Self, String> {
        let room = asset.starts_with("raytraced");
        let traced = asset == "raytraced";
        if traced && !supports_raytraced(state) {
            return Err("ray-traced demo needs ray queries and fragment subgroup arithmetic, ballot and shuffle".into());
        }
        let dir = asset_dir(if room { "boombox" } else { asset });
        Self::with_scene_files(state, cache, &|file| std::fs::read(dir.join(file)).map_err(|e| format!("{}: {e}", dir.join(file).display())), width, height, room, traced)
    }

    /// Like `new`, with the asset's files from `read`.
    pub fn with_files(state: &'static VkState, cache: Option<(&Path, &str)>, read: &dyn Fn(&str) -> Result<Vec<u8>, String>, width: u32, height: u32, raytraced: bool) -> Result<Self, String> {
        Self::with_scene_files(state, cache, read, width, height, raytraced, raytraced)
    }

    fn with_scene_files(state: &'static VkState, cache: Option<(&Path, &str)>, read: &dyn Fn(&str) -> Result<Vec<u8>, String>, width: u32, height: u32, room: bool, traced: bool) -> Result<Self, String> {
        let text = String::from_utf8(read("manifest.json")?).map_err(|e| e.to_string())?;
        let manifest = Json::parse(&mut text.as_str()).ok_or("manifest.json does not parse")?;
        let [mut vertices, mut indices] = [read("vertices.bin")?, read("indices.bin")?];
        let (model_vertices, model_indices) = (vertices.len() / 48, indices.len() / 4);
        let mut room_draws = Vec::new();
        if room {
            let mut room_indices = Vec::new();
            for (corners, normal) in [
                ([[-3.0, -1.0, -3.0], [3.0, -1.0, -3.0], [3.0, -1.0, 3.0], [-3.0, -1.0, 3.0]], [0.0, 1.0, 0.0]),
                ([[-3.0, -1.0, 3.0], [-3.0, -1.0, -3.0], [-3.0, 3.0, -3.0], [-3.0, 3.0, 3.0]], [1.0, 0.0, 0.0]),
                ([[3.0, -1.0, -3.0], [3.0, -1.0, 3.0], [3.0, 3.0, 3.0], [3.0, 3.0, -3.0]], [-1.0, 0.0, 0.0]),
                ([[-3.0, -1.0, -3.0], [3.0, -1.0, -3.0], [3.0, 3.0, -3.0], [-3.0, 3.0, -3.0]], [0.0, 0.0, 1.0]),
                ([[-3.0, 3.0, -3.0], [3.0, 3.0, -3.0], [3.0, 3.0, 3.0], [-3.0, 3.0, 3.0]], [0.0, -1.0, 0.0]),
            ] { room_draws.push(quad(&mut vertices, &mut room_indices, corners, normal)); }
            room_draws.push(sphere(&mut vertices, &mut room_indices, [-1.65, -0.35, -0.55], 0.65));
            room_draws.push(sphere(&mut vertices, &mut room_indices, [1.65, -0.35, -0.45], 0.65));
            room_draws.push(sphere(&mut vertices, &mut room_indices, [0.0; 3], 0.12));
            indices.extend(bytes(&room_indices));
        }
        let textures = manifest.get("textures")?.list();
        let indices_va = align(VERTICES + vertices.len() as u64);
        let mut texture_va = vec![align(indices_va + indices.len() as u64)];
        for texture in textures {
            texture_va.push(align(texture_va.last().unwrap() + texture.get("bytes")?.number() as u64));
        }
        let room_textures = texture_va.pop().unwrap();
        let target = room_textures + if room { ALIGN } else { 0 };
        let end = (target + u64::from(width * height * 4)).next_multiple_of(PAGE);
        let mut gpu = Gpu::new(state, (end - BASE) as usize, cache, *super::SPEED_HACKS.lock().unwrap()).map_err(|e| format!("{e:?}"))?;
        gpu.space.map(BASE, 0, end - BASE);
        let named = |e| format!("{e:?}");
        for (va, data) in [(VERTICES, &vertices), (indices_va, &indices)] {
            gpu.write(va, data).map_err(named)?;
        }

        const A8B8G8R8_UNORM: u32 = 8 | 0o2222 << 7 | 0o5432 << 19;
        const BLOCK_LINEAR: u32 = 3 << 21;
        const TWO_D: u32 = 1 << 23;
        const NORMALIZED: u32 = 1 << 31;
        let mut tic = Vec::new();
        for (texture, &va) in textures.iter().zip(&texture_va) {
            gpu.write(va, &read(texture.get("file")?.text())?).map_err(named)?;
            let field = |key| texture.get(key).map(|value| value.number() as u32);
            let [width, height, mips, gob_height_log2] = [field("width")?, field("height")?, field("mips")?, field("gob_height_log2")?];
            let [hi, lo] = split(va);
            tic.extend([A8B8G8R8_UNORM, lo, hi | BLOCK_LINEAR, gob_height_log2 << 3 | (mips - 1) << 28, (width - 1) | TWO_D, (height - 1) | NORMALIZED, 0, 0]);
        }
        if room {
            for (i, pixel) in ROOM_MATERIALS.iter().flat_map(|(base, metallic, roughness)| [
                [base[0], base[1], base[2], 255], [128, 128, 255, 255], [0, (roughness * 255.0) as u8, (metallic * 255.0) as u8, 255], [255; 4], [base[0], base[1], base[2], 255],
            ]).enumerate() {
                let va = room_textures + i as u64 * 512;
                gpu.write(va, &pixel).map_err(named)?;
                let [hi, lo] = split(va);
                tic.extend([A8B8G8R8_UNORM, lo, hi | BLOCK_LINEAR, 0, TWO_D, NORMALIZED, 0, 0]);
            }
        }
        gpu.write(TIC, &bytes(&tic)).map_err(named)?;
        // Repeat, trilinear, 16x anisotropic, LOD clamped to 0..15.
        const MAX_ANISOTROPY_16: u32 = 7 << 20;
        const TRILINEAR: u32 = 2 | 2 << 4 | 3 << 6;
        gpu.write(TSC, &bytes(&[MAX_ANISOTROPY_16, TRILINEAR, 0xf00 << 12, 0, 0, 0, 0, 0])).map_err(named)?;

        let light = [0.4f32, 0.8, 0.45];
        let light = light.map(|c| c / light.iter().map(|c| c * c).sum::<f32>().sqrt());
        let texture_index = |file: &str| textures.iter().position(|t| t.get("file").is_ok_and(|f| f.text() == file)).map(|i| i as u32);
        let materials = manifest.get("materials")?.list();
        let mut blends: Vec<Option<Vec<u32>>> = materials.iter().map(|m| m.get("blend").ok().map(|b| b.list().iter().map(|v| v.number() as u32).collect())).collect();
        let uniform_pointer = if traced { 0x28 / 4 } else { UNIFORM_POINTER };
        let eye = if room { ROOM_EYE } else { EYE };
        if room { blends.extend(std::iter::repeat_n(None, ROOM_MATERIALS.len())); }
        for (m, material) in materials.iter().enumerate() {
            let maps = material.get("textures")?;
            let base = maps.get("baseColor")?.text();
            let mut table = [0; 12];
            for (k, slot) in SLOTS.into_iter().enumerate() {
                let file = maps.get(slot).map_or(base, Json::text);
                table[k] = texture_index(file).ok_or_else(|| format!("material {m} {slot} {file} is not in textures"))?;
            }
            let va = MATERIALS + MATERIAL_STRIDE * m as u64;
            [table[uniform_pointer], table[uniform_pointer + 1]] = uniform_handle(va + MATERIAL_UNIFORMS, if traced { 128 } else { 112 });
            if traced { [table[6], table[7]] = [TLAS as u32, (TLAS >> 32) as u32]; }
            let numbers = |key| material.get(key).map(|value| value.list().iter().map(Json::number).collect::<Vec<_>>());
            let [base_factor, mut emissive] = [numbers("baseColorFactor")?, numbers("emissiveFactor")?];
            emissive.resize(4, 0.0);
            let scalar = |key| material.get(key).map(Json::number);
            let factors = [scalar("metallicFactor")?, scalar("roughnessFactor")?, scalar("normalScale")?, scalar("occlusionStrength")?];
            let default_light = [&light[..], &[0.0, 3.0, 2.9, 2.7, 0.0, 0.6, 0.62, 0.66, 0.0]].concat();
            let lights = numbers("light").unwrap_or(default_light);
            let mut uniforms = [&lights[..], &eye, &[1.0], &base_factor, &emissive, &factors].concat();
            if traced { uniforms.extend([0.35, 0.45, 0.6, 0.002]); }
            gpu.write(va, &bytes(&table)).map_err(named)?;
            gpu.write(va + MATERIAL_UNIFORMS, &floats(&uniforms)).map_err(named)?;
        }
        if room {
            for (i, (_, metallic, roughness)) in ROOM_MATERIALS.iter().enumerate() {
                let va = MATERIALS + MATERIAL_STRIDE * (materials.len() + i) as u64;
                let t = textures.len() as u32 + i as u32 * 5;
                let mut table = [t, t + 1, t + 2, t + 3, t + 4, 0, 0, 0, 0, 0, 0, 0];
                if traced { [table[6], table[7]] = [TLAS as u32, (TLAS >> 32) as u32]; }
                [table[uniform_pointer], table[uniform_pointer + 1]] = uniform_handle(va + MATERIAL_UNIFORMS, if traced { 128 } else { 112 });
                let emissive = if i + 1 == ROOM_MATERIALS.len() { [2.0, 4.0, 8.0, 0.0] } else { [0.0; 4] };
                let mut uniforms = [&light[..], &[0.0, 3.0, 2.9, 2.7, 0.0, 0.18, 0.18, 0.2, 0.0], &eye, &[1.0], &[1.0; 4], &emissive, &[*metallic, *roughness, 1.0, 1.0]].concat();
                if traced { uniforms.extend([0.65, 0.75, 0.9, 0.002]); }
                gpu.write(va, &bytes(&table)).map_err(named)?;
                gpu.write(va + MATERIAL_UNIFORMS, &floats(&uniforms)).map_err(named)?;
            }
            if traced { gpu.write(RAY_CONSTANTS, &0x20u32.to_le_bytes()).map_err(named)?; }
        }
        let mut vertex_table = [0; 12];
        [vertex_table[uniform_pointer], vertex_table[uniform_pointer + 1]] = uniform_handle(VERTEX_UNIFORMS, 128);
        gpu.write(VERTEX_TABLE, &bytes(&vertex_table)).map_err(named)?;
        // The rig's programs stop at their last EXIT; the self-branch a compiled program ends with marks the end.
        let programs: [&[u8]; 2] = if traced {
            let pixel: &[u8] = if SHOWCASE.load(Ordering::Relaxed) { include_bytes!("../../tests/fixtures/driver-shaders/rt.frag.bin") } else { include_bytes!("../../tests/fixtures/driver-shaders/rt-realtime.frag.bin") };
            [include_bytes!("../../tests/fixtures/driver-shaders/rt.vert.bin"), pixel]
        } else {
            [include_bytes!("../../tests/fixtures/driver-shaders/pbr.vert.bin"), include_bytes!("../../tests/fixtures/driver-shaders/pbr.frag.bin")]
        };
        for (va, program) in [VERTEX_PROGRAM, PIXEL_PROGRAM].into_iter().zip(programs) {
            gpu.write(va, &[program, &bra(-16).to_le_bytes()].concat()).map_err(named)?;
        }

        let mut draws = manifest
            .get("primitives")?
            .list()
            .iter()
            .map(|p| Ok([p.get("first_index")?.number() as u32, p.get("index_count")?.number() as u32, p.get("material")?.number() as u32]))
            .collect::<Result<Vec<_>, String>>()?;
        let light_count = room_draws.last().map_or(0, |draw| draw[1]);
        let light_first = room_draws.last().map_or(0, |draw| model_indices as u32 + draw[0]);
        if room { draws.extend(room_draws.into_iter().enumerate().map(|(material, [first, count])| [model_indices as u32 + first, count, (materials.len() + material) as u32])); }
        let positions: Vec<[f32; 3]> = vertices[..model_vertices * 48].chunks(48).map(|v| std::array::from_fn(|i| f32::from_le_bytes(v[4 * i..4 * i + 4].try_into().unwrap()))).collect();
        let [low, high] = [f32::min, f32::max].map(|pick| std::array::from_fn::<f32, 3, _>(|i| positions.iter().map(|p| p[i]).reduce(pick).unwrap_or(0.0)));
        let extent = (0..3).map(|i| high[i] - low[i]).fold(0.0, f32::max);
        let mut fit = (std::array::from_fn(|i| (low[i] + high[i]) / 2.0), 2.0 / extent);
        if room { fit.0[1] = low[1] + 1.0 / fit.1; }

        let mut words = Vec::new();
        let [vs, fs, vertex_table, ct, depth, vb, ib, tic_pool, tsc_pool] = [VERTEX_PROGRAM, PIXEL_PROGRAM, VERTEX_TABLE, target, DEPTH, VERTICES, indices_va, TIC, TSC].map(split);
        const PITCH: u32 = 0x1000;
        const ZF32: u32 = 0x0a;
        const OGL_LESS: u32 = 0x201;
        const FOUR_BYTES: u32 = 2;
        const FLOAT: u32 = 7 << 27;
        let (x, y) = (width as f32 / 2.0, height as f32 / 2.0);
        inc(SET_OBJECT, &[0xc797], &mut words);
        inc(SET_COLOR_TARGET_A, &[ct[0], ct[1], width * 4, height, 0xd5, PITCH], &mut words);
        inc(SET_CT_WRITE, &[0x1111], &mut words);
        inc(SET_ZT_A, &[depth[0], depth[1], ZF32], &mut words);
        inc(SET_ZT_SELECT, &[1], &mut words);
        inc(SET_VIEWPORT_SCALE_X, &[x, y, 1.0, x, y, 0.0].map(f32::to_bits), &mut words);
        inc(SET_VIEWPORT_CLIP_HORIZONTAL, &[width << 16, height << 16, 0, 1.0f32.to_bits()], &mut words);
        inc(SET_SCISSOR_ENABLE, &[1, width << 16, height << 16], &mut words);
        // SET_COLOR_CLEAR_VALUE is followed by SET_Z_CLEAR_VALUE.
        inc(SET_COLOR_CLEAR_VALUE, &[0.16f32, 0.17, 0.2, 1.0, 1.0].map(f32::to_bits), &mut words);
        inc(SET_DEPTH_TEST, &[1], &mut words);
        inc(SET_DEPTH_WRITE, &[1], &mut words);
        inc(SET_DEPTH_FUNC, &[OGL_LESS], &mut words);
        inc(SET_VERTEX_STREAM_A_FORMAT, &[0x1000 | 48, vb[0], vb[1]], &mut words);
        inc(SET_VERTEX_STREAM_SIZE_A, &split(vertices.len() as u64), &mut words);
        // position, normal and tangent at 0, 12 and 24 as R32_G32_B32, R32_G32_B32 and R32_G32_B32_A32, uv at 40 as R32_G32.
        inc(SET_VERTEX_ATTRIBUTE_A, &[0x02 << 21 | FLOAT, 12 << 7 | 0x02 << 21 | FLOAT, 24 << 7 | 0x01 << 21 | FLOAT, 40 << 7 | 0x04 << 21 | FLOAT], &mut words);
        inc(SET_INDEX_BUFFER_A, &ib, &mut words);
        inc(SET_INDEX_BUFFER_E, &[FOUR_BYTES], &mut words);
        inc(SET_PIPELINE_SHADER + 64, &[0x11], &mut words);
        inc(SET_PIPELINE_SHADER + 64 + 0x10, &[0, vs[0], vs[1]], &mut words);
        inc(SET_PIPELINE_SHADER + 5 * 64, &[0x51], &mut words);
        inc(SET_PIPELINE_SHADER + 5 * 64 + 0x10, &[4, fs[0], fs[1]], &mut words);
        inc(SET_CONSTANT_BUFFER_SELECTOR_A, &[48, vertex_table[0], vertex_table[1]], &mut words);
        inc(BIND_GROUP_CONSTANT_BUFFER, &[POINTER_BANK << 4 | 1], &mut words);
        inc(SET_TEX_HEADER_POOL_A, &[tic_pool[0], tic_pool[1], (tic.len() / 8 - 1) as u32], &mut words);
        inc(SET_TEX_SAMPLER_POOL_A, &[tsc_pool[0], tsc_pool[1], 0], &mut words);
        gpu.write(PUSHBUFFER, &bytes(&words)).map_err(named)?;
        gpu.submit(&[entry(PUSHBUFFER, words.len())])?;
        let geometry = if traced {
            vec![Geometry { vertices: VERTICES, stride: 48, vertex_count: model_vertices as u32, indices: indices_va, index_count: model_indices as u32, transform: [[0.0; 4]; 3], opaque: true },
                Geometry { vertices: VERTICES, stride: 48, vertex_count: (vertices.len() / 48) as u32, indices: indices_va + model_indices as u64 * 4, index_count: (indices.len() / 4 - model_indices) as u32 - light_count, transform: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]], opaque: true }]
        } else { Vec::new() };
        Ok(Self { gpu, target, width, height, draws, blends, textures: texture_va, fit, geometry, traced, room, material_count: materials.len() + if room { ROOM_MATERIALS.len() } else { 0 }, model_indices: model_indices as u32, light_first, light_y: 0.7, frame: 0 })
    }

    /// Turns the model to `seconds` at 0.8 rad/s, draws every primitive and waits for the frame's report.
    pub fn frame(&mut self, seconds: f32) -> Result<(), String> {
        let ([cx, cy, cz], scale) = self.fit;
        let (sin, cos) = (seconds * 0.8 + if self.room { std::f32::consts::PI } else { 0.0 }).sin_cos();
        let mut model = [[cos * scale, 0.0, -sin * scale, 0.0], [0.0, scale, 0.0, 0.0], [sin * scale, 0.0, cos * scale, 0.0], [0.0; 4]];
        model[3] = [-(cos * cx + sin * cz) * scale, -cy * scale, (sin * cx - cos * cz) * scale, 1.0];
        if self.traced {
            self.geometry[0].transform = std::array::from_fn(|row| std::array::from_fn(|col| model[col][row]));
            self.gpu.register_acceleration_structure(TLAS, self.geometry.clone()).map_err(|e| format!("{e:?}"))?;
        }
        // Vulkan clip space, y down and z in 0..1: 45 degrees vertical, near 0.1, far 20, looking from EYE at the origin.
        let (near, far, focal) = (0.1, 20.0, 1.0 / (22.5f32.to_radians()).tan());
        let projection = [[focal * self.height as f32 / self.width as f32, 0.0, 0.0, 0.0], [0.0, -focal, 0.0, 0.0], [0.0, 0.0, far / (near - far), -1.0], [0.0, 0.0, near * far / (near - far), 0.0]];
        let eye = if self.room { [(seconds * 0.18).sin() * 0.55, ROOM_EYE[1], ROOM_EYE[2]] } else { EYE };
        let length = eye.iter().map(|v| v * v).sum::<f32>().sqrt();
        let back = eye.map(|v| v / length);
        let side_length = (back[0] * back[0] + back[2] * back[2]).sqrt();
        let right = [back[2] / side_length, 0.0, -back[0] / side_length];
        let up = [-back[1] * right[2], back[2] * right[0] - back[0] * right[2], -back[1] * right[0]];
        let mut view = [[right[0], up[0], back[0], 0.0], [right[1], up[1], back[1], 0.0], [right[2], up[2], back[2], 0.0], [0.0; 4]];
        view[3] = [-right.iter().zip(eye).map(|(a, b)| a * b).sum::<f32>(), -up.iter().zip(eye).map(|(a, b)| a * b).sum::<f32>(), -length, 1.0];
        if self.room {
            let (orbit, bob) = ((seconds * 0.45).sin(), (seconds * 1.4).sin());
            let key = [orbit * 1.8, 2.15 + bob * 0.12, 0.4 + 1.25 * (seconds * 0.45).cos()];
            let length = key.iter().map(|v| v * v).sum::<f32>().sqrt();
            let direction = key.map(|v| v / length);
            let light = if self.traced { [key[0], key[1], key[2], 0.32] } else { [direction[0], direction[1], direction[2], 0.0] };
            let cool = [-1.65, 0.7 + 0.25 * bob, -0.55, 0.12];
            let head = [&light[..], &[4.1, 3.5, 3.0, 0.0], if self.traced { &cool[..] } else { &[0.17, 0.18, 0.22, 0.0] }, &eye, &[1.0]].concat();
            for material in 0..self.material_count { self.gpu.write(MATERIALS + MATERIAL_STRIDE * material as u64 + MATERIAL_UNIFORMS, &floats(&head)).map_err(|e| format!("{e:?}"))?; }
            self.light_y = 0.7 + 0.25 * bob;
        }
        self.draw(model, multiply(projection, view), eye)
    }

    /// The guest address of `material`'s uniform block.
    pub fn uniforms(material: u32) -> u64 {
        MATERIALS + MATERIAL_STRIDE * u64::from(material) + MATERIAL_UNIFORMS
    }

    /// Draws every primitive with `model` and `view_proj`, lit as seen from `eye`, and waits for the frame's report.
    pub fn draw(&mut self, model: Matrix, view_proj: Matrix, eye: [f32; 3]) -> Result<(), String> {
        let named = |e| format!("{e:?}");
        for material in 0..self.blends.len() as u32 {
            self.gpu.write(Self::uniforms(material) + 48, &floats(&eye)).map_err(named)?;
        }
        let uniforms: Vec<u32> = [model, view_proj].as_flattened().as_flattened().iter().map(|f| f.to_bits()).collect();

        const CLEAR_Z_AND_RGBA: u32 = 0x3d;
        const TRIANGLES: u32 = 4;
        const ONE_WORD: u32 = 1 << 28;
        let mut words = Vec::new();
        inc(SET_CONSTANT_BUFFER_SELECTOR_A, &[128, split(VERTEX_UNIFORMS)[0], split(VERTEX_UNIFORMS)[1], 0], &mut words);
        non_inc(LOAD_CONSTANT_BUFFER, &uniforms, &mut words);
        inc(CLEAR_SURFACE, &[CLEAR_Z_AND_RGBA], &mut words);
        if self.traced {
            inc(SET_CONSTANT_BUFFER_SELECTOR_A, &[16, split(RAY_CONSTANTS)[0], split(RAY_CONSTANTS)[1]], &mut words);
            inc(BIND_GROUP_CONSTANT_BUFFER + 4 * 32, &[1 << 4 | 1], &mut words);
        }
        let mut blending = None;
        for &[first, count, material] in &self.draws {
            let blend = &self.blends[material as usize];
            if blending != Some(blend) {
                const OGL_LEQUAL: u32 = 0x203;
                const OGL_LESS: u32 = 0x201;
                inc(SET_BLEND, &[u32::from(blend.is_some())], &mut words);
                inc(SET_DEPTH_WRITE, &[u32::from(blend.is_none())], &mut words);
                inc(SET_DEPTH_FUNC, &[if blend.is_some() { OGL_LEQUAL } else { OGL_LESS }], &mut words);
                if let Some(blend) = blend {
                    inc(SET_BLEND_COLOR_OP, &blend[..3], &mut words);
                    inc(SET_BLEND_CONST_RED, &blend[3..].iter().map(|&c| (c as f32 / 255.0).to_bits()).collect::<Vec<_>>(), &mut words);
                }
                blending = Some(blend);
            }
            if self.room && first >= self.model_indices {
                let mut object = IDENTITY;
                if first == self.light_first { object[3] = [-1.65, self.light_y, -0.55, 1.0]; }
                let uniforms: Vec<u32> = [object, view_proj].as_flattened().as_flattened().iter().map(|f| f.to_bits()).collect();
                inc(SET_CONSTANT_BUFFER_SELECTOR_A, &[128, split(VERTEX_UNIFORMS)[0], split(VERTEX_UNIFORMS)[1], 0], &mut words);
                non_inc(LOAD_CONSTANT_BUFFER, &uniforms, &mut words);
            }
            let table = split(MATERIALS + MATERIAL_STRIDE * u64::from(material));
            inc(SET_CONSTANT_BUFFER_SELECTOR_A, &[48, table[0], table[1]], &mut words);
            inc(BIND_GROUP_CONSTANT_BUFFER + 4 * 32, &[POINTER_BANK << 4 | 1], &mut words);
            inc(BEGIN, &[TRIANGLES], &mut words);
            inc(SET_INDEX_BUFFER_F, &[first], &mut words);
            inc(DRAW_INDEX_BUFFER, &[count], &mut words);
            inc(END, &[0], &mut words);
        }
        self.frame += 1;
        let report = split(REPORT);
        inc(SET_REPORT_SEMAPHORE_A, &[report[0], report[1], self.frame, ONE_WORD], &mut words);
        self.gpu.write(FRAME_SEGMENT, &bytes(&words)).map_err(named)?;
        self.gpu.submit(&[entry(FRAME_SEGMENT, words.len())])?;
        let mut released = [0; 4];
        self.gpu.read(REPORT, &mut released).map_err(named)?;
        match u32::from_le_bytes(released) == self.frame {
            true => Ok(()),
            false => Err(format!("frame {} did not release its report", self.frame)),
        }
    }
}

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;

pub(super) static SCENE: Mutex<Option<(&str, Scene<'static>)>> = Mutex::new(None);
static RAY_TRACING: AtomicBool = AtomicBool::new(true);
pub static SHOWCASE: AtomicBool = AtomicBool::new(false);

fn render(asset: &'static str, seconds: f32) -> Result<(), String> {
    let mut slot = SCENE.lock().unwrap();
    if slot.as_ref().is_none_or(|(name, _)| *name != asset) {
        *slot = None;
        *slot = Some((asset, Scene::new(device().map_err(|e| format!("{e:?}"))?, Some((&cache_root(), "demos")), asset, WIDTH, HEIGHT)?));
    }
    let (_, scene) = slot.as_mut().unwrap();
    scene.frame(seconds)?;
    scene.gpu.present(scene.target, WIDTH, HEIGHT).map_err(|e| format!("{e:?}"))
}

pub fn render_boombox_frame(seconds: f32) -> Result<(), String> {
    render("boombox", seconds)
}

pub fn render_raytraced_frame(seconds: f32) -> Result<(), String> {
    render(if RAY_TRACING.load(Ordering::Relaxed) { "raytraced" } else { "raytraced-off" }, seconds)
}

pub fn set_raytracing_enabled(enabled: bool) {
    RAY_TRACING.store(enabled, Ordering::Relaxed);
}

pub fn reset_scene() {
    *SCENE.lock().unwrap() = None;
}

/// Needs demos/assets/fetch.sh run first.
pub fn render_helmet_frame(seconds: f32) -> Result<(), String> {
    render("flighthelmet", seconds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::gpu::harness::vk_state;

    #[test]
    fn raytraced_frame_darkens_occluded_surfaces() {
        let Some(state) = vk_state() else { return };
        if !supports_raytraced(&state) { return eprintln!("skipped: device does not support the ray-traced demo"); }
        if !asset_dir("boombox").exists() { return eprintln!("skipped: run demos/assets/fetch.sh for boombox"); }
        let mut scene = Scene::new(Box::leak(Box::new(state)), None, "raytraced", 320, 180).unwrap();
        scene.frame(0.0).unwrap();
        let mut traced = vec![0; 320 * 180 * 4];
        scene.gpu.read(scene.target, &mut traced).unwrap();
        for geometry in &mut scene.geometry { geometry.transform[0][3] += 1e6; }
        scene.gpu.register_acceleration_structure(TLAS, scene.geometry.clone()).unwrap();
        // Replay the same draw commands with geometry outside every ray's reach.
        let mut commands = vec![0; 0x8000];
        scene.gpu.read(FRAME_SEGMENT, &mut commands).unwrap();
        let report = ((SET_REPORT_SEMAPHORE_A / 4) | (4 << 16) | (1 << 29)).to_le_bytes();
        let words = commands.chunks_exact(4).position(|word| word == report).unwrap() + 5;
        scene.gpu.submit(&[entry(FRAME_SEGMENT, words)]).unwrap();
        let mut unoccluded = vec![0; traced.len()];
        scene.gpu.read(scene.target, &mut unoccluded).unwrap();
        let darker = traced.chunks_exact(4).zip(unoccluded.chunks_exact(4)).filter(|(a, b)| u32::from(a[0]) + 10 < u32::from(b[0])).count();
        assert!(darker > 100, "only {darker} pixels darkened by ray intersections");
    }
}
