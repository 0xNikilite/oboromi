<p align="center">
  <img src="https://dummyimage.com/400x100/000/fff&text=oboromi" alt="oboromi logo" width="50%" />
</p>

<p align="center">
  <a href="https://github.com/nikilites/oboromi/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/license-MPL%202.0-blue.svg" alt="License" />
  </a>
</p>

<h1 align="center">oboromi</h1>
<h3 align="center">A proof-of-concept Nintendo Switch 2 emulator written in Rust</h3>
<h4 align="center"><em>Very WIP — nothing works yet, don’t expect miracles.</em></h4>

## Overview

**oboromi** is a clean and modular emulator framework targeting the **Nintendo Switch 2**, written in Rust for maximum maintainability and performance.  
Its goal is to simulate the new hardware step-by-step, following real architectural assumptions (when available) and clean software design.

## Features (so far)

- ✅ **Memory subsystem**: Read/write with bounds checks, 64 MiB placeholder DRAM  
- ✅ **CPU core**: ARM64 registers, memory-mapped, supports NOP, MOV immediate, ADD and SUB instructions with flag updates  
- ✅ **Integration tests**: Validates CPU–Memory interaction and instruction execution correctness  
- 🧪 **Main test harness** in `main.rs` for step-by-step opcode decoding and execution verification  
- 🧱 **Modular architecture**: Designed for scalable addition of CPU components and instruction sets  

## 🧪 Try It

```bash
git clone https://github.com/nikilites/oboromi
cd oboromi
cargo run
```

If everything is set up right, you should see:

```
✅ Memory tests passed  
✅ CPU–Memory integration test passed
```

## 🔍 Current Target Hardware (Known)

| Component       | Details                                   |
|----------------|-------------------------------------------|
| **SoC**         | NVIDIA GMLX30-A1                          |
| **RAM**         | 12 GB LPDDR5X (2 × 6 GB SKhynix)          |
| **Storage**     | UFS 3.1 (Kioxia or SKhynix)               |
| **WiFi/BT**     | MediaTek MT3681AEN                        |
| **Audio**       | Realtek ALC5658                           |
| **Voice**       | Intelligo IG2200                          |
| **Power**       | MAX77851 (PMIC) + DA9092 (Sub-PMIC)       |
| **USB**         | Genesys GL852G + Cypress CYPD6228         |
| **GC ASIC**     | B2349 GCBRG HAC STD T2010423              |

---

> [!NOTE]  
> Only the comments are “vibe-written” — the code itself is fucking written with my hands.

> [!WARNING]  
> This project **does not** include any proprietary firmware, keys, or dumps. Everything is clean-room.

---

## 📜 License

This project is licensed under **MPL-2.0** — see [`LICENSE`](https://github.com/nikilites/oboromi/blob/main/LICENSE) for details.
