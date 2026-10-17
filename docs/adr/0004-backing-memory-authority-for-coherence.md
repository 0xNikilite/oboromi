---
status: "accepted"
date: 2026-10-17
decision-makers: jopdorp
component: [oboromi-gpu]
---
# Emulated RAM is the one true copy of all data

## In plain words

Vulkan cannot read the emulated console's RAM directly, so the emulator keeps PC-side copies of game data. Copies can go stale: the game might move a buffer, change it from the CPU, or draw into memory it later reads as a texture. So emulated RAM is the only truth. Every copy remembers where it came from and when, and every draw writes its pixels back to RAM before it returns. Think of a shared master file: anyone holding a printout checks the date before using it.

## Context and Problem Statement

The engine keeps host copies of game data: buffer copies, the images draws paint into, and translated shaders. The game can repoint a GPU address, write a buffer from the CPU, or draw into memory it later reads. A host copy must never show what the game's memory does not hold.

## Decision Outcome

Guest RAM is the one source of truth. Every host copy is keyed by guest-physical ranges, where the bytes really sit in emulated RAM. It is not keyed by GPU virtual address, which the game can point elsewhere.

A `MemoryTracker` stamps each page write with a global counter, the generation, and who wrote it, CPU or GPU. A copy refreshes when the counter on its pages moved.

Every draw writes its render target, the image it painted, back to guest RAM before `submit` returns. It also records the counter it left. If the target's pages changed since, by the CPU or a clear, the next draw loads it from guest RAM first. Textures follow the same rule, so a texture that was drawn into shows its latest draw.

### What this makes easy

* Moved and shared memory are right by construction, and drawing into a texture then reading it is just a read of RAM.
* A CPU read after `submit` always sees what the GPU wrote.

### What this makes hard

* Every draw pays for copying its pixels back, which is fine for demos but too slow for a game at full speed.
* A small write refreshes a whole page.
* Depth targets are not written back yet, so a depth target must be cleared before its first draw.

### Confirmation

The engine tests repoint a vertex buffer address between draws and check the next draw follows it. They also sample a target as a texture after drawing into it, and load a clear that no draw used.

Sources: core/src/gpu/memory.rs, core/src/gpu/engine.rs.
