<p align="center">
  <img src="https://dummyimage.com/400x100/000/fff&text=oboromi" alt="oboromi logo" width="50%" />
</p>

<p align="center">
  <a href="https://github.com/0xNikilite/oboromi/blob/main/LICENSE"><img alt="License" src="https://img.shields.io/badge/license-MPL%202.0-blue.svg?style=flat"></a>
  <a href="https://discord.gg/g9sehj8bPz"><img alt="Discord" src="https://img.shields.io/discord/1387476383663390732?style=flat&label=Discord&color=5865F2&logo=discord&logoColor=white"></a>
</p>

<h4 align="center">(◕‿◕)&nbsp;&nbsp;Join our Discord here 🢰</h4>

<h1 align="center">oboromi</h1>
<h4 align="center">a proof-of-concept Nintendo Switch 2 emulator written in Rust</h4>

## Overview

**oboromi** is a modular and work-in-progress emulator for the upcoming Nintendo Switch 2. It's built in Rust and focuses on correctness, clarity, and traceability rather than performance at this stage. The current implementation includes a functioning CPU core, a memory management unit (MMU) with basic paging, and a custom memory subsystem.

> [!IMPORTANT]  
> oboromi is **not** yet playable and does not emulate any commercial firmware or games.

## Features

### AArch64 CPU Core
- Clean interpreter with structured instruction decoding
- Implemented instructions:
  - Arithmetic: `ADD`, `SUB`, `ADDI`, `SUBI`
  - Bitwise: `AND`, `ORR`, `EOR`, `MVN`
  - Comparison & logic: `CMP`, `TST`
  - Branching: `B`, `RET`
  - Memory: `LDR`, `STR`
  - Others: `NOP`, `MOV`
- Fully handles NZCV flags (condition codes)
- Optional instruction tracing with feature flag `trace`

### Memory Management Unit (MMU)
- Virtual to physical address translation via simple page table
- 4 KiB paging with TLB support (64 entries)
- Page faults and access violations are logged
- Mapping utility functions for identity and custom regions

### Memory Subsystem
- Custom memory backend with:
  - Region registration
  - Bounds-checked access
  - Load/store abstraction for 32-bit and 64-bit values
  - Endianness-aware access

### Testing & Examples
- Functional testing via `main.rs`, gated behind a button in the GUI
- Unit tests for CPU & MMU behavior using `cargo test`
- Examples to demonstrate step-by-step usage (`examples/` coming soon)

## GUI (via `eframe`)
- Built-in GUI based on `egui`
- Always included and launched by default
- Provides:
  - Partial memory viewer
  - Manual test runner (button-controlled)

## How to Run

```shell
git clone https://github.com/0xNikilite/oboromi
cd oboromi

cargo run
````

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

---

> [!WARNING]  
> oboromi is **not affiliated with Nintendo**. This project does not contain or support any copyrighted firmware,
BIOS, or ROMs.
