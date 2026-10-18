#!/usr/bin/env bash
# Usage: NAK_OFFLINE=/path/to/nak-offline tools/nak-compile.sh <name.vert|name.frag> [out-dir]
set -euo pipefail

src=$(realpath "$1")
out=$(realpath -m "${2:-sass-out}")
stem=${src%.*}
name=$(basename "$stem")
compiler=${NAK_OFFLINE:-nak-offline}
disassembler=${NVDISASM:-nvdisasm}

if [[ -n ${BINDINGS:-} ]]; then
    echo "BINDINGS requires the pending NVK descriptor-lowering shim" >&2
    exit 1
fi

files=("$src")
case "$src" in
    *.vert) files+=("$stem.frag") ;;
    *.frag) files+=("$stem.vert") ;;
    *) echo "expected a .vert or .frag source" >&2; exit 1 ;;
esac

mkdir -p "$out"
work=$(mktemp -d "${TMPDIR:-/tmp}/nak-compile.XXXXXX")
trap 'rm -rf "$work"' EXIT
for file in "${files[@]}"; do
    stage=${file##*.}
    spv="$work/$stage.spv"
    code="$work/$stage.code"
    glslangValidator -V "$file" -o "$spv"
    "$compiler" "$stage" "$spv" "$out/$name.$stage.bin"
    dd if="$out/$name.$stage.bin" of="$code" bs=128 skip=1 status=none
    "$disassembler" -b SM86 "$code" > "$out/$name.$stage.sass"
done

ls -la "$out/$name."*
