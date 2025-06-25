<p align="center">
  <img src="https://dummyimage.com/400x100/000/fff&text=oboromi" alt="oboromi logo" width="50%" />
</p>

<p align="center">
  <a href="https://github.com/0xNikilite/oboromi/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/license-MPL%202.0-blue.svg" alt="License" />
  </a>
  &nbsp;
  <a href="https://discord.gg/g9sehj8bPz">
    <img src="https://img.shields.io/discord/1387476383663390732?style=flat&label=Discord&color=5865F2&logo=discord&logoColor=white" alt="Discord" />
  </a>
</p>

<h1 align="center">oboromi</h1>
<h3 align="center">A proof-of-concept Nintendo Switch 2 emulator written in Rust</h3>
<h4 align="center"><em>Very WIP — nothing works yet, don’t expect miracles.</em></h4>

## Overview

**oboromi** is a clean and modular emulator framework targeting the **Nintendo Switch 2**, written in Rust for maximum maintainability and performance.  
Its goal is to simulate the new hardware step-by-step, following real architectural assumptions (when available) and clean software design.

## Features (so far)

- ✅ **Memory subsystem**  
  • Bounds-checked byte and word reads  
  • 32-bit and 64-bit little-endian access (`read_u32`/`write_u32`, `read_u64`/`write_u64`)  
- ✅ **CPU core**  
  • Full ARM64 register file (X0–X30 + SP, PC)  
  • NZCV flag support (Negative, Zero, Carry, Overflow)  
- ✅ **Implemented instructions**  
  - **NOP**  
  - **MOV Xd, #imm** (ORR Xd, XZR, #imm)  
  - **ADDI / SUBI**: immediate arithmetic with flags  
  - **ADD / SUB**: register-form arithmetic with flags  
  - **AND / ORR / EOR**: logical register-form  
  - **CMP (SUBS XZR, Xn, Xm)** and **TST (ANDS XZR, Xn, Xm)**  
  - **LDR / STR** immediate 64-bit loads/stores  
  - **B** (unconditional branch) and **RET** (return)  
- ✅ **Integration tests**  
  • Extensive `main.rs` harness exercises each instruction path  
- 🧱 **Modular architecture**  
  • Ready for rapid addition of more instructions, subsystems, and peripherals  

## 🧪 Try It

```bash
git clone https://github.com/0xNikilite/oboromi
cd oboromi
cargo run
````

You should see:

```
✅ Memory OK  
✅ NOP OK  
✅ ADDI OK  
✅ SUBI OK  
✅ ADDR OK  
✅ SUBR OK  
✅ AND OK  
✅ CMP OK  
✅ LDR/STR OK  
✅ B OK  
✅ RET OK  
```

## 🔍 Current Target Hardware (Known)

| Component   | Details                           |
| ----------- | --------------------------------- |
| **SoC**     | NVIDIA GMLX30-A1                  |
| **RAM**     | 12 GB LPDDR5X (2 × 6 GB SK hynix) |
| **Storage** | UFS 3.1 (Kioxia or SK hynix)      |
| **WiFi/BT** | MediaTek MT3681AEN                |
| **Audio**   | Realtek ALC5658                   |
| **Voice**   | Intelligo IG2200                  |
| **Power**   | MAX77851 + DA9092                 |
| **USB**     | Genesys GL852G + Cypress CYPD6228 |
| **GC ASIC** | B2349 GCBRG HAC STD T2010423      |

---

> \[!NOTE]
> Only the comments are “vibe-written” — the code itself is fucking handcrafted in Rust.

> \[!WARNING]
> This project **does not** include any proprietary firmware, keys, or dumps. Everything is clean-room.

---

## 📜 License

This project is licensed under **MPL-2.0** — see [`LICENSE`](https://github.com/0xNikilite/oboromi/blob/main/LICENSE) for details.
