---
status: "accepted"
date: 2026-10-16
decision-makers: jopdorp
component: [oboromi-gpu]
---
# One descriptor set with fixed and dense bindings

## Context and Problem Statement

A translated program reads host support data, up to eight guest cbufs, global memory, and any number of textures and surfaces. Compute and graphics pipelines must agree on where each one lives, and graphics has two stages sharing one layout.

## Decision Outcome

Everything lives in set 0. Binding 0 is a host support uniform buffer, never a guest cbuf. Bindings 1 to 8 are guest cbuf slots 0 to 7 as std140 uvec4 arrays. Binding 9 is the global memory window, 10 and 11 the test-only register in and out buffers. Textures and surfaces take dense bindings from 12 in first-use order, deduplicated by where their handle comes from. Only used bindings are emitted.

### Consequences

* Good, because one `Descriptors` type serves compute and graphics, and a module's resource list is all the host needs to bind it.
* Good, because support data can never collide with what the guest put in cbuf 0.
* Bad, because two stages that read the same cbuf slot from different buffers cannot share a pipeline; the engine rejects that by name.
* Bad, because dynamic texture indexing will need descriptor arrays beside this layout.

### Confirmation

spirv-val on every module, the Vulkan validation layer on dispatch and draw, and the texture tests that bind several images at once.

Sources: core/src/gpu/sm86/mod.rs, core/src/gpu/compute.rs, design notes on the Vulkan resource ABI and the texture ABI.
