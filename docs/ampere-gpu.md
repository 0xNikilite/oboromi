# ampere gpu

oboromi can replay a GPU command list and copy bytes between guest memory buffers. The copy runs in software, so you can work on the GPU before the ARM core is ready.

## what works

The Ampere channel reads GPFIFO entries and sends method writes to the C7B5 copy engine. It can copy a single span or a rectangle with different source and destination strides, then write a 32-bit completion value. Each surface must fit in one mapping. If a command fails, the replay reports its address.

Eglantine maps the Ampere C7B5 class to its Maxwell DMA engine. Lichii builds on Eglantine. oboromi handles C7B5 directly in Rust. The copy engine also transfers 2D surfaces between pitched rows and native block-linear storage. Compute dispatch, 3D draws and display output are still ahead. [Eglantine engine bindings](https://github.com/Eglantine-Community/Eglantine/blob/nx2/src/video_core/engines/puller.cpp) · [Lichii README](https://github.com/zortos293/Lichii/blob/11021b0604c1c6462eff14147930481556014998/README.md)

## texture demo

Open **Texture demo** in Qt. Use the built-in logo or open a picture, then click **Upload texture**. The right pane shows the raw tiled bytes in memory order. **Read back texture** runs a second C7B5 submission and compares all RGBA bytes with the source.

Images that already measure 128 × 96 pixels keep their size. Other images are resized to fit and centered on the canvas, with padding if their proportions differ. The copy engine uses one 64-byte × 8-row GOB per block. Both steps run in the software copy engine. The channel supports pitched-to-tiled and tiled-to-pitched copies of a single 2D layer without component remapping.

## try a copy

The **Run GPU Tests** button in Qt checks a three-row copy, including row padding and the completion word. To try a smaller copy from the terminal, run this from the repository root. It writes two fixture files in the current directory.

```sh
python3 - <<'PY'
from pathlib import Path
from struct import pack
words = [0x20010000, 0xc7b5, 0x20070100, 0, 0x1040, 0, 0x1050, 0, 0, 4, 0x818100c0]
ram = bytearray(0x54)
ram[:4 * len(words)] = pack(f'<{len(words)}I', *words)
ram[0x40:0x44] = b'gpu!'
Path('example-ram.bin').write_bytes(ram)
Path('example-entries.bin').write_bytes(pack('<Q', 0x1000 | (len(words) << 42)))
PY
cargo run --quiet --locked -p oboromi-core --example replay_gpfifo -- \
    example-entries.bin 0x1050 4 0x1000 example-ram.bin
```

The command prints `gpu!`, the four bytes copied from `0x1040` to `0x1050`. To replay your own submission, supply little-endian 64-bit entries and one address/file pair for each memory mapping.
