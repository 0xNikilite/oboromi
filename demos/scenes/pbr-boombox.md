# pbr-boombox

The BoomBox (or Avocado, same layout) spinning under one directional light, shaded by `shaders/pbr.vert` + `shaders/pbr.frag`
into a 1280x720 RGBA8 target with a 32-bit depth buffer. This file is also the reference for the setup the other scenes reuse.

Values tagged `(verify)` rest on a convention not yet confirmed on hardware or by the rig's compiled output.

## Conventions shared by every scene

- Pushbuffer words follow clc76f.h: header `SEC_OP << 29 | COUNT << 16 | SUBCHANNEL << 13 | (byte offset >> 2)`,
  SEC_OP 1 incrementing, 3 non-incrementing, 4 immediate (`COUNT` = 13-bit value), 5 increment-once.
  Methods listed here in order can be grouped into incrementing packets where offsets are consecutive.
- Subchannel 0 carries the 3D class (0xC797), subchannel 1 the compute class (0xC7C0).
- Shader programs come from the rig: GLSL -> NVIDIA driver -> SM86 binary. Each graphics program is uploaded as its
  32-word SPHV4 header followed by the code; `SET_PIPELINE_PROGRAM_ADDRESS` points at the header and the code starts
  at +0x80 (verify; NAK `sph.rs` uses a 32-word header for SM >= 73, older docs say 0x50).
  Programs are 256-byte aligned. Compute programs have no SPH; the QMD points at the code.
- Constant buffer slots per stage (verify, read the real banks off the compiled LDC/TEX operands): slot 0 driver
  constants the rig dump shows, slot 1 the GLSL `binding = 1` uniform block, slot 2 the texture handle words.
  Slot s lands in set-0 binding `CBUF_BINDING + s` (core/src/gpu/sm86/access.rs:46).
- A texture handle word is `TIC index | TSC index << 20` (core/src/gpu/texture.rs). The TEX instruction names the
  bank and byte offset holding it; the placeholder layout puts sampler binding `2 + k` at c2[4k] (verify).
- Viewport and clip follow Vulkan: the host builds projections with y down and z in 0..1, the window origin is upper left.
- All targets are pitch-linear A8B8G8R8 (format 0xD5, memory layout PITCH, 0x1000). For a pitch target the hardware
  reads `SET_COLOR_TARGET_WIDTH` as the pitch in bytes (verify); threed.rs currently stores it as the pixel width, so
  the backend divides by 4 for format 0xD5 until that is reconciled.

## Guest memory map

| VA | size | contents |
|---|---|---|
| 0x1000_0000 | 0x1_0000 | pushbuffer: init segment at +0x0000, per-frame segment at +0x8000, GPFIFO ring at +0xF000 |
| 0x1001_0000 | 0x1_0000 | shader heap: pbr.vert program at +0x0000, pbr.frag program at +0x8000 |
| 0x1002_0000 | 0x1_0000 | cbufs: VS c0 +0x0000, VS c1 +0x1000 (128 B), PS c0 +0x2000, PS c1 +0x3000 (112 B), PS c2 handles +0x4000 |
| 0x1003_0000 | 0x1_0000 | TIC pool +0x0000 (4 entries of 32 B), TSC pool +0x8000 (1 entry) |
| 0x1004_0000 | 0x3_0000 | boombox/vertices.bin, 171600 B, stride 48 |
| 0x1007_0000 | 0x2_0000 | boombox/indices.bin, 72432 B, 18108 u32 |
| 0x1010_0000 | 0x6_0000 | baseColor.rgba8 (mip 0 at +0, 256x256, pitch 1024; mips 1-8 follow tightly) |
| 0x1016_0000 | 0x6_0000 | metallicRoughness.rgba8 (also the occlusion map: glTF reads occlusion from R) |
| 0x101C_0000 | 0x6_0000 | normal.rgba8 |
| 0x1022_0000 | 0x6_0000 | emissive.rgba8 (Avocado has none: point TIC 3 at baseColor, emissive factor is 0) |
| 0x1100_0000 | 0x39_0000 | colour target, 1280x720 A8B8G8R8 pitch 5120 (0x38_4000 B), read back to `display::present` |
| 0x1140_0000 | 0x39_0000 | depth target, ZF32 1280x720 |
| 0x1180_0000 | 0x1_0000 | report / semaphore page |

## Texture pools

TIC entries (8 words, word 0 first). Word 0 = components A8B8G8R8 (8), UNORM per channel, swizzle RGBA (verify: TIC
format code and data type values from clc797tex.h, never checked against a rig-written header). Word 2 = header
version PITCH (2 << 21). Word 3 = pitch >> 5. Word 4 = width-1, TWO_D (1 << 23). Word 5 = height-1, normalized coords.

| TIC | texture | w0 | w1 | w2 | w3 | w4 | w5 | w6 | w7 |
|---|---|---|---|---|---|---|---|---|---|
| 0 | baseColor | 0x58d24908 | 0x10100000 | 0x00400000 | 0x00000020 | 0x008000ff | 0x800000ff | 0 | 0 |
| 1 | metallicRoughness / occlusion | 0x58d24908 | 0x10160000 | 0x00400000 | 0x00000020 | 0x008000ff | 0x800000ff | 0 | 0 |
| 2 | normal | 0x58d24908 | 0x101c0000 | 0x00400000 | 0x00000020 | 0x008000ff | 0x800000ff | 0 | 0 |
| 3 | emissive | 0x58d24908 | 0x10220000 | 0x00400000 | 0x00000020 | 0x008000ff | 0x800000ff | 0 | 0 |

These headers describe mip 0 only, which is what texture.rs decodes today. The full chain needs `MAX_MIP_LEVEL` = 8 in
w3 bits 31:28 (w3 = 0x80000020), a mip filter in the sampler, and texture.rs accepting levels; on hardware a mipmapped
texture is block-linear, so the chain in the asset files would also need swizzling (verify).

TSC 0, repeat, linear min and mag, no mip: w0 = 0x00000000, w1 = 0x00000062 (mag 2, min 2 << 4, mip NONE 1 << 6),
w2-w7 = 0. With mips: w1 = 0x000000e2 (MIP_LINEAR) and w2 = 0x00008000 (max LOD clamp 8.0 in 4.8 fixed point).

Handle words at PS c2 (verify): +0x0 = 0x00000000 (baseColor), +0x4 = 0x00000002 (normal), +0x8 = 0x00000001
(metallicRoughness), +0xc = 0x00000001 (occlusion), +0x10 = 0x00000003 (emissive).

## Init program (subchannel 0, once)

| # | method | offset | value | meaning |
|---|---|---|---|---|
| 1 | SET_OBJECT | 0x0000 | 0x0000c797 | bind AMPERE_B 3D to subchannel 0 |
| 2 | SET_SURFACE_CLIP_HORIZONTAL / VERTICAL | 0x0ff4 / 0x0ff8 | 0x05000000 / 0x02d00000 | surface 1280 x 720 at 0,0 |
| 3 | SET_COLOR_TARGET_A / B (0) | 0x0800 / 0x0804 | 0x00000000 / 0x11000000 | CT0 address |
| 4 | SET_COLOR_TARGET_WIDTH (0) | 0x0808 | 0x00001400 | pitch 5120 bytes (verify, see conventions) |
| 5 | SET_COLOR_TARGET_HEIGHT (0) | 0x080c | 0x000002d0 | 720 rows |
| 6 | SET_COLOR_TARGET_FORMAT (0) | 0x0810 | 0x000000d5 | A8B8G8R8 |
| 7 | SET_COLOR_TARGET_MEMORY (0) | 0x0814 | 0x00001000 | LAYOUT PITCH |
| 8 | SET_COLOR_TARGET_THIRD_DIMENSION (0) | 0x0818 | 0x00000001 | one layer |
| 9 | SET_CT_SELECT | 0x121c | 0x00000001 | one target, slot 0 -> CT0 |
| 10 | SET_CT_WRITE (0) | 0x1a00 | 0x00001111 | write R, G, B, A |
| 11 | SET_ZT_A / B | 0x0fe0 / 0x0fe4 | 0x00000000 / 0x11400000 | depth address |
| 12 | SET_ZT_FORMAT | 0x0fe8 | 0x0000000a | ZF32 |
| 13 | SET_ZT_BLOCK_SIZE | 0x0fec | 0x00000000 | one-GOB blocks; depth has no pitch layout, the backend keeps it host-only (verify) |
| 14 | SET_ZT_ARRAY_PITCH | 0x0ff0 | 0x00384000 | one layer |
| 15 | SET_ZT_SIZE_A / B / C | 0x1228 / 0x122c / 0x1230 | 0x00000500 / 0x000002d0 / 0x00010001 | 1280 x 720, array size one |
| 16 | SET_ZT_SELECT | 0x1538 | 0x00000001 | depth target on |
| 17 | SET_VIEWPORT_SCALE_X / Y / Z (0) | 0x0a00 / 0x0a04 / 0x0a08 | 0x44200000 / 0x43b40000 / 0x3f800000 | 640.0, 360.0, 1.0 |
| 18 | SET_VIEWPORT_OFFSET_X / Y / Z (0) | 0x0a0c / 0x0a10 / 0x0a14 | 0x44200000 / 0x43b40000 / 0x00000000 | 640.0, 360.0, 0.0 |
| 19 | SET_VIEWPORT_CLIP_HORIZONTAL / VERTICAL (0) | 0x0c00 / 0x0c04 | 0x05000000 / 0x02d00000 | x 0 width 1280, y 0 height 720 |
| 20 | SET_VIEWPORT_CLIP_MIN_Z / MAX_Z (0) | 0x0c08 / 0x0c0c | 0x00000000 / 0x3f800000 | depth 0..1 |
| 21 | SET_VIEWPORT_Z_CLIP | 0x0d7c | 0x00000001 | ZERO_TO_POSITIVE_W (Vulkan clip space) |
| 22 | SET_WINDOW_ORIGIN | 0x13ac | 0x00000000 | upper left, no flip |
| 23 | SET_SCISSOR_ENABLE / HORIZONTAL / VERTICAL (0) | 0x0e00 / 0x0e04 / 0x0e08 | 0x00000001 / 0x05000000 / 0x02d00000 | full target |
| 24 | SET_CLEAR_RECT_HORIZONTAL / VERTICAL | 0x0d6c / 0x0d70 | 0x05000000 / 0x02d00000 | clear the whole target |
| 25 | SET_COLOR_CLEAR_VALUE (0..3) | 0x0d80-0x0d8c | 0x3ca3d70a 0x3ca3d70a 0x3cf5c28f 0x3f800000 | clear colour 0.02, 0.02, 0.03, 1 |
| 26 | SET_Z_CLEAR_VALUE | 0x0d90 | 0x3f800000 | clear depth 1.0 |
| 27 | OGL_SET_CULL | 0x1918 | 0x00000000 | no culling, so winding conventions do not matter |
| 28 | SET_DEPTH_TEST / SET_DEPTH_WRITE | 0x12cc / 0x12e8 | 0x00000001 / 0x00000001 | depth on |
| 29 | SET_DEPTH_FUNC | 0x130c | 0x00000201 | OGL_LESS |
| 30 | SET_BLEND_STATE_PER_TARGET / SET_BLEND (0) | 0x12e4 / 0x1360 | 0x00000000 / 0x00000000 | opaque |
| 31 | SET_VERTEX_STREAM_A_FORMAT (0) | 0x1c00 | 0x00001030 | stride 48, enabled |
| 32 | SET_VERTEX_STREAM_A_LOCATION_A / B (0) | 0x1c04 / 0x1c08 | 0x00000000 / 0x10040000 | vertices.bin |
| 33 | SET_VERTEX_STREAM_A_FREQUENCY (0) | 0x1c0c | 0x00000000 | per vertex |
| 34 | SET_VERTEX_STREAM_SIZE_A / B (0) | 0x0600 / 0x0604 | 0x00000000 / 0x00029e50 | 171600 bytes |
| 35 | SET_VERTEX_STREAM_INSTANCE_A (0) | 0x1880 | 0x00000000 | not instanced |
| 36 | SET_VERTEX_ATTRIBUTE_A (0) | 0x1160 | 0x38400000 | stream 0, +0, R32_G32_B32, FLOAT: position -> a[0x80] |
| 37 | SET_VERTEX_ATTRIBUTE_A (1) | 0x1164 | 0x38400600 | +12, R32_G32_B32, FLOAT: normal -> a[0x90] |
| 38 | SET_VERTEX_ATTRIBUTE_A (2) | 0x1168 | 0x38200c00 | +24, R32_G32_B32_A32, FLOAT: tangent -> a[0xa0] |
| 39 | SET_VERTEX_ATTRIBUTE_A (3) | 0x116c | 0x38801400 | +40, R32_G32, FLOAT: uv -> a[0xb0] |
| 40 | SET_VERTEX_ATTRIBUTE_A (4..15), _B (16..31) | 0x1170-0x119c, 0x11a0-0x11dc | 0x00000040 | SOURCE INACTIVE |
| 41 | SET_INDEX_BUFFER_A / B | 0x17c8 / 0x17cc | 0x00000000 / 0x10070000 | indices.bin |
| 42 | SET_INDEX_BUFFER_E | 0x17d8 | 0x00000002 | FOUR_BYTES |
| 43 | SET_PIPELINE_SHADER (0) | 0x2000 | 0x00000000 | VERTEX_CULL_BEFORE_FETCH off |
| 44 | SET_PIPELINE_SHADER (1) | 0x2040 | 0x00000011 | enable, type VERTEX |
| 45 | SET_PIPELINE_REGISTER_COUNT (1) | 0x204c | from the rig (verify) | VS GPR count, matches the SPH |
| 46 | SET_PIPELINE_BINDING (1) | 0x2050 | 0x00000000 | VS uses bind group 0 |
| 47 | SET_PIPELINE_PROGRAM_ADDRESS_A / B (1) | 0x2054 / 0x2058 | 0x00000000 / 0x10010000 | pbr.vert SPH, code at +0x80 (verify) |
| 48 | SET_PIPELINE_SHADER (2), (3), (4) | 0x2080 / 0x20c0 / 0x2100 | 0x00000020 / 0x00000030 / 0x00000040 | tessellation and geometry off |
| 49 | SET_PIPELINE_SHADER (5) | 0x2140 | 0x00000051 | enable, type PIXEL |
| 50 | SET_PIPELINE_REGISTER_COUNT (5) | 0x214c | from the rig (verify) | PS GPR count |
| 51 | SET_PIPELINE_BINDING (5) | 0x2150 | 0x00000004 | PS uses bind group 4 |
| 52 | SET_PIPELINE_PROGRAM_ADDRESS_A / B (5) | 0x2154 / 0x2158 | 0x00000000 / 0x10018000 | pbr.frag SPH, code at +0x80 (verify) |
| 53 | INVALIDATE_SHADER_CACHES | 0x021c | 0x00001011 | instruction, data and constant caches after the upload |
| 54 | SET_CONSTANT_BUFFER_SELECTOR_A / B / C | 0x2380 / 0x2384 / 0x2388 | 0x00001000 / 0x00000000 / 0x10020000 | select VS c0 (4 KiB, size from the rig dump) |
| 55 | BIND_GROUP_CONSTANT_BUFFER (0) | 0x2410 | 0x00000001 | group 0 slot 0 valid (verify slot) |
| 56 | SET_CONSTANT_BUFFER_SELECTOR_A / B / C | 0x2380-0x2388 | 0x00000100 / 0x00000000 / 0x10021000 | select VS uniforms |
| 57 | BIND_GROUP_CONSTANT_BUFFER (0) | 0x2410 | 0x00000011 | group 0 slot 1 (verify slot) |
| 58 | SET_CONSTANT_BUFFER_SELECTOR_A / B / C | 0x2380-0x2388 | 0x00001000 / 0x00000000 / 0x10022000 | select PS c0 |
| 59 | BIND_GROUP_CONSTANT_BUFFER (4) | 0x2490 | 0x00000001 | group 4 slot 0 (verify slot) |
| 60 | SET_CONSTANT_BUFFER_SELECTOR_A / B / C | 0x2380-0x2388 | 0x00000100 / 0x00000000 / 0x10023000 | select PS uniforms |
| 61 | BIND_GROUP_CONSTANT_BUFFER (4) | 0x2490 | 0x00000011 | group 4 slot 1 (verify slot) |
| 62 | SET_CONSTANT_BUFFER_SELECTOR_A / B / C | 0x2380-0x2388 | 0x00000100 / 0x00000000 / 0x10024000 | select PS handle words |
| 63 | BIND_GROUP_CONSTANT_BUFFER (4) | 0x2490 | 0x00000021 | group 4 slot 2 (verify slot) |
| 64 | SET_TEX_HEADER_POOL_A / B / C | 0x1574 / 0x1578 / 0x157c | 0x00000000 / 0x10030000 / 0x00000003 | TIC pool, max index 3 |
| 65 | SET_TEX_SAMPLER_POOL_A / B / C | 0x155c / 0x1560 / 0x1564 | 0x00000000 / 0x10038000 / 0x00000000 | TSC pool, max index 0 |
| 66 | SET_SAMPLER_BINDING | 0x1234 | 0x00000000 | INDEPENDENTLY: the TSC index comes from handle bits 31:20 |
| 67 | INVALIDATE_TEXTURE_HEADER_CACHE_NO_WFI / INVALIDATE_SAMPLER_CACHE_NO_WFI | 0x1428 / 0x1424 | 0x00000000 / 0x00000000 | drop cached pool entries |

The uniform buffers are filled by the per-frame segment below and once at init with the same methods.

## Uniforms

VS c1 (pbr.vert): words 0-15 `model` (column-major), words 16-31 `view_proj`. Init: view_proj = Vulkan perspective
(fov 45 deg, aspect 16:9, near 0.1, far 20, y down) * look-at from (0, 0.3, 3) to the origin.

PS c1 (pbr.frag): words 0-3 light_dir (0.4, 0.8, 0.45, 0) normalised, 4-7 light_colour (3, 2.9, 2.7, 0), 8-11 ambient
(0.08, 0.09, 0.11, 0), 12-15 camera_pos (0, 0.3, 3, 1), 16-19 base_colour_factor, 20-23 emissive_factor, 24-27 factors
(metallic, roughness, normalScale, occlusionStrength), the last three straight from manifest.json.

## Per-frame segment (subchannel 0)

| # | method | offset | value | meaning |
|---|---|---|---|---|
| 1 | SET_CONSTANT_BUFFER_SELECTOR_A / B / C | 0x2380-0x2388 | 0x00000100 / 0x00000000 / 0x10021000 | select VS uniforms |
| 2 | LOAD_CONSTANT_BUFFER_OFFSET | 0x238c | 0x00000000 | start at `model` |
| 3 | LOAD_CONSTANT_BUFFER (0..15), non-incrementing | 0x2390 | 16 words | model = rotate_y(t * 0.8 rad/s) * scale(100) (BoomBox, 0.02 units across; Avocado uses scale 30 and translate y -0.95) |
| 4 | CLEAR_SURFACE | 0x19d0 | 0x0000003d | clear Z and RGBA of CT0 |
| 5 | DRAW_INDEX_BUFFER32_BEGIN_END_INSTANCE_FIRST | 0x17e4 | 0x4fff0000 | triangles, 4095 indices from 0 |
| 6 | DRAW_INDEX_BUFFER32_BEGIN_END_INSTANCE_FIRST | 0x17e4 | 0x4fff0fff | 4095 from 4095 |
| 7 | DRAW_INDEX_BUFFER32_BEGIN_END_INSTANCE_FIRST | 0x17e4 | 0x4fff1ffe | 4095 from 8190 |
| 8 | DRAW_INDEX_BUFFER32_BEGIN_END_INSTANCE_FIRST | 0x17e4 | 0x4fff2ffd | 4095 from 12285 |
| 9 | DRAW_INDEX_BUFFER32_BEGIN_END_INSTANCE_FIRST | 0x17e4 | 0x46c03ffc | the last 1728 from 16380 |
| 10 | SET_REPORT_SEMAPHORE_A / B / C / D | 0x1b00-0x1b0c | 0x00000000 / 0x11800000 / frame / 0x1000f010 | one-word release of the frame number after all preceding writes, pipeline location ALL |

The encoded draw's COUNT field is 12 bits, so the 18108 indices go out as five draws of whole triangles. The single-draw
form is `BEGIN` (0x1618) = 0x00000004, `SET_INDEX_BUFFER_F` (0x17dc) = 0, `DRAW_INDEX_BUFFER` (0x17e0) = 18108,
`END` (0x1614) = 0; threed.rs does not decode `BEGIN` yet, so that form draws with a stale topology. Avocado fits in one
draw: 0x47fe0000 (2046 indices).

After the report the host reads CT0 back (pitch 5120) and hands it to `display::present(1280, 720, ..)`.

## What the pipeline must provide

Indexed u32 draws, four vertex attributes from one interleaved stream, a depth target (`SET_ZT_*`, depth write and
func; DrawSnapshot only carries `depth_test_enabled` today), five sampled textures through the TIC/TSC pools, two cbuf
slots per stage plus the handle bank, and full mip chains for the textures once block-linear TICs decode.
