<p align="center">
  <img width="32%" height="32%" src="https://github.com/user-attachments/assets/2cf6431e-e9a5-4f03-98ce-d8c975ddde77" alt="oboromi logo"/>
</p>
<p align="center">
  <a href="https://github.com/0xNikilite/oboromi/blob/main/LICENSE"><img alt="License" src="https://img.shields.io/badge/license-MPL%202.0-blue.svg?style=flat"></a>
  <a href="https://discord.gg/g9sehj8bPz"><img alt="Discord" src="https://img.shields.io/discord/1387476383663390732?style=flat&label=Discord&color=5865F2&logo=discord&logoColor=white"></a>
</p>

<h4 align="center">(◕‿◕)&nbsp;&nbsp;Join our Discord here 🢰</h4>

<h1 align="center">oboromi</h1>
<h4 align="center">a proof-of-concept Nintendo Switch 2 emulator written in Rust</h4>

## Overview

**oboromi** is a modular and work-in-progress emulator for the Nintendo Switch 2. It's built in Rust and focuses on correctness, clarity, and traceability rather than performance at this stage. The current implementation includes a functioning ARM64 CPU core using Dynarmic JIT compilation, comprehensive instruction testing, and cross-platform compatibility.

> [!IMPORTANT]  
> oboromi is **not** yet playable and does not emulate any commercial firmware or games.

## Features

### JIT Backend (Dynarmic)

oboromi uses [Dynarmic](https://github.com/0xNikilite/dynarmic) as a JIT backend for AArch64 instruction translation.  
The included version is a **fork with custom modifications** designed to integrate directly with `DynarmicCPU` in oboromi.

### Comprehensive Instruction Testing
- Reliable test framework using breakpoints and `run()` instead of single-stepping
- 10+ instruction tests covering core ARM64 operations:
  - NOP, ADD, SUB, MOV operations
  - Register and immediate arithmetic
  - Control flow (branches, returns)
  - Multi-instruction sequences

### Memory Management

- **32-bit and 64-bit load/store operations** with proper alignment handling
- **Endianness-aware memory access** using little-endian byte ordering
- **Virtual address translation support** via MMU with 4KB paging and 64-entry TLB

#### Key Components:

**Memory Backend** (`memory.rs`)
- 8MB RAM allocation with bounds checking on all accesses (`dynarmic_interface.rs`)
- Atomic operations support (compare-and-swap, atomic add)
- Exclusive monitor for ARM load/store exclusive instructions
- Direct C interface for Dynarmic integration

**Memory Management Unit** (`mmu/`)
- Virtual to physical address translation
- Sparse page table implementation with permission flags
- Translation Lookaside Buffer (TLB) with FIFO replacement


## Testing & Verification

```
🧪 Starting Dynarmic JIT Instruction Tests...
  Base address: 0x0000000000001000
  Using run() with breakpoints for reliable execution

  ✅ NOP - PASS (24.3ms)
  ✅ ADD X1, X1, #2 - PASS (22.9ms)
  ✅ SUB X2, X2, #1 - PASS (23.4ms)
  ✅ ADD X0, X0, X1 - PASS (23.2ms)
  ✅ MOV X3, X4 - PASS (23.8ms)
  ✅ B +8 - PASS (24.1ms)
  ✅ RET - PASS (23.0ms)
  ✅ Atomic ADD Test - PASS (23.0ms)
  ✅ Memory Access Pattern - PASS (23.7ms)
  ✅ Multiple Arithmetic Ops - PASS (22.2ms)

📊 Test Summary:
  Total tests: 10
  Passed: 10 ✅
  Failed: 0 ❌
  Total time: 254.8ms
```

## GUI (via `eframe`)
- Built-in GUI based on `egui`
- Provides:
  - Test result display
  - Real-time execution statistics

## How to Run

```shell
git clone --recurse-submodules https://github.com/0xNikilite/oboromi
cd oboromi

cargo run
```

The build system will automatically:
- Handle architecture-specific linking (Zydis/Zycore on x86_64 only)
- Compile the C++ interface
- Run the test suite when clicked on the main button

## Platform-Specific Notes

### Windows
- Supports both MSVC and MinGW toolchains
- Automatic library linking for Windows APIs

### macOS
- First test run may be slower due to JIT compilation
- Native support for both Intel and Apple Silicon

### Linux
- nothing to note

## Contributing

Pull requests are welcome! Feel free to fork the repo, open issues, or suggest improvements.

## 📜 License

This project is licensed under the **Mozilla Public License 2.0**.

See [LICENSE](LICENSE) for details.

---

#### Useful Links

* [Rust Lang](https://www.rust-lang.org/)
* [AArch64 ISA Reference](https://developer.arm.com/documentation/ddi0602/latest/)
* [egui](https://github.com/emilk/egui)
* [Dynarmic (my fork)](https://github.com/0xNikilite/dynarmic)

---

> \[!WARNING]
> oboromi is **not affiliated with Nintendo**. This project does not contain any copyrighted firmware
> or ROMs.
