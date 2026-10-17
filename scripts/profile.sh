#!/usr/bin/env bash
# Profiles the perf example and writes report.txt and flamegraph.svg (needs `cargo install inferno`) to target/profile/.
set -euo pipefail
cd "$(dirname "$0")/.."
out=target/profile
mkdir -p "$out"
CARGO_PROFILE_RELEASE_DEBUG=1 cargo build --release -p oboromi-core --example perf
perf record -F 999 -g --call-graph dwarf -o "$out/perf.data" -- target/release/examples/perf
perf report -i "$out/perf.data" --no-children --percent-limit 0.5 --stdio > "$out/report.txt"
perf script -i "$out/perf.data" | inferno-collapse-perf | inferno-flamegraph > "$out/flamegraph.svg"
