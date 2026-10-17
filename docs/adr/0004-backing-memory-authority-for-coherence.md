---
status: "accepted"
date: 2026-10-17
decision-makers: jopdorp
component: [oboromi-gpu]
---
# Guest memory is the authority for coherence

## Context and Problem Statement

The engine keeps host copies of guest data: buffer mirrors, render targets, compiled modules. The guest can remap a VA, write a buffer from the CPU, or render into memory it later samples. Host copies must never show what guest memory does not hold.

## Decision Outcome

Guest RAM is the one source of truth. Every host resource is keyed by the guest-physical ranges behind it, not by VA. A `MemoryTracker` stamps each physical page write with a global generation and its writer. A mirror refreshes when its ranges' generation moved. Every draw writes its target back to guest RAM before `submit` returns, and stores the generation it left. A target whose pages changed outside its own draws is an error until loading it is implemented.

### Consequences

* Good, because remaps and aliasing are correct by construction, and render to texture is a read of guest memory.
* Good, because a CPU read after `submit` always sees GPU results.
* Bad, because every draw pays a readback; fine for the demos, not for a title at full speed.
* Bad, because page granularity refreshes whole pages for small writes.

### Confirmation

The engine acceptance test remaps a vertex VA between draws and checks the next draw follows it.

Sources: core/src/gpu/memory.rs, core/src/gpu/engine.rs, project notes on the graphics plan.
