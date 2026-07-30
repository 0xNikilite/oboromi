updated : 30/07/2026

# contributing to oboromi

Thanks for the interest! oboromi is an (not yet what you probably think) emulator written in Rust

## before you start

You'll need:
- **Rust** stable, latest version ([rustup](https://rustup.rs) if you don't have it)
- **CMake** better latest version, for the Unicorn engine
- **ninja-build**, if you are on windows just install it with `winget install Ninja-Build.Ninja`, on unix-like system just use your Package Manager (e.g. apt, brew)
- a decent C++ compiler (Clang on Linux/macOS, MSVC on Windows)

clone it and try building:

```bash
git clone https://github.com/0xNikilite/oboromi
cd oboromi
cargo run
```

## Project state (read before opening a PR)

do not touch Unicorn-related stuff plz, we are working on using a [custom arm emulator](https://github.com/vrtgs/rapid-arm-emu)

## tests

nothing to mention, but we like tests, if you implement a new feature make sure to make a simple way to test it (not needed, but would be cool)

## code style

~~- Run `cargo fmt` before committing, please.~~ (skip this for now)

## opening a PR

1. Fork, branch off `main`.
2. commit changes
3. Open the PR and explain the "why," not just the "what."

## reporting bugs

An issue with:
- a clear title
- steps to reproduce
- what you expected vs. what actually happened

## license

By contributing, you agree your code gets released under GNU GPLv3, same as the rest of the project.
