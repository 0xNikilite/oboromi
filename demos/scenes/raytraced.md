# Ray-traced BoomBox

`demo::render_raytraced_frame` draws BoomBox on a ground plane through the guest command list. The NVIDIA-compiled
`rt.vert.bin` and `rt.frag.bin` come from `pbr.vert` and `rt.frag`. The fragment shader casts a shadow ray towards the
light, a reflection ray for metallic surfaces and four hemisphere rays for ambient occlusion. Reflections select the
sky or ambient colour according to the hit; they do not shade a second material. AO modulates indirect light in the
same pass, so this scene does not use `rt_ao.comp` or sample the depth target.

Run `demos/assets/fetch.sh`, then:

```sh
cargo run -p oboromi-core --example capture -- raytraced --seconds 8 --fps 60 --out frames
```

The demo requires ray queries, acceleration structures and fragment subgroup arithmetic, ballot and shuffle.
Intel Arc MTL and lavapipe expose these features with Mesa 26.0.8. Unsupported devices return an error before loading
assets; the rendering test skips them.

`Scene` reuses the PBR memory layout. It appends the ground vertices and indices to the model buffers and adds three
single-pixel textures for the ground material. The model is fitted to two units across, rests at y = -1 and turns
around its vertical axis. The ground uses an identity model matrix.

The shader's bank 0x1d contains five texture handles at +0, the guest TLAS address at +0x18 and the uniform-buffer
handle at +0x28. Bank 1 contains the root-offset location, 0x20. The synthetic guest TLAS address is 0x1002f000.
`Gpu::register_acceleration_structure` builds the host structure from the same guest vertices and indices used for
the draws, with the model's transform and the ground's identity transform. The structure is rebuilt each frame.
Native guest BVH construction still needs a source of build inputs; this demo registers its inputs explicitly.

The scene regression redraws the same geometry after moving the acceleration-structure instances beyond the rays'
range and checks that ray intersections darken the visible surfaces. The captured shaders also pass SPIR-V validation.
