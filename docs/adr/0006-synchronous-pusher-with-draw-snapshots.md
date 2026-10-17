---
status: "accepted"
date: 2026-10-17
decision-makers: jopdorp
component: [oboromi-gpu]
---
# GPU commands run one after another, each draw with frozen settings

## In plain words

A game's command list mixes settings changes, draws, constant updates, clears and "tell the CPU I'm done" signals. Their order matters, but the PC's graphics card runs on its own clock. So we process a whole command list before returning. Every draw gets a frozen copy of its settings, everything runs in list order, and the results are in emulated RAM when the call returns. It is like a cashier who rings up each item before taking the next: slower than a busy kitchen, but never out of order.

## Context and Problem Statement

A pushbuffer, the game's command list, interleaves state writes, draws, constant buffer updates, clears and semaphore releases, the GPU's "got this far" signals. The order they take effect in matters, and a host GPU queue runs on its own clock. There is no emulated OS yet to wait on fences, the host's done-flags, or syncpoints, the Switch's hardware counters.

## Decision Outcome

`Pusher::submit` reads the GPFIFO, the queue of command lists, to the end and returns an ordered list of events. The 3D class keeps the raw register words. At each draw it copies what that draw needs into a frozen `DrawSnapshot`.

`LOAD_CONSTANT_BUFFER`, which updates a constant table, is an event in the same list. So are reports and semaphore releases. They therefore apply in command order.

The engine runs each event before the next, and `Gpu::submit` returns once every draw is back in guest memory. There are no macros (MME) yet, and no overlap of GPU work.

### What this makes easy

* Ordering is right by construction, and tests can read results straight after `submit`.
* The command reader and the 3D class are plain data code, tested without a graphics card.

### What this makes hard

* The CPU waits for the GPU on every submit, so real games will need the game's done-signals decoupled from the host's.
* Command lists that use macros fail until the GPU's macro processor, MME64, is emulated.

### Confirmation

`core/src/tests/gpu/fifo.rs` for decoding and ordering, and the engine acceptance test for the report and semaphore values landing in guest memory.

Sources: core/src/gpu/fifo.rs, core/src/gpu/threed.rs, core/src/gpu/engine.rs.
