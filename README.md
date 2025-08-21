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

**oboromi** is a modular and work-in-progress emulator for the upcoming Nintendo Switch 2. It's built in Rust and focuses on correctness, clarity, and traceability rather than performance at this stage. The current implementation includes a functioning CPU core, a memory management unit (MMU) with basic paging, and a custom memory subsystem.

> [!IMPORTANT]  
> oboromi is **not** yet playable and does not emulate any commercial firmware or games.

## Features

### JIT Backend (Dynarmic)

oboromi uses [Dynarmic](https://github.com/0xNikilite/dynarmic) as a JIT backend for AArch64 instruction translation.  
The included version is a **fork with custom modifications** designed to integrate directly with `DynarmicCPU` in oboromi.  

###### these core features below are not used, wait for next updates

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

###### end of features not used

### Testing & Examples
- Instruction-level test framework embedded in the project
- Tests cover:
  - NOP
  - ADD (immediate and register)
  - SUB
  - MOV
  - Branching
  - RET (using X30 as LR)
- Each test is run via `DynarmicCPU::step()` and results are shown in the GUI (and in the terminal)
- Example:

```
✅ NOP - PASS (65.4881ms)
❌ ADD X1, X1, #2 - FAIL: Verification failed (12.6993ms) 
...
````
> [!NOTE]  
> If a test fails, it’s likely due to changes in the code or an unusual environment.  
> The test framework is designed to work reliably: if you run the program without modifying the code, you should see the ✅ marks for passing tests.  
> Any failures should prompt you to double-check recent changes or system configuration that might affect execution.

## GUI (via `eframe`)
- Built-in GUI based on `egui`
- Always included and launched by default
- Provides:
  - Partial memory viewer
  - **Manual test runner** (via GUI button)
  - Live output of instruction test results and stats

## How to Run (On Windows and Linux)

```shell
git clone --recurse-submodules https://github.com/0xNikilite/oboromi
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
* [Dynarmic (my fork)](https://github.com/0xNikilite/dynarmic)

---

> \[!WARNING]
> oboromi is **not affiliated with Nintendo**. This project does not contain any copyrighted firmware
> or ROMs.
