# Roadmap

The compute side of the translator is done, and the drawing engine handles the core of a real frame: indexed and instanced draws, depth, blending, textures and compute launches. No real game runs yet, because the OS layer that would load one is missing. For example, no game can ask the GPU driver service for memory, so tests set the GPU up by hand.

"By design" below means left out on purpose, with a named panic, until something needs it.

```mermaid
flowchart LR
    A["Compute instructions<br/>Done"] --> B["Drawing engine<br/>Core features done"]
    B --> C["Six demo scenes<br/>Prepared, first being wired"]
    D["OS layer: game loader, GPU driver service<br/>To do"] --> E["A real game's GPU commands"]
    B --> E
```

*What has to land before a real game's commands can reach the GPU work.*

## What is done and what is open?

| Area | Item | Status |
|---|---|---|
| Translator | Every compute instruction family, including textures, tensors and divergence | Done |
| Translator | Vertex and pixel shaders: ALD, AST, IPA, pixel outputs and depth | Done |
| Translator | Geometry and tessellation data transport: ISBERD, ISBEWR, LDTRAM, OUT, PIXLD | By design |
| Translator | Ray tracing TTU ops, sparse tensor ops, FOOTPRINT, SUQUERY, UCLEA | By design |
| Translator | Float atomics (need VK_EXT_shader_atomic_float), texture reads with implicit detail level | To do |
| Command processor | GPFIFO, SET_OBJECT, channel semaphores, 3D class and draw snapshots | Done |
| Command processor | Compute launch blocks (QMDV02_03, QMDV03_00) for class 0xC7C0, report sizes, semaphore acquires that are already met | Done |
| Command processor | MME64 macros, 2D, DMA and inline-to-memory classes, acquires that wait for a later release | To do |
| Engine | Command-list draws with constant buffers, address remaps, compute launches, indexed and instanced draws, depth, blending, y-flip | Done |
| Engine | Textures from the TIC and TSC pools in pixel shaders, drawing into a texture and sampling it, clears no draw used | Done |
| Engine | Textures in vertex shaders and compute launches, depth kept in guest memory, vertex streams that step every few instances, u8 indices without the extension | To do |
| Textures | Plain and tiled layouts, mip chains, sRGB, R8, RG8, RGBA16F, R32UI, BC1, BC3, BC7 | Done |
| Textures | Other formats and channel orders, 3D, cube and array textures, texture arrays picked at run time | To do |
| Tensors | HMMA, IMMA, DMMA, BMMA, LDSM, MOVM with the chip's exact sums | Done |
| Verification | Session 6 recordings (TF32, IMMA saturation per k16 half, conversions, indexed MOV) | To do: not replayed yet |
| CUDA | cubin runner, native path on libcuda, the three division fixes | Done |
| CUDA | More cuda-samples kernels | To do |
| Demos | pbr-boombox | Being wired |
| Demos | post-effect, particles, sprites2d, pbr-flighthelmet, raytraced | Prepared, not wired |
| OS layer | Loader, system calls, nvdrv, nvmap and address-space calls, syncpoints, vi | To do |
| Upstream | PR #79 and PR #80 | Open |
| Upstream | A PR for gpu-graphics | Not opened |

## What are the honest limits?

- No game boots. There is no loader, and tests write the GPU's command lists by hand.
- Nothing has run on a Switch 2. Instructions are checked on desktop GA102 cards, the same Ampere core. The Switch 2's instruction table is identical, and its behaviour is assumed equal.
- The engine reads the shader header in the layout NAK, Mesa's NVIDIA compiler, uses; no Switch 2 game's shader has confirmed it yet.
- The GPU page size is Ryujinx's big page size; the Switch 2's value is unverified.
- How the vertex id treats a base vertex, and the front-facing value, are unverified.
- Graphics shaders so far were written by hand or by desktop compilers, never taken from a Switch 2 game.

| Shader header fact | Value |
|---|---|
| Header version | 4 |
| Header size | 32 words, so code starts at 0x80 |
| End of code | The branch-to-self after the last EXIT |

Verified by: each "done" row links to its tests on the subsystem page; "in progress" rows are unmerged branches.

Sources: core/src/gpu/sm86/*.rs (named todos), core/src/gpu/engine.rs (errors by name), core/src/gpu/memory.rs, demos/README, PR #79 and #80.
