#!/usr/bin/env python3
"""Summarize Wine +virtual traces: big allocations by thread (named via +threadname) and flags.
usage: wine-alloc-report.py <log> [min_mb]"""
import re, sys, collections
log, min_mb = sys.argv[1], float(sys.argv[2]) if len(sys.argv) > 2 else 64
names = {}
alloc = re.compile(r'^([0-9a-f]{4}):trace:virtual:(NtAllocateVirtualMemory(?:Ex)?) 0x[0-9a-f]+ (\S+) ([0-9a-f]+) ([0-9a-f]+) ([0-9a-f]+)')
free = re.compile(r'^([0-9a-f]{4}):trace:virtual:NtFreeVirtualMemory 0x[0-9a-f]+ (\S+) ([0-9a-f]+) ([0-9a-f]+)')
rename = re.compile(r'^([0-9a-f]{4}):warn:threadname:.*renamed to L"([^"]+)"')
big = collections.defaultdict(lambda: [0, 0])
frees = collections.defaultdict(lambda: [0, 0])
with open(log, errors='replace') as f:
    for line in f:
        if ':threadname:' in line:
            m = rename.match(line)
            if m: names[m.group(1)] = m.group(2)
            continue
        m = alloc.match(line)
        if m:
            tid, fn, addr, size, typ, prot = m.groups()
            size = int(size, 16); typ = int(typ, 16)
            if size >= min_mb * 2**20:
                k = (tid, fn, f"type={typ:#x}{' WRITE_WATCH' if typ & 0x200000 else ''}", f"prot={prot}")
                big[k][0] += 1; big[k][1] += size
            continue
        m = free.match(line)
        if m:
            tid, addr, size, typ = m.groups()
            k = (tid, f"free type={typ}")
            frees[k][0] += 1; frees[k][1] += int(size, 16)
print(f"allocations >= {min_mb:g} MB, by thread and flags:")
for k, (n, b) in sorted(big.items(), key=lambda kv: -kv[1][1])[:20]:
    print(f"  {n:6d} calls {b/2**30:8.2f} GB  {k[0]} {names.get(k[0], '?'):<24} {k[1]} {k[2]} {k[3]}")
print("frees, by thread:")
for k, (n, b) in sorted(frees.items(), key=lambda kv: -kv[1][0])[:10]:
    print(f"  {n:6d} calls  {k[0]} {names.get(k[0], '?'):<24} {k[1]}")
