# post-effect

Two passes per frame: pass 1 renders the pbr-boombox scene into an offscreen 1280x720 target, pass 2 draws one
full-screen triangle (`shaders/post.vert` + `shaders/post.frag`) that samples it with chromatic aberration, rolling
scanlines and a vignette into the presented target. Conventions, pass-1 state and `(verify)` meaning as in pbr-boombox.md.

## Guest memory map (additions to pbr-boombox.md)

| VA | size | contents |
|---|---|---|
| 0x1002_5000 | in the cbuf page | post uniforms, 32 B (`effect`, `size`) |
| 0x1002_6000 | in the cbuf page | post PS bank 0x1d: texture handle at +0x0, uniform handle at +0x8 |
| 0x1009_0000 | 0x1_0000 | shader heap 2: post.vert program at +0x0000, post.frag program at +0x8000 |
| 0x1100_0000 | 0x39_0000 | pass-1 colour target, sampled in pass 2 (TIC 4) |
| 0x1200_0000 | 0x39_0000 | pass-2 colour target, 1280x720 A8B8G8R8 pitch 5120, read back to `display::present` |

## Pools (additions)

TIC 4 is the pass-1 target: same address, pitch and size as its `SET_COLOR_TARGET_*`, so the render-to-texture link is
just that VA. Words: 0x58d24908 0x11000000 0x00400000 0x000000a0 0x008004ff 0x800002cf 0 0 (pitch 5120 >> 5 = 0xa0,
width-1 0x4ff, height-1 0x2cf; components A8B8G8R8 UNORM to match CT format 0xD5 (verify)).
TSC 1, clamp to edge, linear, no mip: 0x00000092 0x00000062 0 0 0 0 0 0. Pool limits grow to TIC max index 4
(`SET_TEX_HEADER_POOL_C` = 0x00000004) and TSC max index 1 (`SET_TEX_SAMPLER_POOL_C` = 0x00000001).
Bank 0x1d holds the texture handle at +0x0 (post.frag `TEX ... 0x1d, 0x0`): 0x00100004 (TIC 4, TSC 1), and at +0x8 the
uniforms' bindless cbuf handle (`ULDC.64 UR4, c[0x1d][0x8]`, then `cx[UR4][field]`): 0x01002500 0x00004000, address >> 4
with size >> 4 from bit 45 (NVK nvk_bindless_cbuf).

## Uniforms

Post PS c1: words 0-3 `effect` = (time s, 6.0 px aberration, 0.35 scanlines, 0.55 vignette) =
0x00000000 0x40c00000 0x3eb33333 0x3f0ccccd at init; words 4-7 `size` = (1280, 720, 1/1280, 1/720) =
0x44a00000 0x44340000 0x3a4ccccd 0x3ab60b61, written once.

## Per-frame program (subchannel 0)

Pass 1 is the pbr-boombox per-frame segment (rows 1-9, without the report). Pass 2 then:

| # | method | offset | value | meaning |
|---|---|---|---|---|
| 1 | WAIT_FOR_IDLE | 0x0110 | 0x00000000 | pass-1 colour writes complete before anything samples them |
| 2 | INVALIDATE_TEXTURE_DATA_CACHE | 0x1338 | 0x00000000 | drop texture lines that may hold stale target data |
| 3 | SET_COLOR_TARGET_B (0) | 0x0804 | 0x12000000 | CT0 moves to the presented target; width, height, format, memory unchanged |
| 4 | SET_ZT_SELECT | 0x1538 | 0x00000000 | no depth target |
| 5 | SET_DEPTH_TEST / SET_DEPTH_WRITE | 0x12cc / 0x12e8 | 0x00000000 / 0x00000000 | depth off |
| 6 | SET_VERTEX_STREAM_A_FORMAT (0) | 0x1c00 | 0x00000000 | stream 0 disabled |
| 7 | SET_VERTEX_ATTRIBUTE_A (0..3) | 0x1160-0x116c | 0x00000040 | all attributes inactive; corners come from a[0x2fc] |
| 8 | SET_PIPELINE_REGISTER_COUNT (1) | 0x204c | from the rig (verify) | post.vert GPRs |
| 9 | SET_PIPELINE_PROGRAM_ADDRESS_A / B (1) | 0x2054 / 0x2058 | 0x00000000 / 0x10090000 | post.vert SPH, code at +0x80 |
| 10 | SET_PIPELINE_REGISTER_COUNT (5) | 0x214c | from the rig (verify) | post.frag GPRs |
| 11 | SET_PIPELINE_PROGRAM_ADDRESS_A / B (5) | 0x2154 / 0x2158 | 0x00000000 / 0x10098000 | post.frag SPH, code at +0x80 |
| 12 | SET_CONSTANT_BUFFER_SELECTOR_A / B / C | 0x2380-0x2388 | 0x00000100 / 0x00000000 / 0x10025000 | select post uniforms |
| 13 | LOAD_CONSTANT_BUFFER_OFFSET | 0x238c | 0x00000000 | `effect.x` |
| 14 | LOAD_CONSTANT_BUFFER (0) | 0x2390 | time as f32 | the only word that changes per frame |
| 15 | SET_CONSTANT_BUFFER_SELECTOR_A / B / C | 0x2380-0x2388 | 0x00000010 / 0x00000000 / 0x10026000 | select the post bank 0x1d |
| 16 | BIND_GROUP_CONSTANT_BUFFER (4) | 0x2490 | 0x000001d1 | PS slot 0x1d = texture and uniform handles |
| 17 | DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_FIRST | 0x1214 | 0x40030000 | triangles, 3 vertices from 0 |
| 18 | SET_REPORT_SEMAPHORE_A / B / C / D | 0x1b00-0x1b0c | 0x00000000 / 0x11800000 / frame / 0x1000f010 | frame done |

The next frame first restores pass-1 state: `SET_COLOR_TARGET_B (0)` = 0x11000000, `SET_ZT_SELECT` = 1, depth test and
write = 1, `SET_VERTEX_STREAM_A_FORMAT (0)` = 0x00001030, attributes 0-3 as pbr-boombox rows 36-39, the pbr program
addresses and register counts (rows 45, 47, 50, 52), and PS slot 0x1d back to the pbr bank.
No clear is needed in pass 2, the triangle covers every pixel.

## Render-to-texture dependency

The pass-1 target VA 0x1100_0000 is the pass-2 TIC 4 address. On hardware the WFI plus texture-cache invalidate is the
whole barrier because both sides go through L2. In the emulator the draw at row 18 must see pass 1's pixels: either read
CT0 back into guest memory after pass 1 and let the texture path upload it from TIC 4 (works with today's
`HostImage::new` upload), or keep a VA -> image map so TIC 4 resolves to the pass-1 image itself, with a
COLOR_ATTACHMENT_OUTPUT -> FRAGMENT_SHADER barrier and a layout change to SHADER_READ_ONLY between the passes.
The pitch-target width caveat from the conventions applies to both targets.

## What the pipeline must provide

Two render passes per frame with a render-to-texture link by VA, a non-indexed draw with no vertex streams
(gl_VertexIndex), clamp-to-edge linear sampling, and the cbuf rebinding between passes.
