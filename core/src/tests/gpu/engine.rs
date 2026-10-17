//! A synthetic guest the way the NVN driver lays one out: shader programs, vertices, a constant buffer, a colour
//! target and a pushbuffer in guest memory mapped to scattered guest-physical pages, drawing one triangle a frame.
use crate::cpu::GuestRam;
use crate::gpu::VkState;
use crate::gpu::engine::Gpu;
use crate::gpu::memory::{MemoryError, PAGE};
use crate::tests::gpu_test::inst::{self, bra, exit, ipa, texture};
use scene::{Addresses, Vertex, entry, frame_state, inc, split};
use scene::threed::*;

const PUSHBUFFER: u64 = 0x1_0000_0000;
const VERTEX_PROGRAM: u64 = 0x1_0010_0000;
const PIXEL_PROGRAM: u64 = 0x1_0010_1000;
const TEXTURED_PROGRAM: u64 = 0x1_0010_2000;
pub const VERTICES: u64 = 0x1_0020_0000;
const CBUF: u64 = 0x1_0030_0000;
pub const REPORT: u64 = 0x1_0040_0000;
pub const TARGET: u64 = 0x1_0100_0000;
/// The TIC entry at +0, the TSC entry at +0x100, the handle word at +0x200 and room for texels from +0x1000.
pub const TEXTURES: u64 = 0x1_0070_0000;
const ADDRESSES: Addresses = Addresses {
    vertex_program: VERTEX_PROGRAM,
    pixel_program: PIXEL_PROGRAM,
    vertices: VERTICES,
    cbuf: CBUF,
    report: REPORT,
    target: TARGET,
};

pub fn bytes(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_le_bytes()).collect()
}

pub fn vertex_bytes(vertices: &[Vertex]) -> Vec<u8> {
    vertices.iter().flat_map(scene::vertex).collect()
}

fn program(header_bits: impl IntoIterator<Item = usize>, code: &[u128]) -> Vec<u8> {
    let mut bytes = vec![0; scene::SPH_BYTES + 16 * code.len()];
    scene::program(&mut bytes, header_bits, code);
    bytes
}

// A type 2 pixel program reading generic 0 x and y perspective-correct and writing the texel there from the texture
// whose handle is c[2][0x0].
fn textured_program() -> Vec<u8> {
    program([1, 7, 10, 12, 193, 195, 576, 577, 578, 579], &[ipa(0, 0x80, 0), ipa(1, 0x84, 0), texture(inst::TEX_LZ_2D, 0, 2, 0, 2, 0, 0xf), exit(), bra(-16)])
}

pub fn triangle_guest<'a>(state: &'a VkState, width: u32, height: u32, vertices: &[Vertex; 3]) -> Result<Gpu<'a>, MemoryError> {
    let target_pages = (u64::from(width * height) * 4).div_ceil(PAGE);
    let mut gpu = Gpu::new(state, GuestRam::new(((8 + target_pages) * PAGE) as usize));
    for (va, page, pages) in [(TEXTURES, 0, 1), (PUSHBUFFER, 3, 1), (VERTEX_PROGRAM, 1, 1), (VERTICES, 5, 1), (CBUF, 2, 1), (REPORT, 6, 1), (TARGET, 8, target_pages)] {
        gpu.map(va, page * PAGE, pages * PAGE);
    }
    let (mut vertex, mut pixel) = ([0; scene::VERTEX_PROGRAM_BYTES], [0; scene::PIXEL_PROGRAM_BYTES]);
    scene::vertex_program(&mut vertex);
    scene::pixel_program(&mut pixel);
    gpu.write(VERTEX_PROGRAM, &vertex)?;
    gpu.write(PIXEL_PROGRAM, &pixel)?;
    gpu.write(TEXTURED_PROGRAM, &textured_program())?;
    gpu.write(VERTICES, &vertex_bytes(vertices))?;
    Ok(gpu)
}

pub fn run(gpu: &mut Gpu, words: &[u32]) -> Result<(), String> {
    gpu.write(PUSHBUFFER, &bytes(words)).map_err(|e| format!("{e:?}"))?;
    gpu.submit(&[entry(PUSHBUFFER, words.len())])
}

/// Points texture 0 at a pitch-linear RGBA8 image and sampler 0 at nearest clamp-to-edge filtering.
pub fn texture_header(gpu: &mut Gpu, address: u64, width: u32, height: u32, pitch: u32) -> Result<(), MemoryError> {
    gpu.write(TEXTURES, &bytes(&scene::pitch_rgba8_tic(address, width, height, pitch)))?;
    gpu.write(TEXTURES + 0x100, &bytes(&scene::NEAREST_CLAMP_TSC))?;
    gpu.write(TEXTURES + 0x200, &0u32.to_le_bytes())
}

/// Clears `target` to white and draws the vertices from `first` as a four-vertex strip textured with texture 0.
pub fn textured_quad_frame(gpu: &mut Gpu, width: u32, height: u32, target: u64, first: u32) -> Result<(), String> {
    let [textures, program] = [TEXTURES, TEXTURED_PROGRAM].map(split);
    let mut words = Vec::new();
    frame_state(width, height, &Addresses { target, ..ADDRESSES }, [1.0, 0.0, 0.0, 1.0], &mut words);
    inc(SET_PIPELINE_SHADER + 5 * 64 + 0x14, &program, &mut words);
    inc(SET_TEX_HEADER_POOL_A, &[textures[0], textures[1], 0], &mut words);
    inc(SET_TEX_SAMPLER_POOL_A, &[textures[0], textures[1] + 0x100, 0], &mut words);
    inc(SET_CONSTANT_BUFFER_SELECTOR_A, &[16, textures[0], textures[1] + 0x200], &mut words);
    inc(BIND_GROUP_CONSTANT_BUFFER + 4 * 32, &[2 << 4 | 1], &mut words);
    inc(SET_COLOR_CLEAR_VALUE, &[1.0f32.to_bits(); 4], &mut words);
    inc(CLEAR_SURFACE, &[0x3c], &mut words);
    inc(DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_FIRST, &[5 << 28 | 4 << 16 | first], &mut words);
    run(gpu, &words)
}

pub fn triangle_frame(gpu: &mut Gpu, width: u32, height: u32, matrix: [f32; 4], payload: u32) -> Result<(), String> {
    let mut words = Vec::new();
    scene::triangle_frame(width, height, &ADDRESSES, matrix, payload, &mut words);
    run(gpu, &words)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::UnicornCPU;
    use crate::gpu::compute_class::{SEND_PCAS_A, SEND_SIGNALING_PCAS2_B};
    use crate::tests::gpu::harness::vk_state;
    use crate::tests::run::arm64;
    use crate::tests::gpu_test::inst::{ald, ast, cvt, fadd, fmul_imm, i2f, mov_imm};
    use scene::host::{SEMAPHORE_A, SET_OBJECT};
    use std::io::Write;
    use std::process::{Command, Stdio};

    const INDICES: u64 = 0x1_0050_0000;
    const DEPTH: u64 = 0x1_0060_0000;
    const TARGET_B: u64 = 0x1_0200_0000;
    const INSTANCED_PROGRAM: u64 = 0x1_0010_3000;
    // A four-vertex strip with uv in the colour attribute's red and green.
    fn quad(half: f32) -> [Vertex; 4] {
        [([-half, -half], [0, 0, 0, 0]), ([half, -half], [255, 0, 0, 0]), ([-half, half], [0, 255, 0, 0]), ([half, half], [255, 255, 0, 0])]
    }
    const SIZE: u32 = 64;
    const IDENTITY: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
    const WHITE: [u8; 4] = [255; 4];
    // Pixel corners (4, 4), (60, 4) and (4, 60) in normalized device coordinates.
    const CORNERS: [[f32; 2]; 3] = [[-0.875, -0.875], [0.875, -0.875], [-0.875, 0.875]];
    const RED: [u8; 4] = [255, 0, 0, 255];
    const GREEN: [u8; 4] = [0, 255, 0, 255];
    const BLUE: [u8; 4] = [0, 0, 255, 255];
    // On a page the GPU leaves unmapped, so only the guest's stores mark the vertex page.
    const GUEST_CODE: u64 = 7 * PAGE;
    const GUEST_LOAD: u64 = GUEST_CODE + 20;

    fn physical(gpu: &Gpu, va: u64) -> u64 {
        gpu.space.resolve(va, 1).unwrap().next().unwrap().start
    }

    /// A core on the GPU's RAM that has copied `vertices` over the vertex buffer word by word with guest stores.
    fn guest_store_vertices(gpu: &Gpu, vertices: &[Vertex]) -> UnicornCPU {
        let cpu = UnicornCPU::new_with_shared_mem(0, gpu.ram.clone()).unwrap();
        let code = [arm64::ldr_post(3, 1, 4), arm64::str_post(3, 2, 4), arm64::subs_imm(0, 0, 1), arm64::b_ne(-3), arm64::brk(0), arm64::ldr(5, 4), arm64::brk(0)];
        for (pc, word) in (GUEST_CODE..).step_by(4).zip(code) {
            cpu.write_u32(pc, word);
        }
        let bytes = vertex_bytes(vertices);
        gpu.ram.write(GUEST_CODE + 0x100, &bytes);
        for (reg, value) in [(0, bytes.len() as u64 / 4), (1, GUEST_CODE + 0x100), (2, physical(gpu, VERTICES))] {
            cpu.set_x(reg, value);
        }
        cpu.set_pc(GUEST_CODE);
        assert_eq!(cpu.run(), 1);
        cpu
    }

    fn pixel(gpu: &Gpu, target: u64, x: u32, y: u32) -> [u8; 4] {
        let mut rgba = [0; 4];
        gpu.read(target + u64::from(y * SIZE + x) * 4, &mut rgba).unwrap();
        rgba
    }

    fn clear_white(words: &mut Vec<u32>) {
        inc(SET_COLOR_CLEAR_VALUE, &[1.0f32.to_bits(); 4], words);
        inc(CLEAR_SURFACE, &[0x3c], words);
    }

    fn draw_and_check(gpu: &mut Gpu, payload: u32, colours: [[u8; 4]; 3]) {
        triangle_frame(gpu, SIZE, SIZE, IDENTITY, payload).unwrap();
        let mut report = [0; 4];
        gpu.read(REPORT, &mut report).unwrap();
        assert_eq!(u32::from_le_bytes(report), payload);
        let mut pixels = vec![0; (SIZE * SIZE * 4) as usize];
        gpu.read(TARGET, &mut pixels).unwrap();
        let inside = |x: u32, y: u32| {
            let [u, v] = [x, y].map(|p| (p as f32 + 0.5 - 4.0) / 56.0);
            let weights = [1.0 - u - v, u, v];
            [0, 1, 2, 3].map(|c| (0..3).map(|i| weights[i] * f32::from(colours[i][c])).sum::<f32>().round() as u8)
        };
        let expected = [(4, 4), (58, 4), (4, 58), (20, 20)].map(|(x, y)| ((x, y), inside(x, y)));
        for ((x, y), want) in expected.into_iter().chain([((0, 0), [255; 4]), ((40, 40), [255; 4])]) {
            let got = &pixels[((y * SIZE + x) * 4) as usize..][..4];
            assert!(got.iter().zip(want).all(|(&g, w)| g.abs_diff(w) <= 2), "({x}, {y}): {got:?} != {want:?}");
        }
    }

    fn sha256(bytes: &[u8]) -> String {
        let mut sha256sum = Command::new("sha256sum").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
        sha256sum.stdin.take().unwrap().write_all(bytes).unwrap();
        String::from_utf8(sha256sum.wait_with_output().unwrap().stdout).unwrap()[..64].to_string()
    }

    #[test]
    fn the_guest_triangle_scene_writes_the_bytes_the_host_triangle_draws() {
        let (mut pushbuffer, mut vertices) = ([0; 256], [0; 36]);
        let (mut vertex, mut pixel) = ([0; scene::VERTEX_PROGRAM_BYTES], [0; scene::PIXEL_PROGRAM_BYTES]);
        let mut out = scene::Scene { pushbuffer: &mut pushbuffer, vertex_program: &mut vertex, pixel_program: &mut pixel, vertices: &mut vertices };
        let len = scene::triangle_scene(&mut out, &ADDRESSES, [SIZE; 2], &[(CORNERS[0], RED), (CORNERS[1], GREEN), (CORNERS[2], BLUE)], IDENTITY, 0x1234_5678);
        let image = [&bytes(&pushbuffer[..len])[..], &vertex, &pixel, &vertices].concat();
        assert_eq!(sha256(&image), "270bc71b1a547884e5218910d54fe52f835f6685e003d25287c2324ab44a775d");
    }

    // QMDV03_00 fields from clc7c0qmd.h: grid in words 12-14, block in 18-19, release 0 in 24-26, program in 48-49.
    #[test]
    fn qmd_launch_writes_the_gradient_kernel_output_into_guest_memory_and_releases() {
        const INVALIDATE_COPY_SCHEDULE: u32 = 3;
        // The kernel stores its 1920x1080 RGBA8 frame at the address its MOV R4, 0x1000 and MOV R5, 0x2 build.
        const FRAME: u64 = 0x2_0000_1000;
        const CONTROL: u64 = 0x2_007f_0000;
        let [qmd_va, program_va, release_va] = [CONTROL + 0x100, CONTROL + 0x200, CONTROL + 0xf000];
        let Some(state) = vk_state() else { return };
        let json = include_str!("../../../tests/fixtures/gradient-sm86.json");
        let field = |key: &str, open: char, close: char| json.split_once(&format!("\"{key}\":{open}")).unwrap().1.split_once(close).unwrap().0;
        let code: Vec<u8> = field("words", '[', ']')
            .split(',')
            .map(|word| u128::from_str_radix(word.trim().trim_matches('"'), 16).unwrap())
            .chain([bra(-16)])
            .flat_map(u128::to_le_bytes)
            .collect();
        let mut gpu = Gpu::new(&state, GuestRam::new((128 * PAGE) as usize));
        gpu.map(0x2_0000_0000, 0, 128 * PAGE);
        let mut qmd = [0; 64];
        [qmd[12], qmd[13], qmd[14]] = [120, 68, 1];
        [qmd[18], qmd[19]] = [16 << 16 | 3 << 4, 1 << 16 | 16];
        [qmd[24], qmd[25], qmd[26]] = [release_va as u32, 1 << 30 | 1 << 23 | 2, 0xfeed_beef];
        [qmd[48], qmd[49]] = [program_va as u32, 2];
        let mut words = Vec::new();
        inc(SET_OBJECT, &[0xc7c0], &mut words);
        inc(SEND_PCAS_A, &[(qmd_va >> 8) as u32], &mut words);
        inc(SEND_SIGNALING_PCAS2_B, &[INVALIDATE_COPY_SCHEDULE], &mut words);
        gpu.write(program_va, &code).unwrap();
        gpu.write(qmd_va, &bytes(&qmd)).unwrap();
        gpu.write(CONTROL, &bytes(&words)).unwrap();
        gpu.submit(&[entry(CONTROL, words.len())]).unwrap();
        let mut frame = vec![0; 1920 * 1080 * 4];
        gpu.read(FRAME, &mut frame).unwrap();
        assert_eq!(sha256(&frame), field("sha256", '"', '"'));
        let mut release = [0; 4];
        gpu.read(release_va, &mut release).unwrap();
        assert_eq!(u32::from_le_bytes(release), 0xfeed_beef);
    }

    #[test]
    fn pushbuffer_triangle_lands_in_guest_memory_and_follows_vertex_remaps() {
        let Some(state) = vk_state() else { return };
        let mut gpu = triangle_guest(&state, SIZE, SIZE, &[(CORNERS[0], RED), (CORNERS[1], GREEN), (CORNERS[2], BLUE)]).unwrap();
        draw_and_check(&mut gpu, 0x1234_5678, [RED, GREEN, BLUE]);
        gpu.space.unmap(VERTICES, PAGE);
        gpu.map(VERTICES, 7 * PAGE, PAGE);
        gpu.write(VERTICES, &vertex_bytes(&[(CORNERS[0], GREEN), (CORNERS[1], BLUE), (CORNERS[2], RED)])).unwrap();
        draw_and_check(&mut gpu, 0x9abc_def0, [GREEN, BLUE, RED]);
    }

    #[test]
    fn a_core_stores_the_vertices_the_gpu_draws_and_loads_the_report_the_gpu_wrote() {
        let Some(state) = vk_state() else { return };
        let mut gpu = triangle_guest(&state, SIZE, SIZE, &[(CORNERS[0], WHITE); 3]).unwrap();
        let mapped = gpu.memory.generation(physical(&gpu, VERTICES), PAGE);
        let cpu = guest_store_vertices(&gpu, &[(CORNERS[0], RED), (CORNERS[1], GREEN), (CORNERS[2], BLUE)]);
        draw_and_check(&mut gpu, 0x1234_5678, [RED, GREEN, BLUE]);
        assert!(gpu.memory.generation(physical(&gpu, VERTICES), PAGE) > mapped);
        cpu.set_x(4, physical(&gpu, REPORT));
        cpu.set_pc(GUEST_LOAD);
        assert_eq!(cpu.run(), 1);
        assert_eq!(cpu.get_x(5), 0x1234_5678);
    }

    #[test]
    fn a_second_frame_draws_the_vertices_a_core_stored_after_the_first() {
        let Some(state) = vk_state() else { return };
        let mut gpu = triangle_guest(&state, SIZE, SIZE, &[(CORNERS[0], RED), (CORNERS[1], GREEN), (CORNERS[2], BLUE)]).unwrap();
        draw_and_check(&mut gpu, 1, [RED, GREEN, BLUE]);
        guest_store_vertices(&gpu, &[(CORNERS[0], GREEN), (CORNERS[1], BLUE), (CORNERS[2], RED)]);
        draw_and_check(&mut gpu, 2, [GREEN, BLUE, RED]);
    }

    #[test]
    fn begin_end_indexed_draw_builds_a_quad_from_two_triangles() {
        const TWO_BYTES: u32 = 1;
        const TRIANGLES: u32 = 4;
        let Some(state) = vk_state() else { return };
        let mut gpu = triangle_guest(&state, SIZE, SIZE, &[(CORNERS[0], RED); 3]).unwrap();
        gpu.write(VERTICES, &vertex_bytes(&[[-0.5, -0.5], [0.5, -0.5], [-0.5, 0.5], [0.5, 0.5]].map(|corner| (corner, RED)))).unwrap();
        gpu.map(INDICES, 4 * PAGE, PAGE);
        gpu.write(INDICES, &bytes(&[0 | 1 << 16, 2 | 2 << 16, 1 | 3 << 16])).unwrap();
        let mut words = Vec::new();
        frame_state(SIZE, SIZE, &ADDRESSES, IDENTITY, &mut words);
        clear_white(&mut words);
        inc(SET_INDEX_BUFFER_A, &split(INDICES), &mut words);
        inc(SET_INDEX_BUFFER_E, &[TWO_BYTES, 0], &mut words);
        inc(BEGIN, &[TRIANGLES], &mut words);
        inc(DRAW_INDEX_BUFFER, &[6], &mut words);
        inc(END, &[0], &mut words);
        run(&mut gpu, &words).unwrap();
        for (x, y, want) in [(20, 20, RED), (44, 20, RED), (20, 44, RED), (44, 44, RED), (8, 8, WHITE), (56, 56, WHITE)] {
            assert_eq!(pixel(&gpu, TARGET, x, y), want, "({x}, {y})");
        }
    }

    #[test]
    fn depth_test_keeps_the_nearer_triangle_whichever_draws_first() {
        const ZF32: u32 = 0x0a;
        const OGL_LESS: u32 = 0x201;
        const CLEAR_Z_AND_RGBA: u32 = 0x3d;
        let Some(state) = vk_state() else { return };
        let mut gpu = triangle_guest(&state, SIZE, SIZE, &[(CORNERS[0], RED); 3]).unwrap();
        let [near, far] = [RED, GREEN].map(|colour| CORNERS.map(|corner| (corner, colour)));
        gpu.write(VERTICES, &vertex_bytes(&[near, far].concat())).unwrap();
        for order in [[(0.25, 0), (0.75, 3)], [(0.75, 3), (0.25, 0)]] {
            let mut words = Vec::new();
            frame_state(SIZE, SIZE, &ADDRESSES, IDENTITY, &mut words);
            inc(SET_ZT_A, &[split(DEPTH)[0], split(DEPTH)[1], ZF32], &mut words);
            inc(SET_ZT_SELECT, &[1], &mut words);
            inc(SET_DEPTH_TEST, &[1], &mut words);
            inc(SET_DEPTH_WRITE, &[1], &mut words);
            inc(SET_DEPTH_FUNC, &[OGL_LESS], &mut words);
            inc(SET_Z_CLEAR_VALUE, &[1.0f32.to_bits()], &mut words);
            inc(SET_COLOR_CLEAR_VALUE, &[1.0f32.to_bits(); 4], &mut words);
            inc(CLEAR_SURFACE, &[CLEAR_Z_AND_RGBA], &mut words);
            for (depth, first) in order {
                inc(SET_CONSTANT_BUFFER_SELECTOR_A, &[32, split(CBUF)[0], split(CBUF)[1], 16], &mut words);
                inc(LOAD_CONSTANT_BUFFER, &[f32::to_bits(depth)], &mut words);
                inc(DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_FIRST, &[4 << 28 | 3 << 16 | first], &mut words);
            }
            run(&mut gpu, &words).unwrap();
            assert_eq!(pixel(&gpu, TARGET, 20, 20), RED, "{order:?}");
        }
    }

    #[test]
    fn additive_blend_sums_two_translucent_triangles() {
        const OGL_FUNC_ADD: u32 = 0x8006;
        const OGL_ONE: u32 = 0x4001;
        let Some(state) = vk_state() else { return };
        let mut gpu = triangle_guest(&state, SIZE, SIZE, &[(CORNERS[0], RED); 3]).unwrap();
        let [first, second] = [[100, 0, 0, 128], [0, 100, 0, 128]].map(|colour| CORNERS.map(|corner| (corner, colour)));
        gpu.write(VERTICES, &vertex_bytes(&[first, second].concat())).unwrap();
        let mut words = Vec::new();
        frame_state(SIZE, SIZE, &ADDRESSES, IDENTITY, &mut words);
        inc(SET_COLOR_CLEAR_VALUE, &[0; 4], &mut words);
        inc(CLEAR_SURFACE, &[0x3c], &mut words);
        inc(SET_BLEND_COLOR_OP, &[OGL_FUNC_ADD, OGL_ONE, OGL_ONE], &mut words);
        inc(SET_BLEND, &[1], &mut words);
        for first in [0, 3] {
            inc(DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_FIRST, &[4 << 28 | 3 << 16 | first], &mut words);
        }
        run(&mut gpu, &words).unwrap();
        assert_eq!(pixel(&gpu, TARGET, 20, 20), [100, 100, 0, 255]);
    }

    #[test]
    fn textured_quad_shows_each_texel_of_a_two_by_two_texture_with_nearest_sampling() {
        let Some(state) = vk_state() else { return };
        let mut gpu = triangle_guest(&state, SIZE, SIZE, &[(CORNERS[0], RED); 3]).unwrap();
        gpu.write(VERTICES, &vertex_bytes(&quad(0.5))).unwrap();
        let texels = [[RED, GREEN], [BLUE, WHITE]];
        for (row, texels) in (0..).zip(texels) {
            gpu.write(TEXTURES + 0x1000 + 32 * row, &texels.concat()).unwrap();
        }
        texture_header(&mut gpu, TEXTURES + 0x1000, 2, 2, 32).unwrap();
        textured_quad_frame(&mut gpu, SIZE, SIZE, TARGET, 0).unwrap();
        for (x, y, want) in [(20, 20, RED), (44, 20, GREEN), (20, 44, BLUE), (44, 44, WHITE), (8, 8, WHITE)] {
            assert_eq!(pixel(&gpu, TARGET, x, y), want, "({x}, {y})");
        }
    }

    #[test]
    fn a_target_sampled_as_a_texture_shows_its_latest_draw() {
        let Some(state) = vk_state() else { return };
        let mut gpu = triangle_guest(&state, SIZE, SIZE, &[(CORNERS[0], RED); 3]).unwrap();
        gpu.map(TARGET_B, 4 * PAGE, PAGE);
        texture_header(&mut gpu, TARGET, SIZE, SIZE, SIZE * 4).unwrap();
        let mut previous = Vec::new();
        for colours in [[RED, GREEN, BLUE], [GREEN, BLUE, RED]] {
            let triangle = CORNERS.into_iter().zip(colours).collect::<Vec<_>>();
            gpu.write(VERTICES, &vertex_bytes(&[&triangle[..], &quad(1.0)].concat())).unwrap();
            triangle_frame(&mut gpu, SIZE, SIZE, IDENTITY, 0).unwrap();
            textured_quad_frame(&mut gpu, SIZE, SIZE, TARGET_B, 3).unwrap();
            let [mut a, mut b] = [(); 2].map(|_| vec![0; (SIZE * SIZE * 4) as usize]);
            gpu.read(TARGET, &mut a).unwrap();
            gpu.read(TARGET_B, &mut b).unwrap();
            assert!(a == b && a != previous, "{colours:?}");
            previous = a;
        }
    }

    // A type 1 vertex program reading generic 0 x and y, generic 1 and generic 2 x and y: the corner, an integer colour and
    // an offset. It writes position corner + offset and generic 0 colour / 255.
    fn instanced_program() -> Vec<u8> {
        let colour = (0..4).flat_map(|c| [i2f(8 + c, 4 + c, cvt::F32, cvt::U32), fmul_imm(8 + c, 8 + c, 1.0 / 255.0)]);
        let code: Vec<u128> = [ald(0, 0x80, 2), ald(4, 0x90, 4), ald(2, 0xa0, 2), fadd(12, 0, 2), fadd(13, 1, 3)]
            .into_iter()
            .chain(colour)
            .chain([mov_imm(14, 0), mov_imm(15, 1.0f32.to_bits()), ast(0x70, 12, 4), ast(0x80, 8, 4), exit(), bra(-16)])
            .collect();
        program([0, 7, 10].into_iter().chain(192..202).chain(428..436), &code)
    }

    #[test]
    fn instanced_stream_places_and_colours_each_instance_from_unsigned_integer_attributes() {
        const R32_G32_FLOAT: u32 = 0x04 << 21 | 7 << 27;
        const R8_G8_B8_A8_UINT: u32 = 0x0a << 21 | 4 << 27;
        let Some(state) = vk_state() else { return };
        let mut gpu = triangle_guest(&state, SIZE, SIZE, &[(CORNERS[0], RED); 3]).unwrap();
        gpu.write(INSTANCED_PROGRAM, &instanced_program()).unwrap();
        let corners = [[-0.25f32, -0.25], [0.25, -0.25], [-0.25, 0.25], [0.25, 0.25]];
        gpu.write(VERTICES, &corners.iter().flatten().flat_map(|c| c.to_le_bytes()).collect::<Vec<_>>()).unwrap();
        let instances = vertex_bytes(&[([-0.5, 0.0], RED), ([0.5, 0.0], GREEN)]);
        let instances: Vec<u8> = instances.chunks(12).flat_map(|v| [&v[8..], &v[..8]].concat()).collect();
        gpu.write(VERTICES + 0x100, &instances).unwrap();
        let [vertices, program] = [VERTICES, INSTANCED_PROGRAM].map(split);
        let mut words = Vec::new();
        frame_state(SIZE, SIZE, &ADDRESSES, IDENTITY, &mut words);
        clear_white(&mut words);
        inc(SET_VERTEX_STREAM_SIZE_A + 8, &[0, 0x100], &mut words);
        inc(SET_VERTEX_STREAM_A_FORMAT, &[0x1000 | 8], &mut words);
        inc(SET_VERTEX_STREAM_A_FORMAT + 16, &[0x1000 | 12, vertices[0], vertices[1] + 0x100, 1], &mut words);
        inc(SET_VERTEX_STREAM_INSTANCE_A + 4, &[1], &mut words);
        inc(SET_VERTEX_ATTRIBUTE_A, &[R32_G32_FLOAT, 1 | R8_G8_B8_A8_UINT, 1 | 4 << 7 | R32_G32_FLOAT], &mut words);
        inc(SET_PIPELINE_SHADER + 64 + 0x14, &program, &mut words);
        inc(DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_FIRST, &[5 << 28 | 4 << 16], &mut words);
        inc(DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_SUBSEQUENT, &[5 << 28 | 4 << 16], &mut words);
        run(&mut gpu, &words).unwrap();
        for (x, y, want) in [(16, 32, RED), (48, 32, GREEN), (32, 32, WHITE), (16, 16, WHITE)] {
            assert_eq!(pixel(&gpu, TARGET, x, y), want, "({x}, {y})");
        }
    }

    #[test]
    fn a_clear_no_draw_consumes_lands_in_guest_memory_and_the_next_draw_loads_it() {
        let Some(state) = vk_state() else { return };
        let mut gpu = triangle_guest(&state, SIZE, SIZE, &[(CORNERS[0], GREEN), (CORNERS[1], GREEN), (CORNERS[2], GREEN)]).unwrap();
        let mut words = Vec::new();
        frame_state(SIZE, SIZE, &ADDRESSES, IDENTITY, &mut words);
        inc(SET_COLOR_CLEAR_VALUE, &[0.0, 0.0, 1.0, 1.0].map(f32::to_bits), &mut words);
        inc(CLEAR_SURFACE, &[0x3c], &mut words);
        run(&mut gpu, &words).unwrap();
        assert_eq!([pixel(&gpu, TARGET, 20, 20), pixel(&gpu, TARGET, 40, 40)], [BLUE; 2]);
        words.clear();
        frame_state(SIZE, SIZE, &ADDRESSES, IDENTITY, &mut words);
        inc(DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_FIRST, &[4 << 28 | 3 << 16], &mut words);
        run(&mut gpu, &words).unwrap();
        assert_eq!([pixel(&gpu, TARGET, 20, 20), pixel(&gpu, TARGET, 40, 40)], [GREEN, BLUE]);
    }

    #[test]
    fn reports_write_their_structure_size_and_acquires_need_the_value_released() {
        const FOUR_WORDS: u32 = 0;
        const ONE_WORD: u32 = 1 << 28;
        const ACQUIRE: u32 = 1;
        let Some(state) = vk_state() else { return };
        let mut gpu = triangle_guest(&state, SIZE, SIZE, &[(CORNERS[0], RED); 3]).unwrap();
        let [hi, lo] = split(REPORT);
        let mut report = [0xff; 16];
        gpu.write(REPORT, &report).unwrap();
        let mut words = Vec::new();
        inc(SET_OBJECT, &[0xc797], &mut words);
        inc(SET_REPORT_SEMAPHORE_A, &[hi, lo, 9, ONE_WORD], &mut words);
        run(&mut gpu, &words).unwrap();
        gpu.read(REPORT, &mut report).unwrap();
        assert_eq!(report[..8], [9, 0, 0, 0, 0xff, 0xff, 0xff, 0xff]);
        inc(SET_REPORT_SEMAPHORE_A, &[hi, lo, 7, FOUR_WORDS], &mut words);
        inc(SEMAPHORE_A, &[hi, lo, 7, ACQUIRE], &mut words);
        run(&mut gpu, &words).unwrap();
        gpu.read(REPORT, &mut report).unwrap();
        assert_eq!(u64::from_le_bytes(report[..8].try_into().unwrap()), 7);
        assert_ne!(report[8..], [0xff; 8]);
        inc(SEMAPHORE_A, &[hi, lo, 8, ACQUIRE], &mut words);
        let error = run(&mut gpu, &words).unwrap_err();
        assert!(error.contains("semaphore acquire at 0x100400000 waits for 0x8 but memory holds 0x7"), "{error}");
    }

    #[test]
    fn a_counter_clockwise_triangle_stays_front_facing_under_either_y_flip() {
        const UPPER_LEFT: u32 = 0;
        const LOWER_LEFT: u32 = 1;
        const OGL_CCW: u32 = 0x901;
        const OGL_BACK: u32 = 0x405;
        let Some(state) = vk_state() else { return };
        let mut gpu = triangle_guest(&state, SIZE, SIZE, &[(CORNERS[0], GREEN), (CORNERS[1], GREEN), (CORNERS[2], GREEN)]).unwrap();
        for (scale_y, origin) in [(-32.0f32, UPPER_LEFT), (32.0, LOWER_LEFT)] {
            let mut words = Vec::new();
            frame_state(SIZE, SIZE, &ADDRESSES, IDENTITY, &mut words);
            clear_white(&mut words);
            inc(SET_VIEWPORT_SCALE_X + 4, &[scale_y.to_bits()], &mut words);
            inc(SET_WINDOW_ORIGIN, &[origin], &mut words);
            inc(OGL_SET_CULL, &[1, OGL_CCW, OGL_BACK], &mut words);
            inc(DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_FIRST, &[4 << 28 | 3 << 16], &mut words);
            run(&mut gpu, &words).unwrap();
            assert_eq!([pixel(&gpu, TARGET, 8, 50), pixel(&gpu, TARGET, 50, 8)], [GREEN, WHITE], "scale {scale_y} origin {origin}");
        }
    }
}
