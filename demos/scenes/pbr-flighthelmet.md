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
| primitives / materials / draws per frame | 6 / 6 / 6 | 1 / 1 / 5 |
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
| 0x1002_3000 + 0x100 m | 112 B each | PS c1 of material m (m = 0..5) |
| 0x1002_4000 + 0x100 m | 20 B each | PS handle words of material m |
| 0x1003_0000 | 15 x 32 B | TIC 0..14, TIC i = image i of the manifest |
| 0x1004_0000 | 0x30_0000 | flighthelmet/vertices.bin |
| 0x1034_0000 | 0x12_0000 | flighthelmet/indices.bin |
| 0x1200_0000 + 0x16_0000 i | 0x15_5c00 each | the texture file of image i (`textures[].image` in manifest.json), up to 0x134a_0000 |

The colour, depth and report pages stay where pbr-boombox.md puts them.

## Per-material constant buffer

PS c1 has pbr.frag's layout. Words 0-15 (light, ambient, camera) are the pbr-boombox values in all six copies; words
16-19 base_colour_factor, 20-23 emissive_factor, 24-27 (metallic, roughness, normalScale, occlusionStrength) come from
`materials[m]` in manifest.json (all six are 1, 1, 1, 1 / 0, 0, 0 / 1, 1, 1, 1 in this model, so the copies only differ
in their handle words). Handle words of material m, same order as pbr-boombox: baseColor, normal, metallicRoughness,
occlusion, emissive, each `TIC | 0 << 20`.

The rig's SASS reaches the uniform block through a pointer at `c[0x1d][0x18]` and the textures through bank 0x1d
(extracted-sass-analysis), so whether a material switch is a cbuf rebind or a driver pointer-table write is open
(verify); the register program below assumes the rebind of pbr-boombox.md.

## Register program

Init: as pbr-boombox.md with these replacements.

| # | method | offset | value | meaning |
|---|---|---|---|---|
| 32 | SET_VERTEX_STREAM_A_LOCATION_A / B (0) | 0x1c04 / 0x1c08 | 0x00000000 / 0x10040000 | vertices.bin |
| 34 | SET_VERTEX_STREAM_SIZE_A / B (0) | 0x0600 / 0x0604 | 0x00000000 / 0x00289200 | 2658816 bytes |
| 41 | SET_INDEX_BUFFER_A / B | 0x17c8 / 0x17cc | 0x00000000 / 0x10340000 | indices.bin |
| 64 | SET_TEX_HEADER_POOL_A / B / C | 0x1574 / 0x1578 / 0x157c | 0x00000000 / 0x10030000 / 0x0000000e | TIC pool, max index 14 |

TIC i: w0 0x58d24908, w1 `0x12000000 + 0x160000 i`, w2 0x00600000, w3 0x90000008 (MAX_MIP_LEVEL 9 at bits 31:28,
gobs per block height 2), w4 0x00c001ff for baseColor images (2, 5, 8, 11, 14; sRGB) and 0x008001ff for the rest,
w5 0x800001ff, w6 = w7 = 0. Same format and version caveats as the pbr-boombox TICs (verify).

Per frame: rows 1-4 of pbr-boombox.md with model = rotate_y(t * 0.8 rad/s) * translate(0, -0.9, 0) * scale(2.5)
(the helmet is 0.72 units tall with its origin at the base), then for each draw d of the list:

| # | method | offset | value | meaning |
|---|---|---|---|---|
| 1 | SET_CONSTANT_BUFFER_SELECTOR_A / B / C | 0x2380-0x2388 | 0x00000100 / 0x00000000 / 0x10023000 + 0x100 m | PS c1 of the draw's material |
| 2 | BIND_GROUP_CONSTANT_BUFFER (4) | 0x2490 | 0x00000011 | group 4 slot 1 (verify slot) |
| 3 | SET_CONSTANT_BUFFER_SELECTOR_A / B / C | 0x2380-0x2388 | 0x00000100 / 0x00000000 / 0x10024000 + 0x100 m | its handle words |
| 4 | BIND_GROUP_CONSTANT_BUFFER (4) | 0x2490 | 0x00000021 | group 4 slot 2 (verify slot) |
| 5 | BEGIN | 0x1618 | 0x00000004 | triangles |
| 6 | SET_INDEX_BUFFER_F | 0x17dc | first_index | |
| 7 | DRAW_INDEX_BUFFER | 0x17e0 | index_count | |
| 8 | END | 0x1614 | 0x00000000 | |

then the report row of pbr-boombox.md. The encoded `DRAW_INDEX_BUFFER32_BEGIN_END_INSTANCE_FIRST` form cannot reach
these draws (its first field is 16 bits, count 12 bits), so this scene needs `BEGIN` decoded in threed.rs; until
then draw 0 must follow one encoded triangle draw so the stale topology is triangles.

## What it measures

Frame time against pbr-boombox at the same 1280x720, same shaders and state: host time from the GPFIFO kick to the
report semaphore, averaged over a few hundred frames after the first (which pays the texture upload and the pipeline
build). Per-draw cost shows as the gap between the two once triangle count is factored out, and the six material
switches exercise the descriptor update path every frame, which BoomBox never does. Texture upload is measured once
at init: 21 MB of block-linear deswizzle through `texture.rs`.
