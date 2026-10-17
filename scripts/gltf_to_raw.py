#!/usr/bin/env python3
"""Convert a .gltf/.glb into raw buffers for the GPU demos.

Requires: pip install numpy Pillow

Usage: gltf_to_raw.py MODEL.glb OUTDIR [--max-size 512] [--readme README.md]
       [--source-url URL] [--author TEXT] [--license TEXT]

Writes vertices.bin (f32 position3 normal3 tangent4 uv2 per vertex), indices.bin (u32),
one <slot>.rgba8 per distinct image (pitch-linear RGBA8, full mip chain level after level)
and manifest.json. Node transforms are not applied.
"""
import argparse, json, re, struct, sys
from pathlib import Path
import numpy as np
from PIL import Image

CT = {5120: np.int8, 5121: np.uint8, 5122: np.int16, 5123: np.uint16, 5125: np.uint32, 5126: np.float32}
NC = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT2": 4, "MAT3": 9, "MAT4": 16}
SLOTS = ["baseColor", "metallicRoughness", "normal", "occlusion", "emissive"]
SRGB = {"baseColor", "emissive"}


def load(path):
    path = Path(path)
    raw = path.read_bytes()
    bins = []
    if raw[:4] == b"glTF":
        off, js = 12, None
        while off < len(raw):
            n, t = struct.unpack_from("<II", raw, off)
            body = raw[off + 8 : off + 8 + n]
            if t == 0x4E4F534A:
                js = json.loads(body)
            elif t == 0x004E4942:
                bins.append(body)
            off += 8 + n
        g = js
    else:
        g = json.loads(raw)
    bufs = []
    for i, b in enumerate(g.get("buffers", [])):
        uri = b.get("uri")
        bufs.append(bins[0] if uri is None else (_data_uri(uri) if uri.startswith("data:") else (path.parent / uri).read_bytes()))
    return g, bufs, path.parent


def _data_uri(uri):
    import base64
    return base64.b64decode(uri.split(",", 1)[1])


def accessor(g, bufs, idx):
    a = g["accessors"][idx]
    dt, nc = np.dtype(CT[a["componentType"]]), NC[a["type"]]
    n = a["count"]
    if "bufferView" in a:
        bv = g["bufferViews"][a["bufferView"]]
        base = bv.get("byteOffset", 0) + a.get("byteOffset", 0)
        stride = bv.get("byteStride") or dt.itemsize * nc
        buf = bufs[bv["buffer"]]
        rows = np.empty((n, nc * dt.itemsize), np.uint8)
        view = np.frombuffer(buf, np.uint8)
        rows[:] = np.lib.stride_tricks.as_strided(view[base:], (n, nc * dt.itemsize), (stride, 1))
        out = rows.view(dt).reshape(n, nc)
    else:
        out = np.zeros((n, nc), dt)
    if a.get("normalized"):
        info = np.iinfo(dt)
        out = np.maximum(out.astype(np.float32) / info.max, -1.0)
    return out


def tangents(pos, nrm, uv, idx):
    t = np.zeros((len(pos), 4), np.float32)
    if not np.any(uv):
        return t
    tri = idx.reshape(-1, 3)
    p0, p1, p2 = pos[tri[:, 0]], pos[tri[:, 1]], pos[tri[:, 2]]
    u0, u1, u2 = uv[tri[:, 0]], uv[tri[:, 1]], uv[tri[:, 2]]
    e1, e2, d1, d2 = p1 - p0, p2 - p0, u1 - u0, u2 - u0
    det = d1[:, 0] * d2[:, 1] - d2[:, 0] * d1[:, 1]
    r = np.where(np.abs(det) > 1e-12, 1.0 / np.where(det == 0, 1, det), 0.0)[:, None]
    tf = (e1 * d2[:, 1:2] - e2 * d1[:, 1:2]) * r
    bf = (e2 * d1[:, 0:1] - e1 * d2[:, 0:1]) * r
    ta, ba = np.zeros_like(pos), np.zeros_like(pos)
    for k in range(3):
        np.add.at(ta, tri[:, k], tf)
        np.add.at(ba, tri[:, k], bf)
    ot = ta - nrm * np.sum(nrm * ta, 1, keepdims=True)
    ln = np.linalg.norm(ot, axis=1, keepdims=True)
    fallback = np.cross(nrm, np.where(np.abs(nrm[:, :1]) < 0.9, [[1, 0, 0]], [[0, 1, 0]]))
    ot = np.where(ln > 1e-12, ot / np.where(ln == 0, 1, ln), fallback / np.maximum(np.linalg.norm(fallback, axis=1, keepdims=True), 1e-12))
    w = np.where(np.sum(np.cross(nrm, ot) * ba, 1) < 0, -1.0, 1.0)
    t[:, :3], t[:, 3] = ot, w
    return t


def smooth_normals(pos, idx):
    tri = idx.reshape(-1, 3)
    fn = np.cross(pos[tri[:, 1]] - pos[tri[:, 0]], pos[tri[:, 2]] - pos[tri[:, 0]])
    n = np.zeros_like(pos)
    for k in range(3):
        np.add.at(n, tri[:, k], fn)
    return n / np.maximum(np.linalg.norm(n, axis=1, keepdims=True), 1e-12)


def s2l(x):
    return np.where(x <= 0.04045, x / 12.92, ((x + 0.055) / 1.055) ** 2.4)


def l2s(x):
    return np.where(x <= 0.0031308, x * 12.92, 1.055 * np.power(np.maximum(x, 0), 1 / 2.4) - 0.055)


def box(arr, w, h):
    ch = [np.asarray(Image.fromarray(arr[..., c], "F").resize((w, h), Image.BOX)) for c in range(4)]
    return np.stack(ch, -1)


def mip_chain(img, srgb, max_size):
    a = np.asarray(img.convert("RGBA"), np.float32) / 255.0
    if srgb:
        a[..., :3] = s2l(a[..., :3])
    h, w = a.shape[:2]
    s = min(1.0, max_size / max(w, h))
    if s < 1.0:
        w, h = max(1, round(w * s)), max(1, round(h * s))
        a = box(a, w, h)
    levels = [a]
    while max(w, h) > 1:
        w, h = max(1, w // 2), max(1, h // 2)
        a = box(a, w, h)
        levels.append(a)
    out = b""
    for l in levels:
        l = l.copy()
        if srgb:
            l[..., :3] = l2s(l[..., :3])
        out += np.clip(np.rint(l * 255), 0, 255).astype(np.uint8).tobytes()
    return out, levels[0].shape[1], levels[0].shape[0], len(levels)


def readme_meta(path, args):
    meta = {"source_url": args.source_url, "author": args.author, "license": args.license}
    if path and Path(path).exists():
        t = Path(path).read_text()
        m = re.search(r"\[Download[^\]]*\]\((\S+?)\)", t)
        if m and not meta["source_url"]:
            meta["source_url"] = m.group(1)
        m = re.search(r"&copy;[^\n]*?\[([^\]]+)\]\(", t)
        if m and not meta["license"]:
            meta["license"] = m.group(1)
        m = re.search(r"^\s*-\s+(.+?)\s+for\s+", t, re.M)
        if m and not meta["author"]:
            meta["author"] = m.group(1)
    return meta


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("model")
    ap.add_argument("outdir")
    ap.add_argument("--max-size", type=int, default=512)
    ap.add_argument("--readme")
    ap.add_argument("--source-url")
    ap.add_argument("--author")
    ap.add_argument("--license")
    args = ap.parse_args()
    g, bufs, base = load(args.model)
    out = Path(args.outdir)
    out.mkdir(parents=True, exist_ok=True)

    verts, inds, prims, vbase = [], [], [], 0
    for mi, mesh in enumerate(g["meshes"]):
        for pi, p in enumerate(mesh["primitives"]):
            if p.get("mode", 4) != 4:
                sys.exit(f"mesh {mi} primitive {pi}: only triangle lists supported")
            at = p["attributes"]
            pos = accessor(g, bufs, at["POSITION"]).astype(np.float32)
            idx = accessor(g, bufs, p["indices"]).reshape(-1).astype(np.uint32) if "indices" in p else np.arange(len(pos), dtype=np.uint32)
            nrm = accessor(g, bufs, at["NORMAL"]).astype(np.float32) if "NORMAL" in at else smooth_normals(pos, idx)
            uv = accessor(g, bufs, at["TEXCOORD_0"]).astype(np.float32) if "TEXCOORD_0" in at else np.zeros((len(pos), 2), np.float32)
            tan = accessor(g, bufs, at["TANGENT"]).astype(np.float32) if "TANGENT" in at else tangents(pos, nrm, uv, idx)
            verts.append(np.hstack([pos, nrm, tan, uv]).astype(np.float32))
            prims.append({"mesh": mi, "primitive": pi, "material": p.get("material"), "vertex_offset": vbase,
                          "vertex_count": len(pos), "first_index": sum(len(i) for i in inds), "index_count": len(idx)})
            inds.append(idx + vbase)
            vbase += len(pos)
    V, I = np.vstack(verts), np.concatenate(inds)
    (out / "vertices.bin").write_bytes(V.tobytes())
    (out / "indices.bin").write_bytes(I.astype("<u4").tobytes())

    mats = []
    img_slots = {}  # image index -> [(material, slot)]
    for mi, m in enumerate(g.get("materials", [])):
        pbr = m.get("pbrMetallicRoughness", {})
        refs = {"baseColor": pbr.get("baseColorTexture"), "metallicRoughness": pbr.get("metallicRoughnessTexture"),
                "normal": m.get("normalTexture"), "occlusion": m.get("occlusionTexture"), "emissive": m.get("emissiveTexture")}
        tex = {}
        for s, r in refs.items():
            if r is not None:
                src = g["textures"][r["index"]]["source"]
                tex[s] = src
                img_slots.setdefault(src, []).append((mi, s))
        mats.append({"index": mi, "name": m.get("name"), "baseColorFactor": pbr.get("baseColorFactor", [1, 1, 1, 1]),
                     "metallicFactor": pbr.get("metallicFactor", 1.0), "roughnessFactor": pbr.get("roughnessFactor", 1.0),
                     "emissiveFactor": m.get("emissiveFactor", [0, 0, 0]), "normalScale": m.get("normalTexture", {}).get("scale", 1.0),
                     "occlusionStrength": m.get("occlusionTexture", {}).get("strength", 1.0),
                     "alphaMode": m.get("alphaMode", "OPAQUE"), "textures": {s: f"{src_name(img_slots, src)}.rgba8" for s, src in tex.items()}})
    textures = []
    for src, uses in sorted(img_slots.items()):
        slots = sorted({s for _, s in uses}, key=SLOTS.index)
        name = slots[0]
        srgb = name in SRGB
        im = g["images"][src]
        if "bufferView" in im:
            bv = g["bufferViews"][im["bufferView"]]
            o = bv.get("byteOffset", 0)
            data = bufs[bv["buffer"]][o : o + bv["byteLength"]]
            import io
            img = Image.open(io.BytesIO(data))
        else:
            img = Image.open(base / im["uri"])
        blob, w, h, n = mip_chain(img, srgb, args.max_size)
        (out / f"{name}.rgba8").write_bytes(blob)
        textures.append({"file": f"{name}.rgba8", "image": src, "slots": slots, "width": w, "height": h, "mips": n,
                         "color_space": "srgb" if srgb else "linear", "bytes": len(blob)})
    man = {"vertex_count": len(V), "index_count": len(I), "vertex_stride": 48,
           "vertex_layout": "position f32x3, normal f32x3, tangent f32x4, uv f32x2",
           "primitives": prims, "materials": mats, "textures": textures, **readme_meta(args.readme, args)}
    (out / "manifest.json").write_text(json.dumps(man, indent=1))
    for f in sorted(out.iterdir()):
        print(f"{f.name}: {f.stat().st_size}")


def src_name(img_slots, src):
    return sorted({s for _, s in img_slots[src]}, key=SLOTS.index)[0]


if __name__ == "__main__":
    main()
