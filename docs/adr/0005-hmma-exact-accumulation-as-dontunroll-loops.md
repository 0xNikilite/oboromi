---
status: "accepted"
date: 2026-10-17
decision-makers: jopdorp
component: [oboromi-gpu]
---
# Exact HMMA accumulation as DontUnroll loops

## Context and Problem Statement

Silicon sums HMMA products in groups, aligned to the largest exponent and truncated, not in IEEE order. An IEEE fma chain disagreed with the tensor captures. Written out in full, the exact model made each HMMA a module drivers were slow to compile; as one SPIR-V function, both drivers inlined it back.

## Decision Outcome

Implement the silicon model exactly: groups of eight k (four for TF32), alignment to the largest exponent, truncation below it, an exact integer sum, truncating normalisation, per-group rounding for .F16, and IEEE handling of only the inf and NaN terms. Stage the operands once in a Private array and run the sum in four `DontUnroll` loops with Private scalars instead of phis.

### Consequences

* Good, because the tensor scoreboard is bit-exact on Intel and lavapipe.
* Good, because cold compile is fast again on both drivers.
* Bad, because Intel spills registers in this form, and warm runs are slightly slower than the IEEE form.
* Bad, because the loops rely on drivers honouring `DontUnroll`, checked on Intel and lavapipe only.

### Confirmation

The tensor silicon scoreboard and the per-op compile timings in [textures and tensors](../architecture/textures-and-tensors.md).

Sources: core/src/gpu/sm86/tensor.rs, core/src/tests/gpu/tensor.rs.
