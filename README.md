<p align="center">
  <img width="32%" height="32%" src="https://github.com/user-attachments/assets/2cf6431e-e9a5-4f03-98ce-d8c975ddde77" alt="oboromi logo"/>
</p>
<p align="center">
  <a href="https://github.com/0xNikilite/oboromi/blob/main/LICENSE"><img alt="License" src="https://img.shields.io/badge/license-GPL%20v3-blue.svg?style=flat"></a>
  <a href="https://discord.gg/g9sehj8bPz"><img alt="Discord" src="https://img.shields.io/discord/1387476383663390732?style=flat&label=Discord&color=5865F2&logo=discord&logoColor=white"></a>
</p>

<h4 align="center">(◕‿◕)&nbsp;&nbsp;Join our Discord here 🢰</h4>

<h1 align="center">oboromi</h1>
<h4 align="center">a proof-of-concept Nintendo Switch 2 emulator foundation written in Rust</h4>

## Overview

**oboromi** is a modular and work-in-progress emulator foundation for the Nintendo Switch 2. It's built in Rust and focuses on correctness, clarity, and traceability rather than performance at this stage. It currently emulates an 8-core ARM64 CPU with 12GB of shared memory.

> [!IMPORTANT]  
> oboromi is **not (yet)** a full Switch 2 emulator. It does not run any Nintendo firmware or games.

## Current Features

### ARM64 CPU Emulation (Unicorn Engine)

oboromi uses [Unicorn Engine](https://www.unicorn-engine.org/) for ARM64 instruction emulation. The `UnicornCPU` and `CpuManager` provide:
- **8-Core CPU Architecture**: Orchestrated via `CpuManager` with shared memory access.
- Full ARM64 register access (X0-X30, SP, PC) per core.
- Memory mapping with permission control
- Breakpoint handling via `BRK` instructions
- Safe Rust interface with proper error handling

### Comprehensive Instruction Testing
- Reliable test framework using breakpoints and `run()` for stable execution
- **10/10 instruction tests passing** covering core ARM64 operations:
  - NOP, ADD (immediate/register), SUB (immediate)
  - MOV (register), RET, B (branch)
  - Multi-instruction sequences and memory access patterns
- **Fast execution**

### Memory Management

- **12GB Combined Emulated RAM** (Lazily allocated)
- **32-bit and 64-bit load/store operations** with little-endian byte ordering
- Direct memory read/write primitives for testing

### GPU Emulation (Work in Progress)

- **SM86 Instruction Decoding**: Implementation of NVIDIA SM86 shader instruction decoding (128-bit instructions).
- **SPIR-V Generation**: translating decoded instructions (like `al2p`) into SPIR-V intermediate representation.
- Foundation for future compute and graphics shader translation.

### GUI (via `eframe`)

- Built-in GUI based on `egui`
- Provides:
  - Real-time test result display with pass/fail indicators
  - Execution timing statistics
  - Clean, responsive interface

## How to Run

```shell
git clone https://github.com/0xNikilite/oboromi
cd oboromi

# Build and run (requires CMake and Ninja)
cargo run
```

The build system will automatically:
- Compile the Unicorn Engine C++ bindings
- Link required libraries
- Launch the GUI with integrated test suite

### Prerequisites

- **Rust** (latest stable)
- **CMake** (3.16+)
- **Ninja** build system
- **C++ compiler**: MSVC (Windows), Clang (macOS/Linux)

## Contributing

Pull requests are welcome! Feel free to fork the repo, open issues, or suggest improvements.

## 📜 License

This project is licensed under the **GNU General Public License v3**.

See [LICENSE](LICENSE) for details.

---

#### Useful Links

* [Rust Lang](https://www.rust-lang.org/)
* [AArch64 ISA Reference](https://developer.arm.com/documentation/ddi0602/latest/)
* [egui](https://github.com/emilk/egui)
* [Unicorn Engine](https://www.unicorn-engine.org/)
* [official mirror](https://git.eden-emu.dev/Nikilite/oboromi)

---

> \[!WARNING]
> oboromi is **not affiliated with Nintendo**. This project does not contain any copyrighted firmware
> or ROMs.
