<p align="center">
  <img width="32%" src="https://github.com/user-attachments/assets/2cf6431e-e9a5-4f03-98ce-d8c975ddde77" alt="oboromi logo"/>
</p>

<p align="center">
  <a href="https://github.com/0xNikilite/oboromi/blob/main/LICENSE"><img alt="License" src="https://img.shields.io/badge/license-GPL--3.0-blue.svg?style=flat"></a>
  <a href="https://discord.gg/g9sehj8bPz"><img alt="Discord" src="https://img.shields.io/discord/1387476383663390732?style=flat&label=Discord&color=5865F2&logo=discord&logoColor=white"></a>
</p>

<h4 align="center">(◕‿◕)&nbsp;&nbsp;join our discord ↑</h4>

<h1 align="center">oboromi</h1>
<h4 align="center">a work-in-progress nintendo switch 2 emulator foundation</h4>
<h5 align="center">written lowercase.</h5>

oboromi is an experimental emulator written primarily in Rust, with a Qt 6 debugging frontend.

## current focus

we are developing a new [ARMv9 CPU emulator](https://github.com/vrtgs/rapid-arm-emu) (credit: vrtgs), targeting the ARM extensions required by the ns2's T239.

the GPU implementation targets NVIDIA SM86.

oboromi cannot currently run commercial games or firmware.

## repository

- `core/` Rust emulator core
- `gui-qt/` cpp Qt 6 frontend
- `scripts/` code-generation and helper scripts
- `docs/` documentation

## building

Requirements: stable Rust, CMake, Ninja, a C++ compiler, and Qt 6.

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for setup and build instructions.

## license

GPL-3.0. See [`license.txt`](license.txt).

## what does the name mean?

**oboromi (朧実)** is a constructed japanese neologism combining **朧 (oboro**, "hazy/vague") + **実 (-mi**, "essence/fruit").

it means **"the tangible essence of imperfection."** even if oboromi was ever finished, it could never be perfect, and that's not a failure, just what it means to be an emulator.
