---
status: "accepted"
date: 2026-10-16
decision-makers: jopdorp
component: [oboromi-gpu]
---
# Silicon is the reference; known failures are per device

## Context and Problem Statement

There is no trustworthy public SM86 assembler and no complete semantic reference. Hand-built encodings have been wrong before, and documentation and other emulators cover Maxwell, not Ampere. Vulkan drivers also differ in float rounding, denormals and NaNs.

## Decision Outcome

Results are checked against registers read back from real Ampere cards. A capture patches one instruction word into an nvcc-built seed kernel, runs it through the CUDA driver API, and records inputs and outputs; observed results are never hand-written. Encodings are checked with nvdisasm first. The scoreboard tests replay every capture on each device and fail on any mismatch not listed. Allowed mismatches are listed by fixture id with a reason, either for every device or for lavapipe only.

### Consequences

* Good, because bugs no reading of documentation would find were found: NaN rules, tensor rounding, shuffle clamping.
* Good, because a device quirk cannot hide a translator bug: it is excused for that device only.
* Bad, because each session costs a rented card and some seeds fault, so coverage arrives in batches.
* Bad, because the captures come from GA102, not from a Switch 2.

### Confirmation

`fixtures_match_silicon`, `fixtures_match_warp_silicon` and `fixtures_match_tensor_silicon` pass on Intel and lavapipe.

Sources: core/tests/fixtures/README, core/src/tests/gpu/fixtures.rs.
