#!/bin/sh
# Fetches the converted FlightHelmet (512 px, block-linear, full mips) from the fork's assets branch.
set -e
dir=$(dirname "$0")/flighthelmet
[ -f "$dir/manifest.json" ] && exit 0
mkdir -p "$dir"
curl -fL https://raw.githubusercontent.com/jopdorp/oboromi/f6a01e5cb5efbb4603ebc62afeb5eebecc77fae4/flighthelmet.tar.gz -o "$dir/flighthelmet.tar.gz"
echo "4b37e3b926a246c6d8ad8ce449b4f8c9470b953d4533865c6deb8ca0815c2dd7  $dir/flighthelmet.tar.gz" | sha256sum -c -
tar -xzf "$dir/flighthelmet.tar.gz" -C "$dir" && rm "$dir/flighthelmet.tar.gz"
