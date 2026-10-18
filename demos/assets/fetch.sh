#!/bin/sh
# Fetches the converted BoomBox (2048 px) and FlightHelmet (1024 px), block-linear with full mips, from the fork's assets branch.
set -e
fetch() {
    dir=$(dirname "$0")/$1
    [ -f "$dir/manifest.json" ] && return
    mkdir -p "$dir"
    curl -fL https://raw.githubusercontent.com/jopdorp/oboromi/f00cf49e1e03cc2f3728f73ea0c09a81a27ddb1f/$1.tar.gz -o "$dir/$1.tar.gz"
    echo "$2  $dir/$1.tar.gz" | sha256sum -c -
    tar -xzf "$dir/$1.tar.gz" -C "$dir" && rm "$dir/$1.tar.gz"
}
fetch boombox 8256f5d473da5d0df8a7f36c9d85f6a1eeaeaa43b8e6ae99bc0aa4e9579c55c7
fetch flighthelmet 9c409e8ceeb8e5f8500f4d901f4dbb4abb5026ca464f589dbc584f2965dfa1b4

fetch_dungeon() {
    dir=$(dirname "$0")/dungeon
    [ -f "$dir/manifest.json" ] && return
    mkdir -p "$dir"
    curl -fL https://raw.githubusercontent.com/jopdorp/oboromi/e6cb14cfdd2fe60ca233fd5e1eb9d99f42e6b846/dungeon-materials.tar.xz -o "$dir/materials.tar.xz"
    echo "de139bb5257dc508b5f05308511ece3d14717529b101dbe36fc30d9db34bef4e  $dir/materials.tar.xz" | sha256sum -c -
    tar -xJf "$dir/materials.tar.xz" -C "$dir" && rm "$dir/materials.tar.xz"
}
fetch_dungeon

fetch_dungeon_audio() {
    dir=$(dirname "$0")/dungeon-audio
    [ -f "$dir/SOURCES" ] && return
    mkdir -p "$dir"
    curl -fL https://raw.githubusercontent.com/jopdorp/oboromi/0a0fed7aa8769da0ff28d1e529201bd7087fd49a/dungeon-audio.tar.xz -o "$dir/audio.tar.xz"
    echo "1e426c7c3a0f307bc55a2b9dd9a42dbddb0c02a78df7f8a81c1f8ac84dd6b0d6  $dir/audio.tar.xz" | sha256sum -c -
    tar -xJf "$dir/audio.tar.xz" -C "$dir" && rm "$dir/audio.tar.xz"
}
fetch_dungeon_audio
