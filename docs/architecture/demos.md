# Demos

The Qt window has buttons that draw through the GPU work today. Six bigger scenes show more of the engine: post-effect, sprites, particles, BoomBox, FlightHelmet and a ray-traced one. For example, the Triangle button writes GPU commands into emulated memory every frame, and the full engine draws them.

## What runs today?

| Button | Code | What it proves |
|---|---|---|
| Render Test Frame | `frame::render_test_frame` | A translated SASS program (S2R, IMAD, STG) writes a 1080p gradient as a compute shader |
| Demo | `frame::render_demo_frame` | A hand-written SASS Julia set animates every frame; the time arrives through constant buffer 0 |
| Triangle | `frame::render_triangle_demo` | A command list in guest memory draws a turning triangle through the whole engine |
| Post | `demo_post::render_post_frame` | NVIDIA-compiled shaders draw a pinwheel into one image, then sample it as a texture to add screen effects |
| BoomBox | `demo::render_boombox_frame` | NVIDIA-compiled PBR shaders draw the BoomBox with five mipmapped textures and a depth buffer, turning once every eight seconds under a still light; run `demos/assets/fetch.sh` first |
| Helmet | `demo::render_helmet_frame` | The same path draws FlightHelmet's six materials, one draw each; run `demos/assets/fetch.sh` first |
| Sprites | `demo_sprites::render_sprites_frame` | NVIDIA-compiled sprite shaders draw a scrolling parallax scene of blended sprites as one instanced draw |
| Music | `demo_music::play_music` | A 16-bar loop plays through the audio renderer: synthesised samples on voices, a low-pass and an echo on the final mix |
| Bounce | `demo_bounce::render_bounce_frame` | Balls bounce in a box drawn with the particle shaders, and every hit plays a knock on a free voice |

The Julia set program loops with a predicated branch, a jump taken only where a one-bit flag is set. It reuses one pipeline built once, so a frame costs a dispatch, a readback and a present.

| Julia set uses | Instructions |
|---|---|
| Float maths | FFMA, FMUL, FADD, FSETP |
| Special functions | MUFU.COS, SIN and LG2 |
| Conversions | I2F, F2I |
| Integer maths | IADD3, IMAD, LOP3 |
| Memory and flow | STG, predicated BRA |

Every frame ends in `display::present`, which the window shows.

## What does it look like?

![The Render Test Frame button: a full-HD gradient](https://raw.githubusercontent.com/jopdorp/oboromi/assets/test-frame.png)

*Render Test Frame: a short SASS program (S2R, IMAD, STG) writes a gradient, translated to SPIR-V and run on an Intel laptop GPU.*

<video controls muted loop playsinline poster="https://raw.githubusercontent.com/jopdorp/oboromi/assets/julia-demo-poster.png" style="width:100%"><source src="https://raw.githubusercontent.com/jopdorp/oboromi/assets/julia-demo.mp4" type="video/mp4"></video>

*Demo: a hand-written SM86 program, translated to SPIR-V, redrawing a Julia set every frame on an Intel laptop GPU.*

<video controls muted loop playsinline poster="https://raw.githubusercontent.com/jopdorp/oboromi/assets/triangle-poster.png" style="width:100%"><source src="https://raw.githubusercontent.com/jopdorp/oboromi/assets/triangle.mp4" type="video/mp4"></video>

*Triangle: GPU commands written into emulated memory every frame, drawn through the whole engine on an Intel laptop GPU.*

![NVIDIA's Mandelbrot sample, run through the translator](https://raw.githubusercontent.com/jopdorp/oboromi/assets/cuda-mandelbrot.png)

*NVIDIA's cuda-samples Mandelbrot kernel, compiled by nvcc for SM86 and run unchanged through the translator on an Intel laptop GPU.*

<video controls muted loop playsinline poster="https://raw.githubusercontent.com/jopdorp/oboromi/assets/cuda-mandelbrot-zoom-poster.png" style="width:100%"><source src="https://raw.githubusercontent.com/jopdorp/oboromi/assets/cuda-mandelbrot-zoom.mp4" type="video/mp4"></video>

*The same CUDA kernel, launched once per frame with a smaller scale, zooming into the edge of the set.*

Measured on an Intel Arc (MTL), p50 per frame:

| Demo | Time |
|---|---|
| Test frame, 1080p | 4.2 ms |
| Julia set | 4.7 ms |
| Pushbuffer triangle, 1080p | 6.6 ms |

## Which scenes come next?

```mermaid
flowchart LR
    TRI["Triangle<br/>Draws a triangle"] --> PBR["pbr-boombox<br/>A lit 3D model"]
    TRI --> POST["post-effect<br/>Draw, then screen filters"]
    PBR --> FH["pbr-flighthelmet<br/>A heavy model"]
    PBR --> RT["raytraced<br/>Shadows and reflections by ray queries"]
    TRI --> SPR["sprites2d<br/>Layered 2D sprites"]
    TRI --> PART["particles<br/>Moved by a compute launch"]
```

*The six scenes.*

| Scene | Content | Shaders |
|---|---|---|
| pbr-boombox | The BoomBox or Avocado model, lit with physically based materials and normal maps, one light | pbr.vert, pbr.frag |
| post-effect | A scene drawn offscreen, then colour fringing, scanlines and a vignette | post.vert, post.frag |
| particles | Particles moved by a compute shader, drawn as glowing instanced squares | particles_update.comp, particles.vert, particles.frag |
| sprites2d | Scrolling hill layers and floating sprites, blended over a sky | sprite2d.vert, sprite2d.frag |
| pbr-flighthelmet | The FlightHelmet model through the same PBR shaders, one draw per material | pbr.vert, pbr.frag |
| raytraced | BoomBox in a red and green room with moving light, ray-query shadows, reflections and ambient occlusion | pbr.vert, rt.frag |

The ray-traced scene runs through `demo::render_raytraced_frame` or the `capture` example with `raytraced`. It requires ray queries and fragment subgroup operations.

### What do the new scenes look like?

<video controls muted loop playsinline poster="https://raw.githubusercontent.com/jopdorp/oboromi/assets/demo-sprites-poster.png" style="width:100%"><source src="https://raw.githubusercontent.com/jopdorp/oboromi/assets/demo-sprites.mp4" type="video/mp4"></video>

*Sprites: NVIDIA-compiled sprite shaders draw scrolling hills and floating sprites, blended back to front, from a host-written command list.*

<video controls muted loop playsinline poster="https://raw.githubusercontent.com/jopdorp/oboromi/assets/demo-post-poster.png" style="width:100%"><source src="https://raw.githubusercontent.com/jopdorp/oboromi/assets/demo-post.mp4" type="video/mp4"></video>

*Post-effect: a pinwheel drawn into one image, then sampled as a texture by a second pass that adds the screen effects.*

<video controls muted loop playsinline poster="https://raw.githubusercontent.com/jopdorp/oboromi/assets/demo-particles-poster.png" style="width:100%"><source src="https://raw.githubusercontent.com/jopdorp/oboromi/assets/demo-particles.mp4" type="video/mp4"></video>

*Particles: a compute launch from the command list moves the particles, then one instanced draw shows them.*

<video controls muted loop playsinline poster="https://raw.githubusercontent.com/jopdorp/oboromi/assets/demo-boombox-poster.png" style="width:100%"><source src="https://raw.githubusercontent.com/jopdorp/oboromi/assets/demo-boombox.mp4" type="video/mp4"></video>

*BoomBox: the model turns on a turntable under a still light, so the shine and the glowing display move across it.*

<video controls muted loop playsinline poster="https://raw.githubusercontent.com/jopdorp/oboromi/assets/demo-raytraced-poster.png" style="width:100%"><source src="https://raw.githubusercontent.com/jopdorp/oboromi/assets/demo-raytraced.mp4" type="video/mp4"></video>

*Ray-traced BoomBox: Red and green walls frame glossy floor reflections while warm and cool moving lights sweep shadows across the room.*

<video controls muted loop playsinline poster="https://raw.githubusercontent.com/jopdorp/oboromi/assets/demo-helmet-poster.png" style="width:100%"><source src="https://raw.githubusercontent.com/jopdorp/oboromi/assets/demo-helmet.mp4" type="video/mp4"></video>

*FlightHelmet: six materials and fifteen textures through the same shaders, one draw per material.*

<video controls loop playsinline poster="https://raw.githubusercontent.com/jopdorp/oboromi/assets/demo-music-poster.png" style="width:100%"><source src="https://raw.githubusercontent.com/jopdorp/oboromi/assets/demo-music.mp4" type="video/mp4"></video>

*Music: four instruments made at startup play a looping tune through the audio renderer, with a low-pass and an echo on the mix.*

<video controls loop playsinline poster="https://raw.githubusercontent.com/jopdorp/oboromi/assets/demo-bounce-poster.png" style="width:100%"><source src="https://raw.githubusercontent.com/jopdorp/oboromi/assets/demo-bounce.mp4" type="video/mp4"></video>

*Bounce: every hit plays a knock, higher for small balls and louder for hard hits, panned to where it happened.*

Each scene document in `demos/scenes` lists its guest memory layout and the register writes it sends. Every convention not yet confirmed is tagged "(verify)". The shaders are GLSL that `glslangValidator -V` accepts. NVIDIA's driver compiles them into the SM86 programs the scenes load.

Compiling GLSL on a desktop card gives real SM86 code for each stage. It does not give the Switch 2's shader header or attribute conventions, so those stay tagged until a real game shows them.

## Where do the assets come from?

```mermaid
flowchart LR
    GLB["glTF-Binary model<br/>Khronos sample assets"] --> PY["scripts/gltf_to_raw.py<br/>Converter"]
    PY --> V["vertices.bin<br/>Position, normal, tangent, uv"]
    PY --> I["indices.bin<br/>Which vertices form each triangle"]
    PY --> T["slot.rgba8<br/>Textures, NVIDIA-tiled, with mip levels"]
    PY --> M["manifest.json<br/>Layout, materials, source, licence"]
```

*The converter turns a standard 3D model file into raw files the scenes copy into guest memory.*

BoomBox, Avocado and FlightHelmet are CC0 1.0 by Microsoft, from KhronosGroup's glTF-Sample-Assets. BoomBox and FlightHelmet are too big for the code branches, so `demos/assets/fetch.sh` pulls them from the assets branch. Base colour and emissive textures are sRGB, the others linear. DamagedHelmet is avoided because its source carries a non-commercial licence.

Verified by: an offscreen screenshot of each button's output, the engine acceptance test for the triangle, the Julia centre test, and the block-linear tiling matching the converter.

Sources: core/src/gpu/frame.rs, core/src/gpu/demo.rs, core/src/gpu/demo_bounce.rs, core/src/audio/demo_music.rs, core/src/ffi.rs, gui-qt/src/MainWindow.cpp, demos/README, demos/scenes/*.md, demos/assets/README, scripts/gltf_to_raw.py, core/examples/perf.rs.
