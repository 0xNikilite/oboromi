# oboromi

This is the documentation of oboromi, an early Nintendo Switch 2 emulator written in Rust. An emulator is a program that lets a PC act like the console. It cannot run a game yet. Most of the work so far is on the graphics chip, the GPU.

A game hands the Switch 2's GPU small programs called shaders, for example one that computes the colour of each pixel of a sky. Those shaders are NVIDIA machine code. The emulator rewrites them into SPIR-V, the shader format of the cross-vendor graphics API Vulkan. Then an Intel, AMD or NVIDIA card in a PC can run them.

<video controls muted loop playsinline poster="https://raw.githubusercontent.com/jopdorp/oboromi/assets/julia-demo-poster.png" style="width:100%"><source src="https://raw.githubusercontent.com/jopdorp/oboromi/assets/julia-demo.mp4" type="video/mp4"></video>

*The Demo button: a hand-written SM86 program, translated to SPIR-V, redrawing a Julia set every frame on an Intel laptop GPU.*

| Start here | For |
|---|---|
| [Architecture map](architecture/README.md) | One picture of the whole emulator, and which page answers what |
| [Decision records](adr/README.md) | The big design choices, each explained in plain words first |
| [Glossary](glossary.md) | Every GPU term the pages use, with one plain sentence and an example |
| [SM86 instruction table (PDF)](sm86.pdf) | Every instruction encoding of the Switch 2's GPU, generated from NVIDIA's table |
