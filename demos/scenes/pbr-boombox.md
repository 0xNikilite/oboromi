# pbr-boombox

The BoomBox (or any asset the converter writes, same layout) spinning under one directional light, shaded by the NVIDIA
driver's SM86 build of `shaders/pbr.vert` + `shaders/pbr.frag` into an RGBA8 target with a 32-bit depth buffer.
`core/src/gpu/demo.rs` builds it for any size; the tables below use 1280x720. This file is also the reference for the
setup the other scenes reuse.

Values tagged `(verify)` rest on a convention not yet confirmed on hardware or by the rig's compiled output.

## Conventions shared by every scene

- Pushbuffer words follow clc76f.h: header `SEC_OP << 29 | COUNT << 16 | SUBCHANNEL << 13 | (byte offset >> 2)`,
  SEC_OP 1 incrementing, 3 non-incrementing, 4 immediate (`COUNT` = 13-bit value), 5 increment-once.
  Methods listed here in order can be grouped into incrementing packets where offsets are consecutive.
- Subchannel 0 carries the 3D class (0xC797), subchannel 1 the compute class (0xC7C0).
- Shader programs come from the rig: GLSL -> NVIDIA driver -> SM86 binary (core/tests/fixtures/driver-shaders). Each
  graphics program is its 32-word SPHV4 header followed by the code, which starts at +0x80 (every extracted header is
  0x80 bytes). `SET_PIPELINE_PROGRAM_ADDRESS` points at the header. The extracted code stops at its last EXIT, so the
  host appends the branch to itself a compiled program ends with. Programs are 256-byte aligned. Compute programs
  have no SPH; the QMD points at the code.
- The driver reaches everything through bank 0x1d of each stage, a pointer table: the handle of texture k (GLSL
  declaration order) at `c[0x1d][4k]` and the address of the `binding = 1` uniform block at `c[0x1d][0x18]` for pbr
  (`TEX ... 0x1d, 0x4` and `ULDC.64 UR4, c[0x1d][0x18]` in the SASS). The block address is a bindless cbuf handle,
  `va >> 4` in bits 0..45 and `size >> 4` above (NVK nvk_bindless_cbuf); the shader reads it as `cx[UR4][offset]`.
  Whether NVN fills bank 0x1d the same way as the desktop driver is open (verify).
- A texture handle word is `TIC index | TSC index << 20` (core/src/gpu/texture.rs).
- Viewport and clip follow Vulkan: the host builds projections with y down and z in 0..1, the window origin is upper left.
- All targets are pitch-linear A8B8G8R8 (format 0xD5, memory layout PITCH, 0x1000). For a pitch target
  `SET_COLOR_TARGET_WIDTH` holds the pitch in bytes (Ryujinx RtColorState.WidthOrStride), as engine.rs reads it.

## Guest memory map

One contiguous mapping from 0x1000_0000, since the global window the uniform reads go through spans all mapped memory.

| VA | size | contents |
|---|---|---|
| 0x1000_0000 | 0x1_0000 | pushbuffer: init segment at +0x0000, per-frame segment at +0x8000 |
| 0x1001_0000 | 0x1_0000 | pbr.vert program at +0x0000, pbr.frag program at +0x8000 |
| 0x1002_0000 | 32 B | VS bank 0x1d: the VS uniform handle at +0x18 |
| 0x1002_0100 | 128 B | VS uniforms |
| 0x1002_1000 + 0x100 m | 32 B | PS bank 0x1d of material m: texture handles at +0x0..+0x10, its uniform handle at +0x18 |
| 0x1002_1080 + 0x100 m | 112 B | PS uniforms of material m |
| 0x1003_0000 | 32 B each | TIC pool, TIC i = `textures[i]` of manifest.json |
| 0x1003_8000 | 32 B | TSC pool, one sampler |
| 0x1003_f000 | 4 B | report |
| 0x1004_0000 | 171600 B | vertices.bin, stride 48 |
| 0x1007_0000 | 72432 B | indices.bin, 18108 u32 |
| 0x1009_0000 + 0x6_0000 i | 351232 B each | the texture files in manifest order: baseColor, metallicRoughness (also occlusion, glTF reads it from R), normal, emissive; 256x256 block-linear, gob height 2, 9 levels back to back |
| 0x1021_0000 | 0x38_4000 | colour target, 1280x720 A8B8G8R8 pitch 5120, read back to `display::present` |

Each region after the vertices starts on the next 64 KiB boundary. The depth target lives on the host only, so its VA
(0x2000_0000) needs no pages.

## Texture pools

TIC entries (8 words, word 0 first). Word 0 = components A8B8G8R8 (8), UNORM per channel, swizzle RGBA. Word 2 =
header version BLOCKLINEAR (3 << 21). Word 3 = gobs per block height 2 (1 << 3), MAX_MIP_LEVEL 8 (8 << 28, nine
levels). Word 4 = width-1, TWO_D (1 << 23). Word 5 = height-1, normalized coords. No sRGB bit: pbr.frag decodes sRGB
itself, so the bit would decode twice. Format and swizzle values are from clc797tex.h, never checked against a
rig-written header (verify).

| TIC | texture | w0 | w1 | w2 | w3 | w4 | w5 | w6 | w7 |
|---|---|---|---|---|---|---|---|---|---|
| 0 | baseColor | 0x58d24908 | 0x10090000 | 0x00600000 | 0x80000008 | 0x008000ff | 0x800000ff | 0 | 0 |
| 1 | metallicRoughness / occlusion | 0x58d24908 | 0x100f0000 | 0x00600000 | 0x80000008 | 0x008000ff | 0x800000ff | 0 | 0 |
| 2 | normal | 0x58d24908 | 0x10150000 | 0x00600000 | 0x80000008 | 0x008000ff | 0x800000ff | 0 | 0 |
| 3 | emissive | 0x58d24908 | 0x101b0000 | 0x00600000 | 0x80000008 | 0x008000ff | 0x800000ff | 0 | 0 |

TSC 0, repeat, trilinear: w0 = 0x00000000, w1 = 0x000000e2 (mag 2, min 2 << 4, MIP_LINEAR 3 << 6), w2 = 0x00f00000
(max LOD clamp 15.0 in 4.8 fixed point at bits 23:12), w3-w7 = 0.

PS bank 0x1d of the material: +0x0 = 0x00000000 (baseColor), +0x4 = 0x00000002 (normal), +0x8 = 0x00000001
(metallicRoughness), +0xc = 0x00000001 (occlusion), +0x10 = 0x00000003 (emissive), +0x18 / +0x1c = 0x01002108 /
0x0000e000 (uniforms at 0x1002_1080, 112 B). A missing slot points at the material's baseColor. VS bank 0x1d: +0x18 /
+0x1c = 0x01002010 / 0x00010000 (0x1002_0100, 128 B).

## Init program (subchannel 0, once)

| # | method | offset | value | meaning |
|---|---|---|---|---|
| 1 | SET_OBJECT | 0x0000 | 0x0000c797 | bind AMPERE_B 3D to subchannel 0 |
| 2 | SET_COLOR_TARGET_A / B (0) | 0x0800 / 0x0804 | 0x00000000 / 0x10210000 | CT0 address |
| 3 | SET_COLOR_TARGET_WIDTH / HEIGHT (0) | 0x0808 / 0x080c | 0x00001400 / 0x000002d0 | pitch 5120 bytes, 720 rows |
| 4 | SET_COLOR_TARGET_FORMAT / MEMORY (0) | 0x0810 / 0x0814 | 0x000000d5 / 0x00001000 | A8B8G8R8, LAYOUT PITCH |
| 5 | SET_CT_WRITE (0) | 0x1a00 | 0x00001111 | write R, G, B, A |
| 6 | SET_ZT_A / B / FORMAT | 0x0fe0 / 0x0fe4 / 0x0fe8 | 0x00000000 / 0x20000000 / 0x0000000a | depth address, ZF32 |
| 7 | SET_ZT_SELECT | 0x1538 | 0x00000001 | depth target on |
| 8 | SET_VIEWPORT_SCALE_X / Y / Z (0) | 0x0a00 / 0x0a04 / 0x0a08 | 0x44200000 / 0x43b40000 / 0x3f800000 | 640.0, 360.0, 1.0 |
| 9 | SET_VIEWPORT_OFFSET_X / Y / Z (0) | 0x0a0c / 0x0a10 / 0x0a14 | 0x44200000 / 0x43b40000 / 0x00000000 | 640.0, 360.0, 0.0 |
| 10 | SET_VIEWPORT_CLIP_HORIZONTAL / VERTICAL (0) | 0x0c00 / 0x0c04 | 0x05000000 / 0x02d00000 | x 0 width 1280, y 0 height 720 |
| 11 | SET_VIEWPORT_CLIP_MIN_Z / MAX_Z (0) | 0x0c08 / 0x0c0c | 0x00000000 / 0x3f800000 | depth 0..1 |
| 12 | SET_SCISSOR_ENABLE / HORIZONTAL / VERTICAL (0) | 0x0e00 / 0x0e04 / 0x0e08 | 0x00000001 / 0x05000000 / 0x02d00000 | full target |
| 13 | SET_COLOR_CLEAR_VALUE (0..3) | 0x0d80-0x0d8c | 0x3ca3d70a 0x3ca3d70a 0x3cf5c28f 0x3f800000 | clear colour 0.02, 0.02, 0.03, 1 |
| 14 | SET_Z_CLEAR_VALUE | 0x0d90 | 0x3f800000 | clear depth 1.0 |
| 15 | SET_DEPTH_TEST / SET_DEPTH_WRITE | 0x12cc / 0x12e8 | 0x00000001 / 0x00000001 | depth on |
| 16 | SET_DEPTH_FUNC | 0x130c | 0x00000201 | OGL_LESS |
| 17 | SET_VERTEX_STREAM_A_FORMAT / LOCATION_A / B (0) | 0x1c00 / 0x1c04 / 0x1c08 | 0x00001030 / 0x00000000 / 0x10040000 | stride 48, enabled, vertices.bin |
| 18 | SET_VERTEX_STREAM_SIZE_A / B (0) | 0x0600 / 0x0604 | 0x00000000 / 0x00029e50 | 171600 bytes |
| 19 | SET_VERTEX_ATTRIBUTE_A (0) | 0x1160 | 0x38400000 | stream 0, +0, R32_G32_B32, FLOAT: position -> a[0x80] |
| 20 | SET_VERTEX_ATTRIBUTE_A (1) | 0x1164 | 0x38400600 | +12, R32_G32_B32, FLOAT: normal -> a[0x90] |
| 21 | SET_VERTEX_ATTRIBUTE_A (2) | 0x1168 | 0x38200c00 | +24, R32_G32_B32_A32, FLOAT: tangent -> a[0xa0] |
| 22 | SET_VERTEX_ATTRIBUTE_A (3) | 0x116c | 0x38801400 | +40, R32_G32, FLOAT: uv -> a[0xb0] |
| 23 | SET_INDEX_BUFFER_A / B | 0x17c8 / 0x17cc | 0x00000000 / 0x10070000 | indices.bin |
| 24 | SET_INDEX_BUFFER_E | 0x17d8 | 0x00000002 | FOUR_BYTES |
| 25 | SET_PIPELINE_SHADER (1) | 0x2040 | 0x00000011 | enable, type VERTEX |
| 26 | SET_PIPELINE_BINDING / PROGRAM_ADDRESS_A / B (1) | 0x2050 / 0x2054 / 0x2058 | 0x00000000 / 0x00000000 / 0x10010000 | bind group 0, pbr.vert SPH |
| 27 | SET_PIPELINE_SHADER (5) | 0x2140 | 0x00000051 | enable, type PIXEL |
| 28 | SET_PIPELINE_BINDING / PROGRAM_ADDRESS_A / B (5) | 0x2150 / 0x2154 / 0x2158 | 0x00000004 / 0x00000000 / 0x10018000 | bind group 4, pbr.frag SPH |
| 29 | SET_CONSTANT_BUFFER_SELECTOR_A / B / C | 0x2380 / 0x2384 / 0x2388 | 0x00000020 / 0x00000000 / 0x10020000 | select the VS bank 0x1d table |
| 30 | BIND_GROUP_CONSTANT_BUFFER (0) | 0x2410 | 0x000001d1 | group 0 slot 0x1d valid |
| 31 | SET_TEX_HEADER_POOL_A / B / C | 0x1574 / 0x1578 / 0x157c | 0x00000000 / 0x10030000 / 0x00000003 | TIC pool, max index 3 |
| 32 | SET_TEX_SAMPLER_POOL_A / B / C | 0x155c / 0x1560 / 0x1564 | 0x00000000 / 0x10038000 / 0x00000000 | TSC pool, max index 0 |

Cull, blend and SET_SAMPLER_BINDING (INDEPENDENTLY: the TSC index comes from handle bits 31:20) stay at their reset
values: no culling, opaque.

## Uniforms

VS (pbr.vert): words 0-15 `model` (column-major), words 16-31 `view_proj` = Vulkan perspective (fov 45 deg, the
target's aspect, near 0.1, far 20, y down) * look-at from (0, 0.3, 3) to the origin.

PS (pbr.frag): words 0-3 light_dir (0.4, 0.8, 0.45, 0) normalised, 4-7 light_colour (3, 2.9, 2.7, 0), 8-11 ambient
(0.08, 0.09, 0.11, 0), 12-15 camera_pos (0, 0.3, 3, 1), 16-19 base_colour_factor, 20-23 emissive_factor, 24-27 factors
(metallic, roughness, normalScale, occlusionStrength), the last three from the material in manifest.json.

## Per-frame segment (subchannel 0)

| # | method | offset | value | meaning |
|---|---|---|---|---|
| 1 | SET_CONSTANT_BUFFER_SELECTOR_A / B / C, LOAD_CONSTANT_BUFFER_OFFSET | 0x2380-0x238c | 0x00000080 / 0x00000000 / 0x10020100 / 0 | select the VS uniforms |
| 2 | LOAD_CONSTANT_BUFFER, non-incrementing | 0x2390 | 32 words | model = rotate_y(t * 0.8 rad/s) * scale(2 / extent) * translate(-centre) from the vertex bounds (BoomBox: scale 99), then view_proj |
| 3 | CLEAR_SURFACE | 0x19d0 | 0x0000003d | clear Z and RGBA of CT0 |
| 4 | SET_CONSTANT_BUFFER_SELECTOR_A / B / C | 0x2380-0x2388 | 0x00000020 / 0x00000000 / 0x10021000 + 0x100 m | the draw's material table |
| 5 | BIND_GROUP_CONSTANT_BUFFER (4) | 0x2490 | 0x000001d1 | group 4 slot 0x1d |
| 6 | BEGIN | 0x1618 | 0x00000004 | triangles, first instance |
| 7 | SET_INDEX_BUFFER_F | 0x17dc | first_index | 0 |
| 8 | DRAW_INDEX_BUFFER | 0x17e0 | index_count | 18108 |
| 9 | END | 0x1614 | 0x00000000 | |
| 10 | SET_REPORT_SEMAPHORE_A / B / C / D | 0x1b00-0x1b0c | 0x00000000 / 0x1003f000 / frame / 0x10000000 | one-word release of the frame number after all preceding writes |

Rows 4-9 repeat for each primitive of manifest.json. After the report the host reads CT0 back and hands it to
`display::present`.
