# pbr-flighthelmet

The FlightHelmet spinning under the pbr-boombox light and camera, shaded by the same `shaders/pbr.vert` +
`shaders/pbr.frag` into the same 1280x720 RGBA8 target and ZF32 depth. It is the throughput scene: sixteen times the
triangles and fifteen times the texture bytes of BoomBox, six materials and six draws per frame. Conventions, init
program, pools and `(verify)` meaning as in pbr-boombox.md; only the differences are listed here.

Asset: `demos/assets/fetch.sh` downloads `demos/assets/flighthelmet/` (CC0, Microsoft / Gary Hsu, KhronosGroup
glTF-Sample-Assets), converted with `scripts/gltf_to_raw.py FlightHelmet.gltf demos/assets/flighthelmet --max-size 512
--block-linear --readme README.md`. The glTF nodes carry no transforms.

## Numbers

| | FlightHelmet | BoomBox |
|---|---|---|
| primitives / materials / draws per frame | 6 / 6 / 6 | 1 / 1 / 1 |
| vertices (stride 48) | 55392, 2658816 B | 3575, 171600 B |
| triangles (u32 indices) | 94722 (284166 indices, 1136664 B) | 6036 (18108 indices, 72432 B) |
| textures | 15 images, 512x512, 10 mips, 1399808 B each | 4 images, 256x256, 9 mips, 351232 B each |
| texture bytes | 20997120 | 1404928 |
| archive (tar.gz) | 9672515 B | in the repo |

## Draw list

`index_count` and `first_index` come from `primitives` in manifest.json; the indices are already rebased onto the
shared vertex buffer, so every draw uses vertex offset 0.

| draw | primitive (node) | material | textures (TIC) baseColor / normal / metallicRoughness = occlusion | first_index | index_count |
|---|---|---|---|---|---|
| 0 | Hose_low | 0 HoseMat (double sided) | 2 / 0 / 1 | 0 | 59040 |
| 1 | RubberWood_low | 1 RubberWoodMat | 2 / 0 / 1 | 59040 | 72534 |
| 2 | GlassPlastic_low | 2 GlassPlasticMat | 5 / 3 / 4 | 131574 | 24408 |
| 3 | MetalParts_low | 3 MetalPartsMat | 8 / 6 / 7 | 155982 | 60288 |
| 4 | LeatherParts_low | 4 LeatherPartsMat | 11 / 9 / 10 | 216270 | 65688 |
| 5 | Lenses_low | 5 LensesMat (KHR_materials_transmission, drawn opaque) | 14 / 12 / 13 | 281958 | 2208 |

No material has an emissive texture: the emissive handle points at the material's baseColor TIC and the emissive factor
is 0, as for Avocado. Instancing none.

## Guest memory map (changes to pbr-boombox.md)

| VA | size | contents |
|---|---|---|
| 0x1002_1000 + 0x100 m | 32 B each | PS bank 0x1d of material m (m = 0..5) |
| 0x1002_1080 + 0x100 m | 112 B each | PS uniforms of material m |
| 0x1003_0000 | 15 x 32 B | TIC 0..14, TIC i = image i of the manifest |
| 0x1004_0000 | 2658816 B | flighthelmet/vertices.bin |
| 0x102d_0000 | 1136664 B | flighthelmet/indices.bin |
| 0x103f_0000 + 0x16_0000 i | 1399808 B each | the texture file of image i (`textures[].image` in manifest.json) |
| 0x1189_0000 | | colour target |

## Per-material tables

PS uniforms have pbr.frag's layout. Words 0-15 (light, ambient, camera) are the pbr-boombox values in all six copies;
words 16-19 base_colour_factor, 20-23 emissive_factor, 24-27 (metallic, roughness, normalScale, occlusionStrength) come
from `materials[m]` in manifest.json (all six are 1, 1, 1, 1 / 0, 0, 0 / 1, 1, 1, 1 in this model). The material's bank
0x1d table holds the texture handles in pbr-boombox order, each `TIC | 0 << 20`, and the handle of its uniforms at
+0x18. A material switch rebinds bank 0x1d of the pixel stage to the material's table.

## Register program

Init: as pbr-boombox.md with these replacements.

| # | method | offset | value | meaning |
|---|---|---|---|---|
| 17 | SET_VERTEX_STREAM_A_FORMAT / LOCATION_A / B (0) | 0x1c00 / 0x1c04 / 0x1c08 | 0x00001030 / 0x00000000 / 0x10040000 | vertices.bin |
| 18 | SET_VERTEX_STREAM_SIZE_A / B (0) | 0x0600 / 0x0604 | 0x00000000 / 0x00289200 | 2658816 bytes |
| 23 | SET_INDEX_BUFFER_A / B | 0x17c8 / 0x17cc | 0x00000000 / 0x102d0000 | indices.bin |
| 31 | SET_TEX_HEADER_POOL_A / B / C | 0x1574 / 0x1578 / 0x157c | 0x00000000 / 0x10030000 / 0x0000000e | TIC pool, max index 14 |

TIC i: w0 0x58d24908, w1 `0x103f0000 + 0x160000 i`, w2 0x00600000, w3 0x90000008 (MAX_MIP_LEVEL 9 at bits 31:28,
gobs per block height 2), w4 0x008001ff, w5 0x800001ff, w6 = w7 = 0. Same format, version and sRGB notes as the
pbr-boombox TICs.

Per frame: the pbr-boombox segment with model = rotate_y(t * 0.8 rad/s) * scale(2 / extent) * translate(-centre) from
the vertex bounds (the helmet is 0.72 units tall with its origin at the base) and rows 4-9 once per draw of the list.

## What it measures

Frame time against pbr-boombox at the same size, same shaders and state: host time from the GPFIFO kick to the
report semaphore, averaged over a few hundred frames after the first (which pays the texture upload and the pipeline
build). Per-draw cost shows as the gap between the two once triangle count is factored out, and the six material
switches exercise the descriptor update path every frame, which BoomBox never does. Texture upload is measured once
at init: 21 MB of block-linear deswizzle through `texture.rs`.
