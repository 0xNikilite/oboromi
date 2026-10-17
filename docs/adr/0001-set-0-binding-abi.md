---
status: "accepted"
date: 2026-10-16
decision-makers: jopdorp
component: [oboromi-gpu]
---
# Every shader finds its data in one fixed table of numbered slots

## In plain words

A Vulkan shader cannot follow a pointer to its data. Instead it reads from numbered slots, and before each run the host plugs a buffer or an image into each slot. Think of a patch panel: the shader asks for a socket by number and reads whatever is plugged in there. We fixed the socket numbers once, for every shader, so the host always knows what goes where.

## Context and Problem Statement

A translated shader needs several kinds of data: our own helper values, the game's constant tables, the game's main memory, and any number of textures. Compute shaders and drawing shaders must agree on which slot holds what. A vertex shader and a pixel shader drawing together also share one slot layout.

## Decision Outcome

Vulkan calls a slot a binding and a group of slots a descriptor set. Everything lives in one group, set 0, with these slots:

| Binding | Vulkan kind | Holds |
|---|---|---|
| 0 | Uniform buffer: small, read-only | Our helper values, today the start address of the memory window; never game data |
| 1 to 32 | Uniform buffers | The game's constant buffers 0 to 31, all the banks a stage has |
| 33 | Storage buffer: large, read-write | The window onto the game's main memory, see [ADR-0002](0002-one-ssbo-window-for-global-memory.md) |
| 34, 35 | Storage buffers | Register values in and out, for tests only |
| 36 and up | Image slots | Textures and writable images, numbered in the order the shader first uses them |

A constant buffer, or cbuf, is a small read-only table of numbers the game hands the shader, such as the camera matrix. Vulkan's layout rule for uniform buffers, std140, pads each array element to a fixed width. A `uvec4` is exactly that width, so each cbuf is declared as a `uvec4` array and no space is wasted.

| Layout fact | Value |
|---|---|
| std140 array element | Padded to 16 bytes |
| `uvec4` | Four 32-bit words, 16 bytes |

Two texture reads whose handle comes from the same cbuf word share one slot. A shader declares only the slots it uses. That list, its resource list, tells the host exactly what to plug in.

### What this makes easy

* One piece of host code, `Descriptors`, plugs in data for both compute and drawing.
* Our helper values can never be confused with what the game put in its first constant buffer.

### What this makes hard

* In one draw, a vertex and a pixel shader cannot read one cbuf slot from different buffers, and the engine refuses it by name.
* Shaders that pick a texture by a run-time index will need arrays of slots next to this layout.

### Confirmation

Checked by spirv-val on every module, the Vulkan validation layer on dispatch and draw, and the texture tests that bind several images at once.

Sources: core/src/gpu/sm86/mod.rs, core/src/gpu/compute.rs.
