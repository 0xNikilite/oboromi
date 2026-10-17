---
status: "accepted"
date: 2026-10-16
decision-makers: jopdorp
component: [oboromi-gpu]
---
# One storage buffer window for guest global memory

## Context and Problem Statement

SASS loads and stores use 64-bit guest addresses. Vulkan shaders cannot dereference guest addresses, and buffer device addresses are host pointers that know nothing of the guest's page tables.

## Decision Outcome

All global memory a dispatch touches is one storage buffer at binding 9. The host resolves the guest range through the page map, gathers it into one buffer aligned down to 16 bytes, and puts the window's base VA in the support buffer. A translated LDG or STG subtracts the base and indexes the buffer. Unmapped ranges fail; buffer device addresses are not used.

### Consequences

* Good, because one rule covers every load, store and atomic, at any width up to 128 bits.
* Good, because a missing mapping is an error, not a wild access.
* Bad, because the window must hold every byte the program may touch, so scattered accesses copy more than they use.
* Bad, because LDG and STG with a memory descriptor need care: the descriptor is not part of the address.

### Confirmation

The compiled mul-add kernel and the gradient program match silicon through this window, and the memory tests cover remaps.

Sources: core/src/gpu/memory.rs `window`, core/src/gpu/sm86/access.rs, core/src/tests/gpu/harness.rs `run_grid`, design note on the Vulkan resource ABI.
