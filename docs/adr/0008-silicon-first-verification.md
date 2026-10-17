---
status: "accepted"
date: 2026-10-16
decision-makers: jopdorp
component: [oboromi-gpu]
---
# Real chips are the reference, and known differences are listed per device

## In plain words

No public document says exactly what every Ampere instruction does. So we ask the chip. We run one instruction on a rented desktop Ampere card, record its inputs and outputs, and require the translator to give the same bytes. Where a Vulkan driver is known to differ, the exception is written down by name, for that driver only.

## Context and Problem Statement

There is no trustworthy public SM86 assembler and no complete description of what instructions do. Hand-built encodings have been wrong before. Public docs and other emulators cover Maxwell, an older NVIDIA generation, not Ampere. Vulkan drivers also differ in float rounding, denormals (tiny near-zero floats) and NaNs (not-a-number results).

## Decision Outcome

Results are checked against registers read back from real Ampere cards. A capture patches one instruction word into a seed kernel, a small CUDA program built with nvcc that carries the instruction. It runs through the CUDA driver and records inputs and outputs. Observed results are never typed in by hand, and every word is checked with nvdisasm first.

Scoreboard tests replay every capture on each device and fail on any mismatch not listed. Allowed mismatches are listed by fixture id with a reason, either for every device or for lavapipe only.

### What this makes easy

* It finds bugs no reading of documentation would, such as NaN rules, tensor rounding and shuffle clamping.
* A device quirk cannot hide a translator bug, because the quirk is excused for that device only.

### What this makes hard

* Each session needs a rented card and some seeds crash, so coverage arrives in batches.
* The recordings come from desktop GA102 cards, not from a Switch 2.

### Confirmation

`fixtures_match_silicon`, `fixtures_match_warp_silicon` and `fixtures_match_tensor_silicon` pass on Intel and lavapipe.

Sources: core/tests/fixtures/README, core/src/tests/gpu/fixtures.rs.
