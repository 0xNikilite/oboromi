#![allow(dead_code)]

use crate::gpu::dlss::DlssEngine;
use crate::mmu::Memory;

/// Tegra NVGPU Channel / Pushbuffer & 3D Rasterizer Engine.
///
/// Models the Nvidia Tegra (Maxwell GM20B / Ampere T239) command submission interface
/// used by Horizon OS to feed draw calls, surface clears, and vertex arrays to the GPU.

pub const NVGPU_IOCTL_CHANNEL_SUBMIT_GPFIFO: u32 = 0xC0104808;
pub const NVHOST_IOCTL_CTRL_SYNCPT_INCR: u32 = 0x40040003;
pub const NVHOST_IOCTL_CTRL_SYNCPT_WAIT: u32 = 0xC00C0006;

/// Method Register Offsets on the Tegra 3D Engine
pub const METHOD_SET_RENDER_TARGET_ADDR: u16 = 0x0200;
pub const METHOD_SET_CLEAR_COLOR: u16 = 0x0360;
pub const METHOD_SET_MVP_MATRIX: u16 = 0x0400;
pub const METHOD_BIND_SHADER_PROGRAM: u16 = 0x0480;
pub const METHOD_SET_DEPTH_TEST_ENABLE: u16 = 0x04B0;
pub const METHOD_BIND_TEXTURE_2D: u16 = 0x0500;
pub const METHOD_DRAW_ARRAYS: u16 = 0x0585;
pub const METHOD_DRAW_TEXTURED_ARRAYS: u16 = 0x0586;
pub const METHOD_CLEAR_SURFACE: u16 = 0x0674;
pub const METHOD_SET_VERTEX_ARRAY_ADDR: u16 = 0x0700;
pub const METHOD_SET_VIEWPORT_DIMS: u16 = 0x0860;
pub const METHOD_DISPATCH_DLSS_UPSCALE: u16 = 0x0900;

/// Subchannels available on the Tegra GPU
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuSubchannel {
    Engine2D = 1,
    Engine3D = 2,
    Compute = 3,
    CopyEngine = 4,
}

/// Pushbuffer method submission mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmissionMode {
    NonIncreasing = 0,
    Increasing = 1,
    Once = 2,
}

/// Decoded GPU Command from the GPFIFO pushbuffer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuCommand {
    pub subchannel: u8,
    pub method: u16,
    pub mode: SubmissionMode,
    pub args: Vec<u32>,
}

/// 3D Vertex with Normalized Device Coordinates (NDC), RGB color, and UV texture coordinates
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub u: f32,
    pub v: f32,
}

/// NVGPU 3D State Engine
#[derive(Debug)]
pub struct Gpu3DState {
    pub render_target_addr: u64,
    pub vertex_array_addr: u64,
    pub texture_addr: u64,
    pub texture_width: u32,
    pub texture_height: u32,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub clear_color: [f32; 4],
    pub mvp_matrix: [f32; 16],
    pub depth_test_enabled: bool,
    pub depth_buffer: Vec<f32>,
    pub last_decompiled_glsl: Option<String>,
    pub last_decompiled_spirv_words: usize,
    pub total_draw_calls: u64,
    pub total_triangles_rendered: u64,
    pub total_texels_sampled: u64,
    pub total_depth_tests_passed: u64,
    pub total_depth_tests_rejected: u64,
    pub total_commands_processed: u64,
}

impl Default for Gpu3DState {
    fn default() -> Self {
        Self {
            render_target_addr: 0x90000000,
            vertex_array_addr: 0,
            texture_addr: 0,
            texture_width: 0,
            texture_height: 0,
            viewport_width: 1280,
            viewport_height: 720,
            clear_color: [0.05, 0.05, 0.10, 1.0],
            // 4x4 Identity Matrix (Column-Major / Row-Major neutral for identity)
            mvp_matrix: [
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
            depth_test_enabled: false,
            depth_buffer: Vec::new(),
            last_decompiled_glsl: None,
            last_decompiled_spirv_words: 0,
            total_draw_calls: 0,
            total_triangles_rendered: 0,
            total_texels_sampled: 0,
            total_depth_tests_passed: 0,
            total_depth_tests_rejected: 0,
            total_commands_processed: 0,
        }
    }
}

pub struct NvgpuChannel {
    pub state: Gpu3DState,
    pub dlss: DlssEngine,
}

impl Default for NvgpuChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl NvgpuChannel {
    pub fn new() -> Self {
        Self {
            state: Gpu3DState::default(),
            dlss: DlssEngine::new(),
        }
    }

    /// Helper to encode a Tegra GPFIFO method header word.
    #[inline]
    pub const fn encode_header(mode: SubmissionMode, subchannel: u8, method: u16, count: u16) -> u32 {
        ((mode as u32 & 0x7) << 29)
            | ((count as u32 & 0x1FFF) << 16)
            | ((subchannel as u32 & 0x7) << 13)
            | (method as u32 & 0x1FFF)
    }

    /// Parses and executes a stream of GPFIFO pushbuffer words from guest memory.
    pub fn process_pushbuffer(&mut self, mmu: &mut Memory, pb_addr: u64, word_count: usize) {
        let mut offset = 0;

        while offset < word_count {
            let addr = pb_addr + (offset * 4) as u64;
            let header = match mmu.read_u32(addr) {
                Ok(h) => h,
                Err(_) => break,
            };
            offset += 1;

            // Tegra Pushbuffer Header format:
            // bits [31:29] = Submission Mode (0 = NonIncreasing, 1 = Increasing, 2 = Once)
            // bits [28:16] = Count (number of argument words)
            // bits [15:13] = Subchannel (0..7)
            // bits [12:0]  = Method register offset
            let mode_val = (header >> 29) & 0x7;
            let count = ((header >> 16) & 0x1FFF) as usize;
            let subchannel = ((header >> 13) & 0x7) as u8;
            let method = (header & 0x1FFF) as u16;

            let mode = match mode_val {
                0 => SubmissionMode::NonIncreasing,
                1 => SubmissionMode::Increasing,
                _ => SubmissionMode::Once,
            };

            let mut args = Vec::with_capacity(count);
            for _ in 0..count {
                if offset >= word_count {
                    break;
                }
                let arg_addr = pb_addr + (offset * 4) as u64;
                if let Ok(val) = mmu.read_u32(arg_addr) {
                    args.push(val);
                }
                offset += 1;
            }

            self.execute_command(
                mmu,
                GpuCommand {
                    subchannel,
                    method,
                    mode,
                    args,
                },
            );
        }
    }

    /// Dispatches high-level 3D engine commands and executes rasterization into VRAM.
    fn execute_command(&mut self, mmu: &mut Memory, cmd: GpuCommand) {
        self.state.total_commands_processed += 1;

        match cmd.method {
            // 0x0200: Set Render Target Address
            METHOD_SET_RENDER_TARGET_ADDR => {
                if cmd.args.len() >= 2 {
                    self.state.render_target_addr =
                        ((cmd.args[1] as u64) << 32) | (cmd.args[0] as u64);
                } else if !cmd.args.is_empty() {
                    self.state.render_target_addr = cmd.args[0] as u64;
                }
            }
            // 0x0860: Set Viewport Dimensions
            METHOD_SET_VIEWPORT_DIMS => {
                if cmd.args.len() >= 2 {
                    self.state.viewport_width = cmd.args[0];
                    self.state.viewport_height = cmd.args[1];
                }
            }
            // 0x0360: Set Clear Color (RGBA floats)
            METHOD_SET_CLEAR_COLOR => {
                if cmd.args.len() >= 4 {
                    self.state.clear_color = [
                        f32::from_bits(cmd.args[0]),
                        f32::from_bits(cmd.args[1]),
                        f32::from_bits(cmd.args[2]),
                        f32::from_bits(cmd.args[3]),
                    ];
                }
            }
            // 0x0400: Set 4x4 Model-View-Projection (MVP) Matrix Uniform (16 floats)
            METHOD_SET_MVP_MATRIX => {
                for (i, &word) in cmd.args.iter().take(16).enumerate() {
                    self.state.mvp_matrix[i] = f32::from_bits(word);
                }
            }
            // 0x0480: Bind and Decompile Ampere SASS Shader Program from Guest Memory
            METHOD_BIND_SHADER_PROGRAM => {
                if !cmd.args.is_empty() {
                    let shader_addr = if cmd.args.len() >= 2 {
                        ((cmd.args[1] as u64) << 32) | (cmd.args[0] as u64)
                    } else {
                        cmd.args[0] as u64
                    };
                    let mut sass_words = Vec::new();
                    for i in 0..32 {
                        if let Ok(w) = mmu.read_u64(shader_addr + (i * 8) as u64) {
                            sass_words.push(w);
                            if (w >> 56) == 0xFF {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    let decompiled = crate::gpu::shader::SassShaderDecompiler::decompile(&sass_words);
                    self.state.last_decompiled_spirv_words = decompiled.spirv_words.len();
                    self.state.last_decompiled_glsl = Some(decompiled.glsl_source);
                }
            }
            // 0x04B0: Enable/Disable 32-bit Floating-Point Depth Testing (Z-Buffer)
            METHOD_SET_DEPTH_TEST_ENABLE => {
                let enable = cmd.args.first().copied().unwrap_or(0) != 0;
                self.state.depth_test_enabled = enable;
                if enable {
                    let needed = (self.state.viewport_width * self.state.viewport_height) as usize;
                    if self.state.depth_buffer.len() != needed {
                        self.state.depth_buffer.resize(needed, 1.0);
                    }
                }
            }
            // 0x0674: Clear Render Target Surface in VRAM (and Z-Buffer to 1.0)
            METHOD_CLEAR_SURFACE => {
                self.rasterize_clear(mmu);
            }
            // 0x0700: Set Vertex Array Address in Guest Memory
            METHOD_SET_VERTEX_ARRAY_ADDR => {
                if cmd.args.len() >= 2 {
                    self.state.vertex_array_addr =
                        ((cmd.args[1] as u64) << 32) | (cmd.args[0] as u64);
                } else if !cmd.args.is_empty() {
                    self.state.vertex_array_addr = cmd.args[0] as u64;
                }
            }
            // 0x0500: Bind 2D RGBA8888 Texture in Guest Memory (addr, width, height)
            METHOD_BIND_TEXTURE_2D => {
                if cmd.args.len() >= 3 {
                    self.state.texture_addr = cmd.args[0] as u64;
                    self.state.texture_width = cmd.args[1];
                    self.state.texture_height = cmd.args[2];
                }
            }
            // 0x0585: Draw Arrays (Fetches 6-float vertices [x,y,z,r,g,b] and rasterizes 3D triangles)
            METHOD_DRAW_ARRAYS => {
                self.state.total_draw_calls += 1;
                let vertex_count = cmd.args.first().copied().unwrap_or(0) as usize;
                if vertex_count >= 3 {
                    self.rasterize_triangles(mmu, vertex_count, false);
                }
            }
            // 0x0586: Draw Textured Arrays (Fetches 8-float vertices [x,y,z,r,g,b,u,v] with 2D texture sampling)
            METHOD_DRAW_TEXTURED_ARRAYS => {
                self.state.total_draw_calls += 1;
                let vertex_count = cmd.args.first().copied().unwrap_or(0) as usize;
                if vertex_count >= 3 {
                    self.rasterize_triangles(mmu, vertex_count, true);
                }
            }
            // 0x0900: Dispatch Ampere Tensor Core / DLSS 2x Super-Resolution Upscaler
            // Args: [src_addr, src_w, src_h, dst_addr]
            METHOD_DISPATCH_DLSS_UPSCALE => {
                if cmd.args.len() >= 4 {
                    let src_addr = cmd.args[0] as u64;
                    let src_w = cmd.args[1];
                    let src_h = cmd.args[2];
                    let dst_addr = cmd.args[3] as u64;
                    self.dlss.upscale_2x(mmu, src_addr, src_w, src_h, dst_addr);
                }
            }
            _ => {}
        }
    }

    /// Fills the entire active render target in VRAM with the configured clear color and resets Z-Buffer.
    fn rasterize_clear(&mut self, mmu: &mut Memory) {
        let w = self.state.viewport_width as usize;
        let h = self.state.viewport_height as usize;
        if w == 0 || h == 0 || self.state.render_target_addr == 0 {
            return;
        }

        if self.state.depth_test_enabled {
            let needed = w * h;
            if self.state.depth_buffer.len() != needed {
                self.state.depth_buffer.resize(needed, 1.0);
            } else {
                self.state.depth_buffer.fill(1.0);
            }
        }

        let r = (self.state.clear_color[0].clamp(0.0, 1.0) * 255.0) as u8;
        let g = (self.state.clear_color[1].clamp(0.0, 1.0) * 255.0) as u8;
        let b = (self.state.clear_color[2].clamp(0.0, 1.0) * 255.0) as u8;
        let a = (self.state.clear_color[3].clamp(0.0, 1.0) * 255.0) as u8;

        let mut row_bytes = vec![0u8; w * 4];
        for px in 0..w {
            row_bytes[px * 4] = r;
            row_bytes[px * 4 + 1] = g;
            row_bytes[px * 4 + 2] = b;
            row_bytes[px * 4 + 3] = a;
        }

        let stride = (w * 4) as u64;
        for y in 0..h {
            let row_addr = self.state.render_target_addr + (y as u64) * stride;
            let _ = mmu.write_bytes(row_addr, &row_bytes);
        }
    }

    /// Fetches vertices from guest memory and rasterizes shaded (and optionally 2D-textured) triangles into VRAM.
    fn rasterize_triangles(&mut self, mmu: &mut Memory, vertex_count: usize, textured: bool) {
        let w = self.state.viewport_width as f32;
        let h = self.state.viewport_height as f32;
        if w <= 0.0 || h <= 0.0 || self.state.vertex_array_addr == 0 {
            return;
        }

        let stride_bytes = if textured { 32 } else { 24 };
        let num_tris = vertex_count / 3;
        for t in 0..num_tris {
            let mut verts = [Vertex {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                r: 1.0,
                g: 1.0,
                b: 1.0,
                u: 0.0,
                v: 0.0,
            }; 3];

            for (i, v) in verts.iter_mut().enumerate() {
                let v_addr = self.state.vertex_array_addr + ((t * 3 + i) * stride_bytes) as u64;
                let px = f32::from_bits(mmu.read_u32(v_addr).unwrap_or(0));
                let py = f32::from_bits(mmu.read_u32(v_addr + 4).unwrap_or(0));
                let pz = f32::from_bits(mmu.read_u32(v_addr + 8).unwrap_or(0));
                v.r = f32::from_bits(mmu.read_u32(v_addr + 12).unwrap_or(0));
                v.g = f32::from_bits(mmu.read_u32(v_addr + 16).unwrap_or(0));
                v.b = f32::from_bits(mmu.read_u32(v_addr + 20).unwrap_or(0));
                if textured {
                    v.u = f32::from_bits(mmu.read_u32(v_addr + 24).unwrap_or(0));
                    v.v = f32::from_bits(mmu.read_u32(v_addr + 28).unwrap_or(0));
                }

                // Vertex Shader Stage: gl_Position = u_mvp * vec4(px, py, pz, 1.0)
                let m = &self.state.mvp_matrix;
                let cx = m[0] * px + m[1] * py + m[2] * pz + m[3];
                let cy = m[4] * px + m[5] * py + m[6] * pz + m[7];
                let cz = m[8] * px + m[9] * py + m[10] * pz + m[11];
                let cw = (m[12] * px + m[13] * py + m[14] * pz + m[15]).max(1e-4);

                v.x = cx / cw;
                v.y = cy / cw;
                v.z = cz / cw;
            }

            self.draw_single_triangle(mmu, &verts[0], &verts[1], &verts[2], textured);
            self.state.total_triangles_rendered += 1;
        }
    }

    /// Barycentric 2D/3D triangle rasterizer with smooth RGB vertex color and 2D texture UV sampling.
    fn draw_single_triangle(
        &mut self,
        mmu: &mut Memory,
        v0: &Vertex,
        v1: &Vertex,
        v2: &Vertex,
        textured: bool,
    ) {
        let w = self.state.viewport_width as i32;
        let h = self.state.viewport_height as i32;
        let wf = w as f32;
        let hf = h as f32;

        // Viewport transformation: NDC [-1.0, 1.0] -> Screen space [0, W] x [0, H]
        let sx0 = (v0.x + 1.0) * 0.5 * wf;
        let sy0 = (1.0 - v0.y) * 0.5 * hf;
        let sx1 = (v1.x + 1.0) * 0.5 * wf;
        let sy1 = (1.0 - v1.y) * 0.5 * hf;
        let sx2 = (v2.x + 1.0) * 0.5 * wf;
        let sy2 = (1.0 - v2.y) * 0.5 * hf;

        // Bounding box clipped to viewport
        let min_x = (sx0.min(sx1).min(sx2).floor() as i32).max(0);
        let max_x = (sx0.max(sx1).max(sx2).ceil() as i32).min(w - 1);
        let min_y = (sy0.min(sy1).min(sy2).floor() as i32).max(0);
        let max_y = (sy0.max(sy1).max(sy2).ceil() as i32).min(h - 1);

        let area = edge_fn(sx0, sy0, sx1, sy1, sx2, sy2);
        if area.abs() < 1e-5 {
            return;
        }
        let inv_area = 1.0 / area;

        let has_tex = textured
            && self.state.texture_addr != 0
            && self.state.texture_width > 0
            && self.state.texture_height > 0;

        for py in min_y..=max_y {
            let fy = py as f32 + 0.5;
            for px in min_x..=max_x {
                let fx = px as f32 + 0.5;

                let w0 = edge_fn(sx1, sy1, sx2, sy2, fx, fy) * inv_area;
                let w1 = edge_fn(sx2, sy2, sx0, sy0, fx, fy) * inv_area;
                let w2 = edge_fn(sx0, sy0, sx1, sy1, fx, fy) * inv_area;

                if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                    let pixel_idx = (py * w + px) as usize;
                    if self.state.depth_test_enabled {
                        let needed = (w * h) as usize;
                        if self.state.depth_buffer.len() != needed {
                            self.state.depth_buffer.resize(needed, 1.0);
                        }
                        let frag_z = w0 * v0.z + w1 * v1.z + w2 * v2.z;
                        if frag_z < self.state.depth_buffer[pixel_idx] {
                            self.state.depth_buffer[pixel_idx] = frag_z;
                            self.state.total_depth_tests_passed += 1;
                        } else {
                            self.state.total_depth_tests_rejected += 1;
                            continue;
                        }
                    }

                    let mut r_f = (w0 * v0.r + w1 * v1.r + w2 * v2.r).clamp(0.0, 1.0);
                    let mut g_f = (w0 * v0.g + w1 * v1.g + w2 * v2.g).clamp(0.0, 1.0);
                    let mut b_f = (w0 * v0.b + w1 * v1.b + w2 * v2.b).clamp(0.0, 1.0);

                    if has_tex {
                        let u = (w0 * v0.u + w1 * v1.u + w2 * v2.u).clamp(0.0, 1.0);
                        let v = (w0 * v0.v + w1 * v1.v + w2 * v2.v).clamp(0.0, 1.0);
                        let tx = ((u * (self.state.texture_width as f32 - 1.0)).round() as u32)
                            .min(self.state.texture_width - 1);
                        let ty = ((v * (self.state.texture_height as f32 - 1.0)).round() as u32)
                            .min(self.state.texture_height - 1);
                        let texel_addr = self.state.texture_addr
                            + ((ty * self.state.texture_width + tx) as u64) * 4;
                        if let Ok(texel) = mmu.read_u32(texel_addr) {
                            let tr = (texel & 0xFF) as f32 / 255.0;
                            let tg = ((texel >> 8) & 0xFF) as f32 / 255.0;
                            let tb = ((texel >> 16) & 0xFF) as f32 / 255.0;
                            r_f *= tr;
                            g_f *= tg;
                            b_f *= tb;
                            self.state.total_texels_sampled += 1;
                        }
                    }

                    let r = (r_f * 255.0).round() as u32;
                    let g = (g_f * 255.0).round() as u32;
                    let b = (b_f * 255.0).round() as u32;
                    let rgba = r | (g << 8) | (b << 16) | 0xFF000000;

                    let pixel_addr =
                        self.state.render_target_addr + ((py * w + px) as u64) * 4;
                    let _ = mmu.write_u32(pixel_addr, rgba);
                }
            }
        }
    }
}

#[inline(always)]
fn edge_fn(ax: f32, ay: f32, bx: f32, by: f32, cx: f32, cy: f32) -> f32 {
    (cx - ax) * (by - ay) - (cy - ay) * (bx - ax)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mmu::Permission;

    #[test]
    fn test_nvgpu_pushbuffer_clear_and_triangle() {
        let mut mmu = Memory::new();
        let mut gpu = NvgpuChannel::new();

        // Map tiny 16x16 render target at 0x90000000 and pushbuffer at 0x80010000
        let rt_addr = 0x90000000u64;
        let vb_addr = 0x80020000u64;
        let pb_addr = 0x80010000u64;
        mmu.map_range(rt_addr, 4096, Permission::READ_WRITE);
        mmu.map_range(vb_addr, 4096, Permission::READ_WRITE);
        mmu.map_range(pb_addr, 4096, Permission::READ_WRITE);

        // Write 3 vertices at vb_addr covering the center of the 16x16 viewport
        let verts: [f32; 18] = [
            0.0,  0.8, 0.0, 1.0, 0.0, 0.0, // Top (Red)
           -0.8, -0.8, 0.0, 0.0, 1.0, 0.0, // Bottom-Left (Green)
            0.8, -0.8, 0.0, 0.0, 0.0, 1.0, // Bottom-Right (Blue)
        ];
        for (i, &val) in verts.iter().enumerate() {
            mmu.write_u32(vb_addr + (i * 4) as u64, val.to_bits()).unwrap();
        }

        // Build GPFIFO pushbuffer commands
        let pb_words: Vec<u32> = vec![
            NvgpuChannel::encode_header(SubmissionMode::Increasing, 2, METHOD_SET_RENDER_TARGET_ADDR, 1),
            rt_addr as u32,
            NvgpuChannel::encode_header(SubmissionMode::Increasing, 2, METHOD_SET_VIEWPORT_DIMS, 2),
            16, 16,
            NvgpuChannel::encode_header(SubmissionMode::Increasing, 2, METHOD_CLEAR_SURFACE, 1),
            1,
            NvgpuChannel::encode_header(SubmissionMode::Increasing, 2, METHOD_SET_VERTEX_ARRAY_ADDR, 1),
            vb_addr as u32,
            NvgpuChannel::encode_header(SubmissionMode::Increasing, 2, METHOD_DRAW_ARRAYS, 1),
            3, // 3 vertices = 1 triangle
        ];

        for (i, &w) in pb_words.iter().enumerate() {
            mmu.write_u32(pb_addr + (i * 4) as u64, w).unwrap();
        }

        gpu.process_pushbuffer(&mut mmu, pb_addr, pb_words.len());

        assert_eq!(gpu.state.total_draw_calls, 1);
        assert_eq!(gpu.state.total_triangles_rendered, 1);

        // Center pixel (8, 8) should have non-zero interpolated RGB color!
        let center_px = mmu.read_u32(rt_addr + (8 * 16 + 8) * 4).unwrap();
        assert_ne!(center_px & 0x00FFFFFF, 0);
    }

    #[test]
    fn test_nvgpu_textured_triangle_sampling() {
        let mut mmu = Memory::new();
        let mut gpu = NvgpuChannel::new();

        let rt_addr = 0x90000000u64;
        let vb_addr = 0x80020000u64;
        let tex_addr = 0x80030000u64;
        let pb_addr = 0x80010000u64;
        mmu.map_range(rt_addr, 4096, Permission::READ_WRITE);
        mmu.map_range(vb_addr, 4096, Permission::READ_WRITE);
        mmu.map_range(tex_addr, 4096, Permission::READ_WRITE);
        mmu.map_range(pb_addr, 4096, Permission::READ_WRITE);

        // Write a 2x2 RGBA8888 texture with solid bright cyan/gold texels (0xFF40C0FF)
        for i in 0..4 {
            mmu.write_u32(tex_addr + i * 4, 0xFF40C0FF).unwrap();
        }

        // Write 3 textured vertices (8 floats each: x, y, z, r, g, b, u, v)
        let verts: [f32; 24] = [
             0.0,  0.8, 0.0, 1.0, 1.0, 1.0, 0.5, 0.0,
            -0.8, -0.8, 0.0, 1.0, 1.0, 1.0, 0.0, 1.0,
             0.8, -0.8, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0,
        ];
        for (i, &val) in verts.iter().enumerate() {
            mmu.write_u32(vb_addr + (i * 4) as u64, val.to_bits()).unwrap();
        }

        let pb_words: Vec<u32> = vec![
            NvgpuChannel::encode_header(SubmissionMode::Increasing, 2, METHOD_SET_RENDER_TARGET_ADDR, 1),
            rt_addr as u32,
            NvgpuChannel::encode_header(SubmissionMode::Increasing, 2, METHOD_SET_VIEWPORT_DIMS, 2),
            16, 16,
            NvgpuChannel::encode_header(SubmissionMode::Increasing, 2, METHOD_BIND_TEXTURE_2D, 3),
            tex_addr as u32, 2, 2,
            NvgpuChannel::encode_header(SubmissionMode::Increasing, 2, METHOD_SET_VERTEX_ARRAY_ADDR, 1),
            vb_addr as u32,
            NvgpuChannel::encode_header(SubmissionMode::Increasing, 2, METHOD_DRAW_TEXTURED_ARRAYS, 1),
            3,
        ];
        for (i, &w) in pb_words.iter().enumerate() {
            mmu.write_u32(pb_addr + (i * 4) as u64, w).unwrap();
        }

        gpu.process_pushbuffer(&mut mmu, pb_addr, pb_words.len());
        assert!(gpu.state.total_texels_sampled > 0);
        let center_px = mmu.read_u32(rt_addr + (8 * 16 + 8) * 4).unwrap();
        assert_eq!(center_px, 0xFF40C0FF);
    }

    #[test]
    fn test_nvgpu_zbuffer_depth_occlusion() {
        let mut mmu = Memory::new();
        let mut gpu = NvgpuChannel::new();

        let rt_addr = 0x90000000u64;
        let vb_addr = 0x80020000u64;
        let pb_addr = 0x80010000u64;
        mmu.map_range(rt_addr, 4096, Permission::READ_WRITE);
        mmu.map_range(vb_addr, 4096, Permission::READ_WRITE);
        mmu.map_range(pb_addr, 4096, Permission::READ_WRITE);

        // Two overlapping triangles:
        // Triangle 0 (Near, z = 0.2): Pure Green (r=0, g=1, b=0)
        // Triangle 1 (Far,  z = 0.8): Pure Red   (r=1, g=0, b=0) — drawn SECOND!
        let verts: [f32; 36] = [
            // Near Green Triangle (z = 0.2)
             0.0,  0.8, 0.2, 0.0, 1.0, 0.0,
            -0.8, -0.8, 0.2, 0.0, 1.0, 0.0,
             0.8, -0.8, 0.2, 0.0, 1.0, 0.0,
            // Far Red Triangle (z = 0.8)
             0.0,  0.8, 0.8, 1.0, 0.0, 0.0,
            -0.8, -0.8, 0.8, 1.0, 0.0, 0.0,
             0.8, -0.8, 0.8, 1.0, 0.0, 0.0,
        ];
        for (i, &val) in verts.iter().enumerate() {
            mmu.write_u32(vb_addr + (i * 4) as u64, val.to_bits()).unwrap();
        }

        let pb_words: Vec<u32> = vec![
            NvgpuChannel::encode_header(SubmissionMode::Increasing, 2, METHOD_SET_RENDER_TARGET_ADDR, 1),
            rt_addr as u32,
            NvgpuChannel::encode_header(SubmissionMode::Increasing, 2, METHOD_SET_VIEWPORT_DIMS, 2),
            16, 16,
            NvgpuChannel::encode_header(SubmissionMode::Increasing, 2, METHOD_SET_DEPTH_TEST_ENABLE, 1),
            1,
            NvgpuChannel::encode_header(SubmissionMode::Increasing, 2, METHOD_CLEAR_SURFACE, 1),
            1,
            NvgpuChannel::encode_header(SubmissionMode::Increasing, 2, METHOD_SET_VERTEX_ARRAY_ADDR, 1),
            vb_addr as u32,
            NvgpuChannel::encode_header(SubmissionMode::Increasing, 2, METHOD_DRAW_ARRAYS, 1),
            6, // 2 triangles (6 vertices)
        ];
        for (i, &w) in pb_words.iter().enumerate() {
            mmu.write_u32(pb_addr + (i * 4) as u64, w).unwrap();
        }

        gpu.process_pushbuffer(&mut mmu, pb_addr, pb_words.len());
        assert!(gpu.state.total_depth_tests_passed > 0);
        assert!(gpu.state.total_depth_tests_rejected > 0);

        // Because the near green triangle (z=0.2) is in front of the far red triangle (z=0.8),
        // the center pixel (8, 8) MUST remain Pure Green (0xFF00FF00)!
        let center_px = mmu.read_u32(rt_addr + (8 * 16 + 8) * 4).unwrap();
        assert_eq!(center_px, 0xFF00FF00);
    }
}
