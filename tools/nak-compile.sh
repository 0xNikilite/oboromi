#!/usr/bin/env bash
# Mesa 25.2.7: curl -LO https://archive.mesa3d.org/mesa-25.2.7.tar.xz && tar xf mesa-25.2.7.tar.xz
# cp tools/nak-offline.c mesa-25.2.7/src/nouveau/compiler/; append this target to that directory's meson.build:
# executable('nak-offline', files('nak_offline.c'), include_directories : [inc_include, inc_src], dependencies : [idep_nak, idep_vtn, idep_mesautil])
# meson setup mesa-25.2.7/build mesa-25.2.7 -Dgallium-drivers=nouveau -Dvulkan-drivers=nouveau -Dplatforms=x11 -Dbuild-tests=false
# ninja -C mesa-25.2.7/build src/nouveau/compiler/nak-offline
# Usage: NAK_OFFLINE=/path/to/nak-offline tools/nak-compile.sh <name.vert|name.frag> [out-dir]
set -euo pipefail

src=$(realpath "$1")
out=$(realpath -m "${2:-sass-out}")
stem=${src%.*}
name=$(basename "$stem")
compiler=${NAK_OFFLINE:-nak-offline}
disassembler=${NVDISASM:-nvdisasm}

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
    "$compiler" "$stage" "$spv" "$out/$name.$stage.bin" "${BINDINGS:-}"
    dd if="$out/$name.$stage.bin" of="$code" bs=128 skip=1 status=none
    "$disassembler" -b SM86 "$code" > "$out/$name.$stage.sass"
done

ls -la "$out/$name."*
