#!/usr/bin/env bash
# Profiles the perf example under perf and writes report.txt and flamegraph.svg to target/profile/.
set -euo pipefail
cd "$(dirname "$0")/.."
out=target/profile
mkdir -p "$out"
CARGO_PROFILE_RELEASE_DEBUG=1 cargo build --release -p oboromi-core --example perf
perf record -F 999 -g --call-graph dwarf -o "$out/perf.data" -- target/release/examples/perf "$@"
perf report -i "$out/perf.data" --no-children --percent-limit 0.5 --stdio > "$out/report.txt"
if command -v inferno-collapse-perf >/dev/null && command -v inferno-flamegraph >/dev/null; then
  perf script -i "$out/perf.data" | inferno-collapse-perf | inferno-flamegraph > "$out/flamegraph.svg"
elif command -v flamegraph >/dev/null; then
  flamegraph --perfdata "$out/perf.data" -o "$out/flamegraph.svg"
else
  echo "no flamegraph tool found, run:"
  echo "  cargo install inferno"
  echo "  perf script -i $out/perf.data | inferno-collapse-perf | inferno-flamegraph > $out/flamegraph.svg"
  exit 0
fi
echo "wrote $out/report.txt and $out/flamegraph.svg"
