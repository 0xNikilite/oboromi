---
status: "accepted"
date: 2026-10-16
decision-makers: jopdorp
component: [oboromi-gpu]
---
# Game memory reaches a shader through one window buffer

## In plain words

Game shaders read memory by address, like a pointer in C. A Vulkan shader cannot use those addresses, because they point into the emulated console, not into the PC. So before each run the host copies the stretch of game memory the program may touch into one buffer, the window. The shader turns each game address into a position in that window by subtracting the window's start.

## Context and Problem Statement

SASS loads and stores use the game's own GPU addresses. Vulkan shaders cannot follow those. Vulkan's own shader pointers, buffer device addresses, point into host memory and know nothing about the game's address map.

## Decision Outcome

All main memory a run touches, called global memory, is one storage buffer at binding 33. A storage buffer is a large buffer a shader can read and write.

The host looks the game's address range up in the page map, the table from GPU addresses to emulated RAM. It copies that range into one buffer and stores the window's start address in the helper slot, binding 0. For a compute launch from a command list, the range is all mapped guest memory. After the run, the pages the program changed are written back to guest RAM.

| Window fact | Value |
|---|---|
| Start address | Rounded down to a 16-byte boundary |
| Widest access | 128 bits |
| Window for a launch from a command list | All mapped guest memory, at most 4 GiB |

A translated `LDG` (load from global memory) or `STG` (store) subtracts the start and indexes the buffer. An address outside mapped memory fails with an error. Buffer device addresses are not used.

### What this makes easy

* One rule covers every load, store and atomic, at any width the chip has.
* A missing mapping is a clear error instead of a read of random memory.

### What this makes hard

* The window must hold every byte the program might touch, so the first launch copies all mapped memory to the card; later launches only copy pages that changed.
* Some loads carry a memory descriptor, an extra policy word from the compiler, and it must not be added to the address.

### Confirmation

The compiled mul-add kernel and the gradient program match the real card through this window. The engine test runs the gradient kernel from a QMD launch and finds its output in guest memory. The memory tests cover remaps.

Sources: core/src/gpu/memory.rs `window`, core/src/gpu/sm86/access.rs, core/src/tests/gpu/harness.rs `run_grid`.
