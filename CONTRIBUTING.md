updated : 08/08/2026

# contributing to oboromi

Thanks for the interest! oboromi is an (not yet what you probably think) emulator written in Rust

## before you start

You'll need:
- **Rust** stable, latest version ([rustup](https://rustup.rs) if you don't have it)
- **CMake** latest version
- **Ninja** (`winget install Ninja-Build.Ninja` on Windows, or your package manager on Linux/macOS)
- **C++ compiler**: MSVC on Windows (via Visual Studio 2022/2026 Build Tools), Clang on Linux/macOS
- **Qt 6** (6.12.0 or later recommended)

### installing Qt

**Windows:**
Download Qt from [qt.io](https://www.qt.io/development/download-qt-installer-oss) and install it. The default installation path is usually `C:\Qt`. During installation, make sure to select the MSVC 64-bit component (e.g., `msvc2022_64`).

**Linux (Debian/Ubuntu):**
```bash
sudo apt install qt6-base-dev qt6-declarative-dev
```
For other distros, use your package manager.

**macOS:**
```bash
brew install qt@6
```

## building

> **Windows users:** run these commands from the **Developer Command Prompt** or **Developer PowerShell** for Visual Studio (not the regular PowerShell). This ensures `cl.exe` and Ninja are in your `PATH`.

### 1. configure the project

Tell CMake where Qt is installed via `CMAKE_PREFIX_PATH`:

**Windows (adjust the path to your Qt install):**
```bash
cmake -B build -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH="C:/Qt/6.12.0/msvc2022_64"
```

**Linux:**
```bash
cmake -B build -G Ninja -DCMAKE_BUILD_TYPE=Release
```

**macOS:**
```bash
cmake -B build -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH="$(brew --prefix qt@6)"
```

This only needs to be done once, or whenever you change CMake options.

### 2. build the project

same command on all platforms:

```bash
cmake --build build
```

### 3. run the executable

The executable is placed directly in `build/`:

**Windows:**
```bash
.\build\oboromi.exe
```

**Linux/macOS:**
```bash
./build/oboromi
```

## project state (read before opening a PR)

Do not touch Unicorn-related stuff plz, we are working on using a [custom arm emulator](https://github.com/vrtgs/rapid-arm-emu).

## tests

Nothing to mention, but we like tests. If you implement a new feature, make sure to provide a simple way to test it (not required, but would be cool).

## code style

~~- Run `cargo fmt` before committing, please.~~ (skip this for now)

## opening a PR

1. Fork, branch off `main`.
2. commit changes.
3. Open the PR and explain the "why," not just the "what."

## reporting bugs

open an issue with:
- a clear title
- steps to reproduce
- what you expected vs. what actually happened

## license

By contributing, you agree your code gets released under GNU GPLv3, same as the rest of the project.
