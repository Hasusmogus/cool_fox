#!/usr/bin/env python3
"""Profile report for a victory run recorded with `PERF=1 ./play-linux.sh`.

Wine copies PE images into anonymous memory, so perf names neither victory.exe's
functions nor the title's. This maps sampled addresses back:
  - victory.exe (at its preferred base, 0x140000000) -> Rust functions via the PDB
    (llvm-symbolizer reads it natively);
  - the title's image (base from the run log) -> offsets in it (no symbols);
  - everything else -> perf's own DSO/symbol.

usage: perf-report.py <perf.data> <victory.exe> <run.log> [--since SEC] [--top N]
Prints self time by area and by function, and inclusive time of victory.exe functions.
"""
import collections, re, subprocess, sys

args = sys.argv[1:]
data, exe, log = args[0], args[1], args[2]
since = float(args[args.index("--since") + 1]) if "--since" in args else 0.0
top = int(args[args.index("--top") + 1]) if "--top" in args else 40
comm = args[args.index("--comm") + 1] if "--comm" in args else None

VICTORY_BASE = 0x140000000
# where Wine put victory.exe (`WINEDEBUG=+loaddll`: "Loaded L"...victory.exe" at <address>")
with open(log, errors="replace") as f:
    for line in f:
        m = re.search(r'Loaded L"[^"]*victory\.exe" at ([0-9a-fA-F]+)', line)
        if m:
            VICTORY_BASE = int(m.group(1), 16)
            break
# victory.exe's size of image from its PE header
with open(exe, "rb") as f:
    pe = f.read(4096)
off = int.from_bytes(pe[0x3C:0x40], "little")
size_of_image = int.from_bytes(pe[off + 24 + 56: off + 24 + 60], "little")
PREFERRED = int.from_bytes(pe[off + 24 + 24: off + 24 + 32], "little")  # PE32+ ImageBase
VICTORY_END = VICTORY_BASE + size_of_image
print(f"victory.exe at {VICTORY_BASE:#x}")

# title image: "image mapped base=0x... preferred=0x140000000 path=...victory-image-<hash>.dll" (the first one is the exe)
title = None
ansi = re.compile(r"\x1b\[[0-9;]*m")
with open(log, errors="replace") as f:
    for line in f:
        line = ansi.sub("", line)
        m = re.search(r"image mapped base=(0x[0-9a-f]+) preferred=0x140000000", line)
        if m:
            title = int(m.group(1), 16)
            break
TITLE_SIZE = 0xB06D000  # SizeOfImage of game_scarlett_master.exe (victory inspect)

def area(ip, dso):
    if VICTORY_BASE <= ip < VICTORY_END:
        return "victory.exe"
    if title and title <= ip < title + TITLE_SIZE:
        return "title"
    if dso.startswith("[JIT]") or dso == "[unknown]":
        return "other PE (dlls)"
    return dso.rsplit("/", 1)[-1]

# perf script: one sample = header line, then stack lines "\t addr sym (dso)"
proc = subprocess.Popen(["perf", "script", "-i", data, "-F", "time,ip,sym,dso", "--no-demangle"] + (["--comms", comm] if comm else []),
                        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, errors="replace")
frame = re.compile(r"^\s+([0-9a-f]+)\s+(.*?)\s+\((.*)\)\s*$")
self_area = collections.Counter()
self_sym = collections.Counter()
incl_victory = collections.Counter()
victory_self_ips = collections.Counter()
first_time = None
samples = 0
stack = []
t = 0.0

def flush():
    global samples
    if not stack or t - (first_time or t) < since:
        return
    samples += 1
    ip, sym, dso = stack[0]
    a = area(ip, dso)
    self_area[a] += 1
    if a == "victory.exe":
        victory_self_ips[ip] += 1
    elif a == "title":
        self_sym[f"title+{ip - title:#x}"[:-3] + "xxx"] += 1  # 4 KiB buckets
    else:
        self_sym[f"{a}: {sym}"] += 1
    seen = set()
    for ip2, _, dso2 in stack:
        if VICTORY_BASE <= ip2 < VICTORY_END and ip2 not in seen:
            seen.add(ip2)
            incl_victory[ip2] += 1

for line in proc.stdout:
    if not line.strip():
        flush()
        stack = []
        continue
    m = frame.match(line)
    if m:
        stack.append((int(m.group(1), 16), m.group(2), m.group(3)))
        continue
    tm = re.search(r"(\d+\.\d+):", line)
    if tm:
        t = float(tm.group(1))
        if first_time is None:
            first_time = t
flush()

def symbolize(ips):
    ips = list(ips)
    if not ips:
        return {}
    out = subprocess.run(["llvm-symbolizer", f"--obj={exe}", "--no-inlines", "--functions=short"],
                         input="\n".join(hex(i - VICTORY_BASE + PREFERRED) for i in ips), capture_output=True, text=True).stdout
    names = [blk.split("\n")[0] for blk in out.strip().split("\n\n")]
    return dict(zip(ips, names))

print(f"samples: {samples} (from {since:.0f} s after the first)")
print("\nself time by area:")
for a, n in self_area.most_common():
    print(f"  {100 * n / samples:6.2f}%  {a}")

# victory.exe self time by function
names = symbolize(victory_self_ips.keys())
by_fn = collections.Counter()
for ip, n in victory_self_ips.items():
    by_fn[names.get(ip, hex(ip))] += n
print(f"\nself time, top {top} functions (victory.exe by name; others by DSO/symbol; title by 4 KiB block):")
merged = collections.Counter({f"victory: {k}": v for k, v in by_fn.items()})
merged.update(self_sym)
for k, n in merged.most_common(top):
    print(f"  {100 * n / samples:6.2f}%  {k[:150]}")

# inclusive time of victory.exe functions
inames = symbolize(incl_victory.keys())
incl_fn = collections.Counter()
# a function counts once per sample: approximate by summing per-ip counts per name
for ip, n in incl_victory.items():
    incl_fn[inames.get(ip, hex(ip))] += n
print(f"\ninclusive time, top {top} victory.exe functions (a function's callees counted in):")
for k, n in incl_fn.most_common(top):
    print(f"  {100 * n / samples:6.2f}%  {k[:150]}")
