#!/usr/bin/env python3
"""Extract shader containers from victory's known-shaders.bin by short hash.

Records: stage u8, blake3 [32], length u32 LE, container. The short hash a run
logs is the first 16 hex digits of the blake3 hash. Output stays outside the
repository (asset policy): it is the title's own code.
usage: extract-shader.py <known-shaders.bin> <out-dir> <hash16>... | --list
"""
import os, struct, sys

STAGES = ["vertex", "pixel", "domain", "hull", "geometry", "compute", "amplification", "mesh"]

def walk(path):
    with open(path, "rb") as f:
        data = f.read()
    at = 0
    while at + 37 <= len(data):
        stage = data[at]
        h = data[at + 1:at + 33].hex()
        (n,) = struct.unpack_from("<I", data, at + 33)
        start = at + 37
        if start + n > len(data):
            break
        yield stage, h, data[start:start + n]
        at = start + n

def main():
    src, out, *want = sys.argv[1:]
    if want == ["--list"]:
        for stage, h, c in walk(src):
            print(h[:16], STAGES[stage] if stage < len(STAGES) else stage, len(c))
        return
    want = {w.lower() for w in want}
    found = set()
    for stage, h, c in walk(src):
        if h[:16] in want:
            p = os.path.join(out, f"{h[:16]}.{STAGES[stage] if stage < len(STAGES) else stage}.bin")
            with open(p, "wb") as f:
                f.write(c)
            found.add(h[:16])
            print("wrote", p, len(c))
    for m in sorted(want - found):
        print("not found", m, file=sys.stderr)

main()
