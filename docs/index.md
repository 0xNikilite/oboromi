# oboromi

oboromi is a work-in-progress Nintendo Switch 2 emulator written in Rust. These pages explain how it is built, with most of the depth in the GPU work: translating the Switch 2's Ampere shader code to Vulkan and driving it from guest pushbuffers.

| start here | for |
|---|---|
| [Architecture map](architecture/README.md) | one diagram of the whole emulator and a line per page |
| [Decision records](adr/README.md) | why the GPU work is shaped the way it is |
| [Glossary](glossary.md) | the terms the pages use |
| [SM86 instruction table (PDF)](sm86.pdf) | every SM86 encoding class, generated from the instruction table |
