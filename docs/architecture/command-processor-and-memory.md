# Command processor and memory

The command processor is the GPU's front desk: it reads the command lists a game writes and hands each command to the right engine. Memory follows one rule: the emulated console RAM holds the only real copy of anything. For example, if a game moves a vertex buffer, the next draw reads the new place, because every copy is checked against RAM.

## What does a command look like?

A game puts commands in a pushbuffer, a list of 32-bit words in guest memory. It then adds an entry to the GPFIFO, a ring of pointers to those lists. Each command starts with a header word saying which register to write, on which engine, and how many data words follow.

| Item | Bits | Meaning |
|---|---|---|
| GPFIFO entry word 0 | 31:2 | Pushbuffer address, low part |
| GPFIFO entry word 1 | 7:0, 9, 30:10, 31 | Address high byte, level, length in words, sync |
| Header | 11:0 | Method, the register to write, as a word index |
| Header | 15:13 | Subchannel, the slot that picks the engine |
| Header | 28:16 | How many data words follow, or the value itself in immediate mode |
| Header | 31:29 | Mode: 1 incrementing, 3 non-incrementing, 4 immediate, 5 increment once, 7 end of segment |

Low method numbers belong to the channel, the command stream itself. Its `SET_OBJECT` binds an engine, a class, to a subchannel, and its `SEMAPHORE_A` to `D` methods write "got this far" signals. Every other method goes to the class bound on the header's subchannel. An empty subchannel or an unknown class is an error.

```mermaid
flowchart LR
    GP["GPFIFO entry<br/>Pointer to a command list"] --> PB["Pushbuffer words<br/>Headers and data"]
    PB --> HDR{"Channel method?<br/>Number below 0x40"}
    HDR -->|"Yes"| CH["Channel<br/>Bind engines, write signals"]
    HDR -->|"No"| SUB{"Which engine is on<br/>The subchannel?"}
    SUB -->|"0xC797"| TD["3D engine registers<br/>Frozen per draw"]
    SUB -->|"0xC7C0"| CC["Compute engine, own branch<br/>Reads the launch block"]
    SUB -->|"Other"| ERR["Named error"]
```

*How one command word finds the engine it is meant for.*

## What does the 3D engine keep?

The 3D class, 0xC797, is the drawing engine. `ThreeD` keeps its raw register words and notes which groups changed: vertex streams, attributes, colour targets, viewport and scissor, shaders, constant buffer bindings, blend and depth, and the index buffer.

At each draw it freezes a `DrawSnapshot`, a copy of everything the draw needs. That covers the triangle type and counts, vertex streams and attributes, and every shader slot. It also holds the first colour target, viewport, scissor, write mask, depth, culling, blending, the index buffer and the bound constant buffers.

A constant buffer, or cbuf, is a small read-only table the game hands a shader, such as the camera matrix. `LOAD_CONSTANT_BUFFER` updates one, and it is an ordered event too. So a cbuf changed between draws reaches the right draw.

## How is a compute launch read?

The compute class, 0xC7C0, lives on the `gpu/qmd-dispatch` branch. A launch is described by a QMD, a fixed-size block in guest memory. Writing `SEND_SIGNALING_PCAS_B` makes the class fetch the QMD at `SEND_PCAS_A` and decode it into a `DispatchSnapshot`.

| QMD field group | What it holds |
|---|---|
| Program | The shader's address |
| Grid and block | How many thread groups, and how many threads per group |
| Memory | Shared memory size, shared and local memory windows |
| Constant buffers | Eight cbuf bindings |
| Signals | Two release semaphores |
| Limits | Barrier and register counts |
| Textures | The sampler and texture pools |
| Version | QMDV02_03 only; others are rejected |

## How do GPU addresses become memory?

Shaders use GPU virtual addresses (VA), which the game can point anywhere. `PageMap` turns them into guest-physical addresses (PA), the place in emulated RAM where the bytes really sit. There is one map per GPU address space, and an address outside it is an error, never a guess.

`MemoryTracker` stamps every write to a physical page with a global counter, the generation, and who wrote it, CPU or GPU. A host copy compares that counter with the one it was made from, so it knows when it is stale.

| Fact | Value |
|---|---|
| Page size | 64 KiB, the big page size Ryujinx uses; the T239 value is unverified |
| Map scope | One per GPU address space |

| Host copy | Keyed by | Refreshed when |
|---|---|---|
| Buffer mirror: vertices, cbufs | Guest-physical ranges | The ranges' generation moved |
| Render target, the image a draw paints | Ranges, format, size, depth | Always written back after the draw |
| Translated shader | Hash of shader header and code | Never, the key is the content |

The rule: guest memory is the authority. Every GPU result is written back before `submit` returns. A colour target whose pages changed outside its own draws is an error until loading is implemented, never a stale image. Repointing a VA changes what the next draw reads.

Why: keying by physical page stays right when memory is remapped or shared. And nothing can show a frame the guest's memory does not hold. See [ADR-0004](../adr/0004-backing-memory-authority-for-coherence.md).

## What is still missing, and why?

| Item | Why it waits |
|---|---|
| MME64 macros, the GPU's built-in macro processor | No Ampere header exists; the only public MME64 docs are Turing's |
| Syncpoint and channel calls, nvdrv, nvmap, address-space calls | No loader or game makes them yet |
| Semaphore acquire, waiting on a signal | The engine reports it as a named error |
| 2D, DMA and inline-to-memory classes | The first scenes do not need them |

Verified by: `core/src/tests/gpu/fifo.rs` and `memory_map.rs`, the engine acceptance test (draw, report, VA remap), and the QMD tests on its branch.

Sources: core/src/gpu/{fifo,threed,memory,engine}.rs, core/src/gpu/compute_class.rs on gpu/qmd-dispatch, NVIDIA's clc76f.h, clc797.h and clc7c0qmd.h.
