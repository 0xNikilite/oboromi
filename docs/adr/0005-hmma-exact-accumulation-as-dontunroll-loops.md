---
status: "accepted"
date: 2026-10-17
decision-makers: jopdorp
component: [oboromi-gpu]
---
# Matrix sums copy the chip exactly, written as small loops

## In plain words

Tensor cores multiply small matrices in one instruction. The real chip adds up the products its own way: in groups, with the lowest bits cut off instead of rounded. Normal float maths gives slightly different last bits, so we copied the chip's method exactly. Written out naively, that makes shaders so big that drivers take seconds to compile them. So we wrote it as small loops and told the driver not to unroll them.

## Context and Problem Statement

The chip sums HMMA products, the half-precision matrix multiply-add, in groups lined up to the largest exponent and truncated. It does not add them one at a time with rounding, as standard IEEE float rules do. A chain of standard fused multiply-adds disagreed with the recordings from a real card. The exact method, written out in full, made each HMMA a shader drivers were slow to compile. Wrapping it in one SPIR-V function did not help, because both drivers inlined it, pasting it back into every caller.

## Decision Outcome

Implement the chip's method exactly:

| Step | Rule |
|---|---|
| Grouping | Eight products per group along k, four for TF32 (a reduced-precision float) |
| Alignment | Every term lined up to the largest exponent |
| Truncation | Bits below that are dropped, not rounded |
| Sum | An exact integer sum, turned back into a float by truncation |
| .F16 results | Each group rounds to a half-precision float |
| Inf and NaN | Standard float rules, applied to only those special terms |
| Code shape | Four loops, each marked `DontUnroll` |

The inputs are loaded once into a per-thread array. `DontUnroll` is a SPIR-V hint that tells the driver to keep a loop as a loop. Loop state lives in plain per-thread variables, not in phi values, the compiler's merge points at the top of a loop.

### What this makes easy

* The tensor tests match the real chip bit for bit on Intel and lavapipe.
* Shaders with tensor instructions compile fast again on both drivers.

### What this makes hard

* Intel runs out of registers in this form, so warm runs are a little slower than standard float maths.
* The speed relies on drivers honouring `DontUnroll`, which is checked on Intel and lavapipe only.

### Confirmation

The tensor silicon scoreboard and the per-instruction compile timings in [textures and tensors](../architecture/textures-and-tensors.md).

Sources: core/src/gpu/sm86/tensor.rs, core/src/tests/gpu/tensor.rs.
