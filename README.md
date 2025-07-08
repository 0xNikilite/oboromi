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

> \[!WARNING]
> oboromi is **not** yet a playable emulator. Right now it’s a skeleton of CPU+MMU cores and a test harness, designed to grow into a full emulator as hardware/software details (and reverse‑engineering) become available.

## 🚀 What’s in the box today

- **CPU Core**  
  - Implements a subset of ARM64/AArch64 (15+ instructions: NOP, ADDI/SUBI, ADD/SUB, AND/ORR/EOR, CMP/TST, LDR/STR, B, RET)  
  - Full NZCV flag handling  
  - Optional `trace` feature for disassembly and PC‐tagged logs

- **Virtual Memory + MMU**  
  - 4 KiB pages with identity mapping  
  - Simple page table + TLB (64 entries)  
  - All memory reads/writes go through the MMU

- **Memory Subsystem**  
  - Bounds‐checked reads/writes  
  - 32‑bit and 64‑bit little‑endian helpers  

- **Test Harness**  
  - `main.rs` runs 12 quick integration tests (CPU instructions + MMU)  
  - `cargo test` covers unit tests for CPU + Memory  

## 🧪 Try it

```bash
git clone https://github.com/0xNikilite/oboromi
cd oboromi
cargo run --features trace   # see each instruction disassembled
# or simply:
cargo run
````

## 🤝 Contributing

We’re actively looking for collaborators in these areas:

<ul>
  <li>
    <strong>ARM64/AArch64 Architecture &amp; CPU Implementation</strong>
  </li>
  <li>
    <strong>Memory Management &amp; Virtualization</strong>
  </li>
  <li>
    <strong>Graphics &amp; GPU Backends</strong><br>
    Experience with low‑level graphics APIs (Vulkan, Metal, DirectX) and shader pipeline emulation.
  </li>
  <li>
    <strong>Firmware &amp; Hardware Reverse Engineering</strong><br>
    Skills in extracting, analyzing, and documenting proprietary firmware, SoC internals, and board‑level schematics.
  </li>
  <li>
    <strong>Rust Systems Programming</strong><br>
    Passion for zero‑unsafe, high‑performance Rust code.
  </li>
</ul>

## 📜 License

This project is licensed under **MPL‑2.0**.
See [LICENSE](https://github.com/0xNikilite/oboromi/blob/main/LICENSE).
