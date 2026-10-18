---
status: "accepted"
date: 2026-10-17
decision-makers: jopdorp
component: [oboromi-gpu]
---
# The Switch 2 chip decodes with the desktop instruction table

## In plain words

We read Switch 2 shader code with the instruction table of NVIDIA's desktop Ampere chips. That only works if both chips encode instructions the same way. We checked inside NVIDIA's own disassembler: its tables for the desktop chip and the Switch 2 chip are byte for byte the same. So recordings made on a desktop card test the same encodings a Switch 2 runs.

## Context and Problem Statement

The decoder is generated from the SM86 table, the desktop Ampere instruction set. The Switch 2's chip, the T239, has a GA10F GPU, which CUDA calls SM88. If its encodings differed, every handler keyed on SM86 would be wrong in places.

## Decision Outcome

Keep the SM86 table. NVIDIA's disassembler, nvdisasm, accepts `--binary SM88`, and its built-in tables for SM86, SM87 and SM88 are byte-identical.

| Compared with the committed table | Result |
|---|---|
| CUDA version checked | 13.1 |
| Encoding classes, the table's rows | None added or removed |
| Bit fields | None changed |
| Decoder keys | None changed |
| Opcode names | Some marked internal, which would break the generated handler names |
| Every fixture word | Disassembles the same under SM86 and SM88 |

NVIDIA's public open-gpu-doc headers also list the T239 under the same 3D class as the desktop GA102 family.

### What this makes easy

* Recordings from a desktop GA102 card check the same encodings the Switch 2 runs.
* Handler names stay the public, readable opcode names.

### What this makes hard

* Behaviour is only checked on desktop chips, so equality on a Switch 2 is assumed.

### Confirmation

The extracted SM88 table is kept on `gpu/sm88-table` beside a diff summary; regenerate and diff it when a new CUDA ships.

Sources: scripts/sm_86_instructions.txt, scripts/decoder_generator.rs, CUDA 13.1 nvdisasm, open-gpu-doc classes/3d.
