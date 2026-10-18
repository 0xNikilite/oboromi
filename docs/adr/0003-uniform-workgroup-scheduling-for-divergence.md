---
status: "accepted"
date: 2026-10-17
decision-makers: jopdorp
component: [oboromi-gpu]
---
# Threads that disagree take turns

## In plain words

The GPU runs threads in teams called warps. When teammates take different branches, the real chip parks some, runs the others, and joins them back later. Some instructions make teammates swap values, so they must be at the same place at the same time. We run such programs in turns: each turn, the teammates furthest behind take one step while the rest wait. It is like a tour group where the stragglers always walk next, so everyone meets at every stop.

## Context and Problem Statement

The translator runs each thread as a loop over its own program counter, the pc. Collectives, instructions where teammates swap values such as VOTE and SHFL, need every thread to reach the same Vulkan barrier together. So does BAR, the chip's own barrier. With threads at different pcs that is undefined behaviour, and votes by part of a warp could not be reproduced.

## Considered Options

* Rebuild the game's jumps as nested Vulkan loops and ifs.
* Use only Vulkan's subgroups, its own name for warps.
* Keep the pc loop, and run it in turns across the whole thread group.

## Decision Outcome

Keep the pc loop. A program with collectives or BAR moves every thread of its group, the workgroup, together. Each turn the group votes for the lowest pc any thread waits at. Threads at that pc run one step, and the rest sit out with their memory writes masked.

BSSY and BSYNC, the chip's "rejoin here" markers, are tracked per thread. BAR arrivals are counted per barrier until all the required threads arrive. Programs without collectives keep the free per-thread loop. Swapping values through shared memory is the one method on every device.

| Device fact | Value |
|---|---|
| NVIDIA warp width | 32 threads |
| Subgroup width on lavapipe | 8 threads |

### What this makes easy

* Votes, shuffles and barriers after a branch match the real chip, including votes by part of a warp.
* It runs on lavapipe, whose subgroups are narrower than a warp.

### What this makes hard

* Programs with collectives pay a vote and a barrier on every step.
* Whether a thread spinning in a wait loop ever lets the others move is not verified.

### Confirmation

The warp scoreboard passes on Intel and lavapipe, and tests cover branches of unequal length, nested markers and early exit.

Sources: core/src/gpu/sm86/mod.rs `translate_program`, core/src/gpu/sm86/warp.rs.
