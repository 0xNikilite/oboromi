#!/usr/bin/env python3
"""Pack ambientCG 2K ZIP maps into mipmapped RGBA textures for the dungeon."""
import argparse
import io
import json
import zipfile
from pathlib import Path
from PIL import Image
from gltf_to_raw import mip_chain

p = argparse.ArgumentParser(__doc__)
p.add_argument('source', type=Path)
p.add_argument('out', type=Path)
a = p.parse_args()
a.out.mkdir(parents=True, exist_ok=True)
assets = ['PavingStones142', 'Rock035', 'Metal055C']
manifest = {'license': 'CC0-1.0', 'assets': assets, 'textures': []}
for asset in assets:
    with zipfile.ZipFile(a.source / (asset + '.zip')) as z:
        def read(slot, fallback=None):
            name = f'{asset}_2K-JPG_{slot}.jpg'
            if name not in z.namelist():
                if fallback is None:
                    raise ValueError(f'missing {name}')
                return Image.new('L', (2048, 2048), fallback)
            return Image.open(io.BytesIO(z.read(name))).copy()
        colour = read('Color').convert('RGBA')
        normal = read('NormalGL').convert('RGBA')
        packed = Image.merge('RGBA', (read('AmbientOcclusion', 255).convert('L'), read('Roughness').convert('L'), read('Metalness', 0).convert('L'), read('Displacement').convert('L')))
        for slot, image in [('colour', colour), ('normal', normal), ('surface', packed)]:
            output, _, _, _ = mip_chain(image, slot == 'colour', 2048, 1)
            name = f'{asset}-{slot}.rgba8'
            (a.out / name).write_bytes(output)
            manifest['textures'].append({'file': name, 'bytes': len(output), 'width': 2048, 'height': 2048, 'mips': 12, 'gob_height_log2': 1})
(a.out / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
