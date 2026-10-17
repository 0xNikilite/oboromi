---
status: "accepted"
date: 2026-10-17
decision-makers: jopdorp
component: [oboromi-gpu]
---
# The T239 decodes with the SM86 instruction table

## Context and Problem Statement

The decoder is generated from the SM86 instruction table. The Switch 2's T239 has a GA10F GPU, which CUDA calls SM88. If its encodings differ, every handler keyed on SM86 is wrong in places.

## Decision Outcome

Keep the SM86 table. CUDA 13.1's nvdisasm accepts `--binary SM88`, and its embedded tables for SM86, SM87 and SM88 are byte-identical. Against the committed table: no class added or removed, no bit field changed, no encoding key changed. The only differences are opcode names that became internal, which would break the generated handler names. Every fixture word disassembles the same under SM86 and SM88. open-gpu-doc also lists the T239 under the same 3D class as the GA102 family.

### Consequences

* Good, because silicon captures from a desktop GA102 check the same encodings the T239 runs.
* Good, because the public opcode names stay readable.
* Bad, because execution semantics are verified on GA102, not on a T239; equality there is assumed, by design.

### Confirmation

The extracted SM88 table is kept on `gpu/sm88-table` beside a diff summary; regenerate and diff when a new CUDA ships.

Sources: scripts/sm_86_instructions.txt, scripts/decoder_generator.rs, the SM88 table diff note, open-gpu-doc classes/3d README notes.
