---
status: "accepted"
date: 2026-10-17
decision-makers: jopdorp
component: [oboromi-gpu]
---
# A synchronous pusher that hands out draw snapshots

## Context and Problem Statement

A pushbuffer interleaves state writes, draws, cbuf updates, clears and semaphore releases. The order they take effect in matters, and a host GPU queue runs on its own clock. There is no guest OS layer yet to wait on fences or syncpoints.

## Decision Outcome

`Pusher::submit` decodes GPFIFO entries to completion and returns an ordered list of events. The 3D class keeps raw register words and copies the state a draw needs into an immutable `DrawSnapshot` at the draw trigger. `LOAD_CONSTANT_BUFFER`, reports and semaphore releases are events in the same list, so they apply in pushbuffer order. The engine runs each event before the next, and `Gpu::submit` returns when every draw is back in guest memory. No MME, no queue overlap.

### Consequences

* Good, because ordering is trivially right, and tests read results right after `submit`.
* Good, because the pusher and the 3D class are plain data code, tested without a device.
* Bad, because the CPU waits for the GPU on every submit; real titles will need guest fences decoupled from host fences.
* Bad, because macro-driven pushbuffers fail until MME64 exists.

### Confirmation

`core/src/tests/gpu/fifo.rs` for decode and ordering, and the engine acceptance test for the report and the semaphore payloads landing in guest memory.

Sources: core/src/gpu/fifo.rs, core/src/gpu/threed.rs, core/src/gpu/engine.rs, design note "graphics research", project notes on the graphics plan.
