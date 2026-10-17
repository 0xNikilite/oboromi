# particles

8192 particles orbit two moving attractors. Each frame a compute dispatch (`shaders/particles_update.comp`, 32 CTAs of
256 threads) integrates the particle buffer in place, then one instanced draw (`shaders/particles.vert` +
`shaders/particles.frag`) reads the same buffer as a per-instance vertex stream and adds a soft glow quad per particle.
Conventions and `(verify)` meaning as in pbr-boombox.md.

## Guest memory map

| VA | size | contents |
|---|---|---|
| 0x1000_0000 | 0x1_0000 | pushbuffer (init +0x0000, per frame +0x8000), GPFIFO ring +0xF000 |
| 0x1001_0000 | 0x1_0000 | shader heap: particles.vert program +0x0000, particles.frag program +0x4000, particles_update.comp code +0x8000 (no SPH) |
| 0x1002_0000 | 0x1_0000 | cbufs: VS c0 +0x0000, VS c1 +0x1000 (96 B), PS c0 +0x2000, CS c0 +0x3000, CS c1 +0x4000 (64 B) |
| 0x1003_0000 | 0x1_0000 | QMD at +0x0000 (64 words, 256-byte aligned) |
| 0x1010_0000 | 0x6_0000 | particle buffer, 8192 x 48 B: vec4 position (xyz, w = age), vec4 velocity (xyz, w = 1/lifetime), vec4 colour |
| 0x1100_0000 | 0x39_0000 | colour target, 1280x720 A8B8G8R8 pitch 5120 |
| 0x1180_0000 | 0x1_0000 | +0x00 compute release semaphore, +0x10 frame report |

Initial particle buffer (host-written): positions uniform in a 0.5-radius ball around attractor A, age uniform in 0..1 so
respawns are staggered, velocity.w = 1/lifetime with lifetime 2-4 s, colour 0.

## Uniforms

CS c1 (particles_update.comp): words 0-3 attractor_a (xyz, w = strength 6.0), 4-7 attractor_b (strength 4.0), 8-11
step = (dt 1/60 = 0x3c888889, gravity 0.5 = 0x3f000000, damping 0.995 = 0x3f7eb852, time), 12 count = 8192 (0x2000),
13-15 unused. The SSBO's 64-bit address and size live wherever the compiled SASS loads them from (CS c0 at a driver
offset, verify): write 0x10100000 / 0x00060000 there.

VS c1 (particles.vert): words 0-15 view_proj (fov 60 deg from (0, 0.5, 4) at the origin), 16-19 camera_right (xyz,
w = quad half size 0.03 = 0x3cf5c28f), 20-23 camera_up. particles.frag reads no uniforms.

Per frame: CS c1 words 0-2 and 4-6 (A = (1.2 cos t, 0.4 sin 2t, 1.2 sin t), B = -A), word 11 (time). Nothing else.

## QMD (QMDV03_00, words not listed are 0)

| word | bits | field | value |
|---|---|---|---|
| 5 | 31:26 | INVALIDATE_TEXTURE_HEADER / SAMPLER / TEXTURE_DATA / SHADER_DATA / INSTRUCTION / SHADER_CONSTANT caches | 0xfc000000 on the first launch, then 0xa0000000 (shader data and constants, both change every frame) |
| 4 | 6 | SM_GLOBAL_CACHING_ENABLE | 1, as NAK sets for this QMD version (0x00000040) |
| 11 | 30 | SAMPLER_INDEX | 0, INDEPENDENTLY |
| 12 | 31:0 | CTA_RASTER_WIDTH | 32 (8192 / 256) |
| 13 | 15:0 | CTA_RASTER_HEIGHT | 1 |
| 14 | 15:0 | CTA_RASTER_DEPTH | 1 |
| 17 | 17:0, 23:18, 30:25 | SHARED_MEMORY_SIZE, MIN_ and MAX_SM_CONFIG_SHARED_MEM_SIZE | 0 shared; config sizes per NAK `gv100_get_hw_smem_sizes` (verify) |
| 18 | 3:0, 7:4 | QMD_VERSION, QMD_MAJOR_VERSION | 0, 3 (verify) |
| 18 | 31:16 | CTA_THREAD_DIMENSION0 | 256: word 18 = 0x01000030 |
| 19 | 15:0, 31:16 | CTA_THREAD_DIMENSION1, 2 | 1, 1: 0x00010001 |
| 20 | 7:0 | CONSTANT_BUFFER_VALID | 0x03, slots 0 and 1 (verify slots) |
| 20 | 16:8 | REGISTER_COUNT_V | from the rig (verify) |
| 20 | 22:17 | TARGET_SM_CONFIG_SHARED_MEM_SIZE | per NAK (verify) |
| 23 | 31:27 | BARRIER_COUNT | 0, the shader has no barrier() |
| 24 | 31:0 | RELEASE0_ADDRESS_LOWER | 0x11800000 |
| 25 | 7:0, 19, 23, 31:30 | RELEASE0 ADDRESS_UPPER, MEMBAR_TYPE, ENABLE, STRUCTURE_SIZE | 0, FE_SYSMEMBAR, 1, ONE_WORD: 0x40880000 |
| 26 | 31:0 | RELEASE0_PAYLOAD_LOWER | frame number, host-rewritten each frame |
| 32, 33 | 31:0; 16:0, 31:19 | CONSTANT_BUFFER_ADDR (0), SIZE_SHIFTED4 (0) | 0x10023000; 0x08000000 (4 KiB) |
| 34, 35 | same for slot 1 | CONSTANT_BUFFER_ADDR (1), SIZE_SHIFTED4 (1) | 0x10024000; 0x00800000 (256 B) |
| 48, 49 | 31:0; 16:0 | PROGRAM_ADDRESS_LOWER / UPPER | 0x10018000; 0 (code, no header) |

## Init program

Subchannel 0 (3D), as pbr-boombox rows 1-10, 17-27, 43-53 with these differences:

| # | method | offset | value | meaning |
|---|---|---|---|---|
| 1 | SET_ZT_SELECT / SET_DEPTH_TEST / SET_DEPTH_WRITE | 0x1538 / 0x12cc / 0x12e8 | 0 / 0 / 0 | no depth, order-independent additive glow |
| 2 | SET_COLOR_CLEAR_VALUE (0..3) | 0x0d80-0x0d8c | 0 0 0 0x3f800000 | black |
| 3 | SET_BLEND_STATE_PER_TARGET | 0x12e4 | 0x00000000 | one blend state for all targets |
| 4 | SET_BLEND_SEPARATE_FOR_ALPHA | 0x133c | 0x00000000 | alpha uses the colour equation |
| 5 | SET_BLEND_COLOR_OP / SOURCE_COEFF / DEST_COEFF | 0x1340 / 0x1344 / 0x1348 | 0x00008006 / 0x00004001 / 0x00004001 | OGL FUNC_ADD, ONE, ONE |
| 6 | SET_BLEND_ALPHA_OP / SOURCE_COEFF / DEST_COEFF | 0x134c / 0x1350 / 0x1358 | 0x00008006 / 0x00004001 / 0x00004001 | the same for alpha |
| 7 | SET_BLEND (0) | 0x1360 | 0x00000001 | blend on for CT0 |
| 8 | SET_VERTEX_STREAM_A_FORMAT (0) | 0x1c00 | 0x00001030 | stride 48, enabled |
| 9 | SET_VERTEX_STREAM_A_LOCATION_A / B (0) | 0x1c04 / 0x1c08 | 0x00000000 / 0x10100000 | the particle buffer |
| 10 | SET_VERTEX_STREAM_SIZE_A / B (0) | 0x0600 / 0x0604 | 0x00000000 / 0x00060000 | 393216 bytes |
| 11 | SET_VERTEX_STREAM_INSTANCE_A (0) | 0x1880 | 0x00000001 | advance per instance |
| 12 | SET_VERTEX_STREAM_A_FREQUENCY (0) | 0x1c0c | 0x00000001 | divisor 1 (verify: hardware divisor semantics) |
| 13 | SET_VERTEX_ATTRIBUTE_A (0) | 0x1160 | 0x38200000 | stream 0, +0, R32_G32_B32_A32 FLOAT: position -> a[0x80] |
| 14 | SET_VERTEX_ATTRIBUTE_A (1) | 0x1164 | 0x38201000 | +32, R32_G32_B32_A32 FLOAT: colour -> a[0x90] |
| 15 | SET_VERTEX_ATTRIBUTE_A (2..15), _B (16..31) | 0x1168-0x11dc | 0x00000040 | inactive |
| 16 | SET_PIPELINE_PROGRAM_ADDRESS_B (1) / (5) | 0x2058 / 0x2158 | 0x10010000 / 0x10014000 | particles.vert / particles.frag SPH, code at +0x80 (verify) |
| 17 | SET_PIPELINE_REGISTER_COUNT (1) / (5) | 0x204c / 0x214c | from the rig (verify) | GPR counts |
| 18 | VS cbufs | 0x2380-0x2388, 0x2410 | c0 0x10020000 size 0x1000 -> 0x00000001; c1 0x10021000 size 0x100 -> 0x00000011 | group 0 slots 0 and 1 (verify slots) |
| 19 | PS cbuf | 0x2380-0x2388, 0x2490 | c0 0x10022000 size 0x1000 -> 0x00000001 | group 4 slot 0 (verify slot) |

Subchannel 1 (compute):

| # | method | offset | value | meaning |
|---|---|---|---|---|
| 1 | SET_OBJECT | 0x0000 | 0x0000c7c0 | bind AMPERE_COMPUTE_B to subchannel 1 |
| 2 | SET_SHADER_LOCAL_MEMORY_A / B | 0x0790 / 0x0794 | only if the rig's binary needs local memory (verify) | spill space |

## Per-frame program

| # | subch | method | offset | value | meaning |
|---|---|---|---|---|---|
| 1 | 0 | SET_CONSTANT_BUFFER_SELECTOR_A / B / C | 0x2380-0x2388 | 0x00000100 / 0 / 0x10024000 | select CS c1 through the 3D cbuf-update path |
| 2 | 0 | LOAD_CONSTANT_BUFFER_OFFSET + LOAD_CONSTANT_BUFFER x12 | 0x238c, 0x2390 | 0, then words 0-11 | attractors and time; writes guest memory the QMD's cbuf 1 points at |
| 3 | 0 | WAIT_FOR_IDLE | 0x0110 | 0 | last frame's draw is done reading the buffer and the cbuf write has landed before the launch |
| 4 | 1 | SEND_PCAS_A | 0x02b4 | 0x00100300 | QMD address 0x1003_0000 >> 8 |
| 5 | 1 | SEND_SIGNALING_PCAS2_B | 0x02c0 | 0x00000003 | PCAS_ACTION INVALIDATE_COPY_SCHEDULE: launch |
| 6 | 0 | WAIT_FOR_IDLE | 0x0110 | 0 | the compute writes retire before vertex fetch reads them |
| 7 | 0 | CLEAR_SURFACE | 0x19d0 | 0x0000003c | clear RGBA of CT0 |
| 8 | 0 | DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_FIRST | 0x1214 | 0x50040000 | triangle strip, 4 vertices from 0, instance 0 |
| 9 | 0 | DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_SUBSEQUENT, non-incrementing packet of 8191 | 0x1218 | 0x50040000 each | instances 1-8191 (COUNT is 13 bits, 8191 fits one packet) |
| 10 | 0 | SET_REPORT_SEMAPHORE_A / B / C / D | 0x1b00-0x1b0c | 0 / 0x11800010 / frame / 0x1000f010 | frame done |

## Ordering

Hardware: compute and 3D share the graphics engine and its L2, so the WFI after the launch (row 6) is the only barrier
the draw needs, and the WFI before it (row 3) covers the write-after-read on the next frame. The QMD release at
0x1180_0000 is a CPU-visible fence the host can poll instead of the frame report.

Emulator: the launch becomes a Vulkan dispatch over guest VA 0x1010_0000 (the global SSBO, binding 9), the draw reads
the same bytes as an instanced vertex buffer. Between them: a SHADER_WRITE -> VERTEX_ATTRIBUTE_READ barrier
(COMPUTE_SHADER -> VERTEX_INPUT); before the next dispatch: VERTEX_INPUT -> COMPUTE_SHADER. If the backend copies guest
memory into per-draw buffers instead of aliasing it, the copy-back of the SSBO must happen before the vertex upload.

## What the pipeline must provide

The 0xC7C0 class in the pusher (SEND_PCAS_A + SEND_SIGNALING_PCAS2_B -> QMD decode -> dispatch; fifo.rs rejects it
today), per-instance vertex streams (`SET_VERTEX_STREAM_INSTANCE_A`, frequency; not in DrawSnapshot yet), blend
ops and coefficients (only the enable is decoded), and coalescing the FIRST + 8191 SUBSEQUENT draws, which threed.rs
reports as 8192 Draw events, into one `vkCmdDraw(4, 8192)`.
