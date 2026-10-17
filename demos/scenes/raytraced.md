# raytraced

The pbr-boombox scene on a ground plane, with ray-queried shadows: `shaders/pbr.vert` + `shaders/rt.frag` cast one
shadow ray per pixel towards the light (and one reflection ray on metallic pixels) against a top-level acceleration
structure, then `shaders/rt_ao.comp` casts four ambient-occlusion rays per pixel from the depth target into a float
buffer a post pass can multiply in. Conventions, init program and `(verify)` meaning as in pbr-boombox.md.

The shaders are `#version 460` (GL_EXT_ray_query requires it) and compile with `glslangValidator -V --target-env
vulkan1.2` to SPIR-V 1.5 with the RayQueryKHR capability. The SM86 rig compiles them with the NVIDIA driver like the
other scenes, and the ray-query parts come out as TTU instructions (TTUOPEN, TTUST, TTUGO, TTULD, TTUCCTL, TTUCLOSE,
TTUMACROFUSE in sm_86_instructions.txt); those captures are being taken now and nothing below depends on their exact
sequence.

## Scene

| object | geometry | material | in the acceleration structure |
|---|---|---|---|
| BoomBox | pbr-boombox vertices and indices, 6036 triangles, about 2 units across after scale(100) | its own, TIC 0-3 | BLAS 0, instance 0 with the frame's model matrix |
| ground | 4 vertices (±3, -1.0, ±3), normal +y, tangent +x, uv 0..1, 2 triangles | grey, roughness 0.9, metallic 0: TIC 4-6 | BLAS 1, instance 1, identity |

Camera from (0, 1.0, 3.2) to (0, -0.3, 0), so the shadow behind and left of the box is on screen; light, ambient and
projection as pbr-boombox. rt.frag's uniform block is pbr.frag's plus `sky` at +112: (0.35, 0.45, 0.6, bias 0.002).

## Guest memory map (additions to pbr-boombox.md)

| VA | size | contents |
|---|---|---|
| 0x1002_3000 / 0x1002_3100 | 128 B each | PS c1 for the BoomBox and the ground draw |
| 0x1002_4100 | 20 B | ground handle words: 4, 5, 6, 6, 4 |
| 0x1002_7000 | 96 B | rt_ao.comp c1: inv_view_proj, params (1280, 720, radius 0.5, frame), rays (4, 0, 0, 0) |
| 0x1003_0080 | 4 x 32 B | TIC 4-6 ground (8x8 RGBA8 pitch 32, one level), TIC 7 the depth target as a ZF32 texture (verify format) |
| 0x1008_0000 | 0x1000 | ground vertices.bin (192 B) at +0, indices 0 1 2 0 2 3 at +0x100, TIC 4-6 texels at +0x200/+0x300/+0x400 |
| 0x1009_0000 | 0x1_0000 | shader heap 2: rt.frag program at +0x0000, rt_ao.comp code at +0x8000 |
| 0x100a_0000 | 0x100 | QMD for rt_ao.comp, as particles.md with a 160 x 90 CTA raster of 8 x 8 threads, cbuf 1 = 0x1002_7000 |
| 0x1240_0000 | 0x38_4000 | AO buffer, 1280 x 720 f32 row-major |
| 0x1300_0000 | from the driver (verify) | TLAS: instance descriptors, then the top-level BVH |
| 0x1310_0000 | from the driver (verify) | BLAS 0 (BoomBox) |
| 0x1340_0000 | from the driver (verify) | BLAS 1 (ground) |

## How the acceleration structure reaches the shader (guest side)

On the guest, NVN builds acceleration structures through the driver into guest memory, in NVIDIA's own BVH format, and
the application only hands over vertex and index buffers, instance transforms and a destination. The shader never sees
the layout: it passes the TLAS address to the RT core through TTU instructions, and the address arrives in a driver
constant bank the way the uniform pointers do (`c[0x1d]`, extracted-sass-analysis) (verify). The BVH format, the build
path (compute launches or a dedicated method) and the TTU register protocol are unknown (verify); no TTU mapping exists
in the translator yet. For this demo the synthetic guest writes no BVH at all: it writes the instance list and the
TLAS address, which is all the host plan below needs.

## Register program additions

Init, after pbr-boombox.md:

| # | method | offset | value | meaning |
|---|---|---|---|---|
| 1 | SET_PIPELINE_PROGRAM_ADDRESS_A / B (5) | 0x2154 / 0x2158 | 0x00000000 / 0x10090000 | rt.frag replaces pbr.frag |
| 2 | SET_TEX_HEADER_POOL_C | 0x157c | 0x00000007 | TIC pool max index 7 |
| 3 | LOAD_CONSTANT_BUFFER into the PS driver bank | 0x2390 | 0x13000000, 0 | TLAS address at the offset the rig capture shows (verify offset and bank) |
| 4 | SET_OBJECT (subchannel 1) | 0x0000 | 0x0000c7c0 | compute class for rt_ao.comp |

Per frame, replacing pbr-boombox rows 4-10:

| # | subch | method | offset | value | meaning |
|---|---|---|---|---|---|
| 1 | 0 | CLEAR_SURFACE | 0x19d0 | 0x0000003d | clear Z and RGBA |
| 2 | 0 | LOAD_CONSTANT_BUFFER into the instance list | 0x2390 | instance 0 transform, 12 words | the model matrix as 3x4 rows (verify layout) |
| 3 | 0 | five BoomBox draws | 0x17e4 | as pbr-boombox rows 5-9 | PS c1 0x1002_3000, handles 0x1002_4000 |
| 4 | 0 | SET_VERTEX_STREAM_A_LOCATION_B (0), SET_VERTEX_STREAM_SIZE_B (0), SET_INDEX_BUFFER_B | 0x1c08 / 0x0604 / 0x17cc | 0x10080000 / 0xc0 / 0x10080100 | ground buffers |
| 5 | 0 | rebind group 4 slots 1 and 2 | 0x2380-0x2388, 0x2490 | 0x10023100, 0x10024100 | ground uniforms and handles |
| 6 | 0 | DRAW_INDEX_BUFFER32_BEGIN_END_INSTANCE_FIRST | 0x17e4 | 0x40060000 | 6 indices |
| 7 | 0 | restore stream, index buffer and slots to the BoomBox values | | | next frame starts clean |
| 8 | 0 | WAIT_FOR_IDLE | 0x0110 | 0 | depth is written before the AO pass reads it |
| 9 | 1 | SEND_PCAS_A / SEND_SIGNALING_PCAS2_B | 0x02b4 / 0x02c0 | 0x00100a00 / 0x00000003 | launch rt_ao.comp |
| 10 | 0 | WAIT_FOR_IDLE, then the report | 0x0110, 0x1b00-0x1b0c | as pbr-boombox row 10 | frame done |

The AO pass samples depth through a TIC, so the depth target must live in guest memory, which pbr-boombox keeps
host-only (verify).

## Host side

The emulator builds the Vulkan acceleration structures itself from the same guest vertex data, with
VK_KHR_acceleration_structure and VK_KHR_ray_query:

- BLAS 0 and 1 once, from the stream and index buffer VAs the draws use: positions R32G32B32_SFLOAT at offset 0 stride
  48, u32 indices, opaque geometry, PREFER_FAST_TRACE. The guest BLAS VA is the cache key.
- The TLAS every frame from the guest instance list at 0x1300_0000 (two instances, transform plus BLAS VA), with
  PREFER_FAST_BUILD; two instances are cheap enough that refit is not worth it.
- The translated shader gets the TLAS as an `accelerationStructureEXT` descriptor next to the texture bindings of
  ADR 0001 (a new binding kind, not yet in the ABI), and the TTU sequences map to OpRayQueryInitializeKHR / Proceed /
  GetIntersectionType once the captures show their shape (verify).

When the TTU emulation lands it receives the guest's TLAS address and resolves it through this same cache, so a real
game whose BVH bytes the host cannot read still gets a host-built structure from its build inputs.

Local devices, vulkaninfo, Mesa 26.0.8: Intel Arc (MTL, ANV) and lavapipe (LLVM 21.1.8) both expose
VK_KHR_acceleration_structure and VK_KHR_ray_query with `accelerationStructure` and `rayQuery` true (lavapipe also
VK_KHR_ray_tracing_pipeline), so the demo and its tests run on CI.

## What the pipeline must provide

Everything pbr-boombox needs, plus: a second vertex and index buffer bound mid-frame, a compute launch after a draw
with a depth-to-texture barrier, depth in guest memory, an acceleration-structure binding kind, the TTU translation and
the host BLAS/TLAS cache.
