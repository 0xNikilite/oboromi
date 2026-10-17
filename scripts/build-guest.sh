#!/bin/sh
# Rebuilds core/tests/fixtures/guest/guest.bin, the flat AArch64 image of guest/, and prints its sha256.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
rustup target list --installed | grep -qx aarch64-unknown-none || rustup target add aarch64-unknown-none
objcopy="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin/rust-objcopy"
rustup component list --installed | grep -q "^llvm-tools" || rustup component add llvm-tools
cargo build --release --target aarch64-unknown-none --manifest-path "$root/guest/Cargo.toml" --target-dir "$root/target/guest"
mkdir -p "$root/core/tests/fixtures/guest"
"$objcopy" -O binary "$root/target/guest/aarch64-unknown-none/release/guest" "$root/core/tests/fixtures/guest/guest.bin"
sha256sum "$root/core/tests/fixtures/guest/guest.bin"
