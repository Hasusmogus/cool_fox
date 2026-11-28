# victory-linux-nvidia

**The VIctory Xbox Series runtime on Linux, tuned for NVIDIA RTX cards.**

A fork of [cool_fox](https://github.com/DoubleGhosty/cool_fox) (the CGBJ runtime continuation)
that runs under Wine with vkd3d-proton. It carries the Linux fixes the runtime needs and the
best measured changes from across the VIctory forks, each one tested on the same machine and the
same scene before it was kept.

> **No game files are included or distributed.** You need your own legal dump of the game.
> Never download a prebuilt "GTA VI PC" package: build from source.

[![Support on Ko-fi](https://img.shields.io/badge/Support%20on-Ko--fi-FF5E5B?logo=ko-fi&logoColor=white)](https://ko-fi.com/danf490581)

Development runs on paid AI tokens and long test sessions. If you'd like to help cover those costs,
you can [support the work on Ko-fi](https://ko-fi.com/danf490581). Everything here is and stays free:
donations buy nothing but time spent on the next fix.

---

## Where it stands

Measured in the opening prison scenes (Series X profile, 1080p output) on the test machine below.
Every number is from a scripted run: launch, wait for the scene, measure 150 s.

| | Start of the tuning (run 56) | Runs 85–90 | Now (runs 113–120) |
|---|---|---|---|
| Steady gameplay | ~128 ms a frame (7.8 fps) | ~51 ms (~19 fps) | **~43 ms (~23 fps)** |
| GPU busy | | ~87–92 % | ~68 % |
| Draws a frame | ~3,700 | ~4,000–4,800 | ~4,800 |

**Working:** boot, menus, cutscenes, gameplay in the first areas, lit interiors, faces with moving
mouths, NPC heads and clothing, audio.

**Known issues:** some interiors still darker than on console; coloured speckles ("confetti") on one
NPC's face in some runs; some texture and lighting pop-in after camera cuts; no heat haze (left out
for speed, see below). The frame is now limited by the thread that prepares and submits GPU work
(~41 ms of work a frame); the GPU has headroom (~68 % busy). 30 fps needs that thread ~10 ms faster.

**Test machine:** Arch Linux, KDE Plasma (Wayland), AMD Ryzen 5 7600X (12 threads), NVIDIA GeForce
RTX 5070 12 GB, Wine-staging 11.22 with a patched wineserver, vkd3d-proton (master), DXVK's dxgi.

---

## What this fork changes

Measured one at a time against the run before (frame times are steady gameplay unless noted).

### Speed

| Change | Source | Effect |
|---|---|---|
| Small fills the title wrote itself go on without draining every flight (`VICTORY_FILL_TITLE=1`) | upstream switch, turned on | waits for the card ~65 ms → ~0 a frame |
| Memory profile for 10–14 GB NVIDIA cards: more title buffers in VRAM (`VICTORY_CARD_BUFFERS_MB=1536`, `VICTORY_CARD_WINDOWS_MB=1024`) | [Vice-Tay](https://github.com/TaynanGT/Vice-Tay) #339 | **110–120 → 58 ms**; copy-back 38 → 15 ms |
| Structure builder off with the frame-estimated ambient light (`VICTORY_BVH_BUILDER=0`, `VICTORY_AMBIENT=1`) | upstream switches | 58 → 51–55 ms, and lit interiors |
| Six helper threads for refreshes and copies (`VICTORY_WIDE_WORKERS=6`) | this fork | ~1–2 ms |
| Ray walks end at the root while the builder is off (`VICTORY_BVH_MISS=1`) | VIctory-NVIDIA 24a479d32, via [Victory-United](https://github.com/Fayberr/Victory-United) | card time not spent walking empty structures |
| A wineserver that keeps a process's views in a tree (`linux/wine/`) | this fork | server CPU 40 % → 3–11 % (the title maps ~1,000 views a second) |
| Wave64 programs on 32-lane NVIDIA cards: pair meetings, alternating exchange slots | this fork | a defrag shader 32 → 11.6 ms of card time |
| The submitting thread looks only at card-side buffer copies touched since its last submission, not all ~4,000 | this fork | 51.3 → 49.1 ms (it was 10 % of that thread's time) |
| Textures guarded instead of hashed whole at every check (`VICTORY_TEXTURE_GUARD=1`) | upstream switch, turned back on | 153 → 15 MB hashed a frame; preparing 46 → 36 ms |
| Decal-heap defrag, heat haze and the water's tiled light left out (`VICTORY_LEAVE_OUT`; set it empty to run them) | VIctory-NVIDIA | the defrag alone was 13.7 ms of card time a frame: GPU 92 → 68 % busy, 48 → 45.5 ms |
| The descriptor-heap thread makes a round after 1 ms without a submission (`VICTORY_HEAP_SYNC_EARLY=1`) | upstream switch, tuned | 44.6 → 43.3 ms |
| 16-byte SSE2 `memcmp` under Wine (Wine's compares 8 bytes a step) | cool_fox d78b9eb5 | no measurable change here; kept |

### Picture

| Change | Source | Effect |
|---|---|---|
| Textures without levels on demand (`VICTORY_LEVELS_ON_DEMAND=0`) | upstream switch, turned off | unbacked tiled levels read zero under vkd3d-proton/NVIDIA: **black rooms, walls, clothes and NPC heads fixed** |
| Ray programs' fault pages raised from 2,048 to 16,384 for the run | this fork | pages read as zero after a few minutes: dark, speckled indirect light fixed |
| 16-bit values moved bit-exactly through typed views (`HostShader::half_views`) | [VIctory fork](https://github.com/fxceless/victory-fork) 58de9bf, pull request #927 | **mouths move** (the face network's indices) |
| Console threads from a group counter in groups of several host waves | VIctory-NVIDIA 854f57f (Vice-Tay c125c0c) | NVIDIA's 2-D group layout no longer runs some threads twice |
| Placed views ignore padding words | VIctory-NVIDIA PR #58 (project-flamingo) | confetti on faces, leaked views |
| Reflections that miss take the ambient estimate, not the sky (`VICTORY_AMBIENT_SPECULAR=1`) | VIctory-NVIDIA 2db59d48; the miss program's depth found at its slot in this fork's translation (1, 96) | **glowing blue-white eyes fixed** |
| A slot keeps showing its old texture until the new one has uploaded (`VICTORY_KEEP_SHOWN=2`) | upstream switch, raised | far less confetti on faces in the diner cutscene |
| Decal defrag and the water's tiled light left out (above) | VIctory-NVIDIA | no decal smears; no dark blocks on open water (the water is a little darker and bluer instead) |

### Stability

| Change | Source |
|---|---|
| The title's hang handler logs and returns instead of faulting at `+0x23cf683` (`VICTORY_TITLE_WATCHDOG=1` keeps it) | this fork |
| No arena storm when the low address range is full; upload room reserved at start | VIctory-NVIDIA PR #88, e69e39c5 |
| Mono convolutions in the audio DSP (phone calls, TVs, reverb; not yet verified here) | VIctory-NVIDIA PR #77 |
| Copy range checks and late-write learning for card steps | [SlaktarnVI](https://github.com/Slaktarn700/SlaktarnVI) |

Tried and left off by default because they measured neutral or worse here: Kelvaroq's mesh launch
(`VICTORY_MESH_LAUNCH=1`), renderer ports 6 and 7 (`VICTORY_PORT_GUARD`, `VICTORY_PORT_JOURNAL`),
`VICTORY_PLACED_ORDER`, `VICTORY_ASK_EARLY`, `VICTORY_PAGE_WRITES=all`, `VICTORY_CARD_RESERVE_BUDGETS`,
`VICTORY_WIDE_MERGE=1`, write watches asked in pieces (`VICTORY_WATCH_CHUNK_MB`, upstream PR #71),
more or fewer worker threads (`VICTORY_WIDE_WORKERS=8`, `VICTORY_THIN_THREADS=6`). The code stays, for
other machines. `VICTORY_ASK_NORESET=1` froze the game here.

---

## Quick start

Full setup (Wine prefix, vkd3d-proton, DXVK's dxgi, `d3dcompiler_47`, the patched wineserver) is in
**[linux/README.md](linux/README.md)**. 
**Prebuilt download (easiest):** grab `victory-linux-nvidia-<version>.tar.gz` from the
[releases](https://github.com/dfmbrollins/victory-linux-nvidia/releases), unpack it, and run
`./start.sh /path/to/your/dump`. It sets up Wine, the bundled vkd3d-proton/DXVK and `d3dcompiler_47`
by itself; `README-FIRST.txt` inside walks through it. You need your own dump. Linux only.

Building from source, in short:

```sh
# 1. Build (cross-compiled to Windows with cargo-xwin)
rustup toolchain install
cargo install --locked cargo-xwin
cargo xwin build --release --locked -p victory --target x86_64-pc-windows-msvc

# 2. Translate the shaders and build every pipeline ahead (once; ~30 min the first time)
DUMP=/path/to/your/dump linux/play.sh shaders
DUMP=/path/to/your/dump linux/play.sh warm --again

# 3. Play
DUMP=/path/to/your/dump linux/play.sh
```

Everything a run writes (prefix, caches, saves, logs) goes outside the repository: its parent folder by
default, or `VICTORY_LINUX_WORK`. Every setting in `linux/play.sh` can be overridden by exporting it
first. `CONSOLE=series-s` selects the Series S profile.

**Tips**
- Close the browser and other GPU-heavy programs: the game only gets the VRAM the desktop leaves it
  (~8 GB of 12 here), and VRAM residency was the biggest single speed lever.
- After a change to the translator, vkd3d-proton or the shader cache key, run `warm --again` in full and
  measure on the second run: the pipeline library is rebuilt by the first.
- `linux/tools/ab-run.sh <name> [VAR=value ...]` runs one scripted measurement (frame time, GPU load,
  two screenshots), for comparing one switch at a time.

---

## Credits

This fork stands on the work of many people. Imported changes keep their original authorship and
commit references in the history.

- **[cool_fox](https://github.com/DoubleGhosty/cool_fox)** (CGBJ runtime continuation), the base, and the
  upstream VIctory work it carries: avatarmanz, codec-d, DexNom, Belobug and contributors
  ([its README](docs/README-cool_fox.md)).
- **[Vice-Tay](https://github.com/TaynanGT/Vice-Tay)** (TaynanGT and contributors): the NVIDIA memory profiles.
- **[VIctory-NVIDIA](https://github.com/trustno1-coder/VIctory-NVIDIA)** (trustno1-coder): face, reflection,
  arena and audio fixes.
- **[VIctory fork](https://github.com/fxceless/victory-fork)** (fxceless) and **tippythetaidum** (#927): the mouth fix.
- **[Kelvaroq](https://github.com/stellarr-studio/kelvaroq)** (stellarr-studio): the mesh launch, measurements
  of the memory wall.
- **[SlaktarnVI](https://github.com/Slaktarn700/SlaktarnVI)** (Slaktarn700): card-step learning.
- **[Victory-United](https://github.com/Fayberr/Victory-United)** (Fayberr): the BVH-miss switch's port.
- The community testers and writers whose measurements and write-ups pointed at the drains, the memory
  profile and the watchdog.

## License

GPL-3.0-or-later, as upstream ([LICENSE](LICENSE)). This project contains no game code or assets and is
not affiliated with or endorsed by Rockstar Games, Take-Two Interactive or Microsoft.
