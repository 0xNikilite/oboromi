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

**oboromi** is a modular and work-in-progress emulator foundation for the Nintendo Switch 2. It's built in Rust and focuses on correctness, clarity, and traceability rather than performance at this stage.

> [!IMPORTANT]  
> oboromi is **not (yet)** a full Switch 2 emulator. It does not run any Nintendo firmware or games.

## Current Features

### ARM64 CPU Emulation (Unicorn Engine)

oboromi uses [Unicorn Engine](https://www.unicorn-engine.org/) for ARM64 instruction emulation. The `UnicornCPU` wrapper provides:
- Full ARM64 register access (X0-X30, SP, PC)
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

- **8MB emulated RAM** with bounds-checked access
- **32-bit and 64-bit load/store operations** with little-endian byte ordering
- Direct memory read/write primitives for testing

### GUI (via `eframe`)

- Built-in GUI based on `egui`
- Provides:
  - Real-time test result display with pass/fail indicators
  - Execution timing statistics
  - Clean, responsive interface

## Testing & Verification

```
Starting Unicorn Instruction Tests...
  Base address: 0x0000000000001000
  Breakpoint address: 0x0000000000002000

Warming up Unicorn emulator...
  JIT warmup completed in 199.4µs

  Running test: NOP
  Running test: ADD X1, X1, #2
  Running test: SUB X2, X2, #1
  Running test: ADD X0, X0, X1
  Running test: MOV X3, X4
  Running test: B +8
  Running test: RET
  Running test: Atomic ADD Test
  Running test: Memory Access Pattern (3 instructions)
  Running test: Multiple Arithmetic Ops (3 instructions)

📊 Test Summary:
  Total tests: 10
  Passed: 10 ✅
  Failed: 0 ❌
  Total time: 18.7ms
```

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
