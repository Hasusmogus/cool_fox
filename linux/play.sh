#!/usr/bin/env bash
# Runs victory.exe under Wine on Linux: the Linux counterpart of Play-VIctory.cmd.
# See linux/README.md for the one-time setup (prefix, vkd3d-proton, DXVK's dxgi,
# d3dcompiler_47, the patched wineserver).
#
#   DUMP=/path/to/your/dump linux/play.sh            run the title
#   DUMP=... linux/play.sh warm                      build every known pipeline ahead
#   DUMP=... linux/play.sh shaders <shaders.rpf>     translate every shader ahead
#
# Everything the run writes (prefix, caches, saves, logs, profiles) goes to WORK,
# outside the repository (docs/asset-policy.md): by default the repository's parent.
set -euo pipefail
REPO="$(cd "$(dirname "$0")/.." && pwd)"
WORK="${VICTORY_LINUX_WORK:-$(dirname "$REPO")}"
: "${DUMP:?set DUMP to your own dump folder (the one holding game_scarlett_master.exe)}"

export WINEPREFIX="${WINEPREFIX:-$WORK/.wineprefix}"
export WINEDEBUG="${WINEDEBUG:--all}"
# vkd3d-proton (D3D12), DXVK's dxgi, and Microsoft's d3dcompiler_47 (Wine's own
# D3DCompile, vkd3d-shader's, lacks RWByteAddressBuffer Load*/Interlocked* and
# SM 5.1 resource arrays, which twelve of the runtime's own shaders use).
export WINEDLLOVERRIDES="d3d12,d3d12core,dxgi,d3dcompiler_47=n"
# DXVK reports NVIDIA cards as AMD by default; the runtime then takes AMD-only
# paths (2^20 indirect dispatch groups) that NVIDIA cannot run.
export DXVK_CONFIG="${DXVK_CONFIG:-dxgi.hideNvidiaGpu = False}"
# Pipelines whose creation crashes a driver's compiler can be left out by shader
# hash, one per line in $WORK/skip-pipelines.txt (linux/tools/auto-skip.sh fills it).
if [[ -z "${VICTORY_SKIP_PIPELINES:-}" && -f "$WORK/skip-pipelines.txt" ]]; then
  VICTORY_SKIP_PIPELINES="$(grep -oE '^[0-9a-f]{16}' "$WORK/skip-pipelines.txt" | paste -sd,)"
  export VICTORY_SKIP_PIPELINES
fi
# NVIDIA's on-disk shader cache: large and never trimmed, so pipelines built ahead
# by `play.sh warm` are still there when the title asks for them.
export __GL_SHADER_DISK_CACHE=1
export __GL_SHADER_DISK_CACHE_SIZE="${__GL_SHADER_DISK_CACHE_SIZE:-17179869184}"
export __GL_SHADER_DISK_CACHE_SKIP_CLEANUP=1

winpath() { echo "Z:${1//\//\\}"; }
export VICTORY_GAME_DIR="$(winpath "$DUMP")"
export VICTORY_CACHE_DIR="$(winpath "$WORK/victory-cache")"
export VICTORY_DATA_DIR="$(winpath "$WORK/victory-data")"

# Defaults from Play-VIctory.cmd (its NVIDIA-safe set); export one to override it.
: "${VICTORY_AMBIENT_AIR:=1.5}" "${VICTORY_TEXTURE_NO_WAIT:=1}" "${VICTORY_RELAY:=2}"
: "${VICTORY_WIDE_BACK:=1}" "${VICTORY_CARD_REHASH_RUNS:=1}" "${VICTORY_JOIN_COPIES:=1}"
: "${VICTORY_INDIRECT_WINDOW:=1}" "${VICTORY_RELAY_COPY:=1}" "${VICTORY_CARD_STEPS:=2}"
: "${VICTORY_INDIRECT_BATCH:=2}" "${VICTORY_MAP_THREADS:=1}" "${VICTORY_CARD_AWAKE:=1}"
: "${VICTORY_SHOW_AFTER:=1}" "${VICTORY_AMBIENT:=1}" "${VICTORY_TEXTURE_GUARD:=0}"
: "${VICTORY_JOIN_PARTS:=0}" "${VICTORY_PORT_CONSTANTS:=1}"
: "${VICTORY_PORT_CBV:=1}" "${VICTORY_UPLOAD_AHEAD:=1}" "${VICTORY_LEVELS_ON_DEMAND:=0}"
: "${VICTORY_CARD_BUFFERS_MIN_KB:=64}" "${VICTORY_LOG:=warn}"
# Tiled textures with levels on demand read zeros where a level has no memory under
# vkd3d-proton on NVIDIA: rooms, walls, clothes and heads drew black. Off: they show,
# at no frame cost.
# Small fills the title itself wrote go on without waiting for every flight in the air
# (the drain was ~65 ms a frame). Kelvaroq's mesh launch (patch 0041) stays off here.
: "${VICTORY_FILL_TITLE:=1}" "${VICTORY_MESH_LAUNCH:=0}"
# A slot rewritten with another texture of the same dimension keeps showing the old one until
# the new one has uploaded (default 1: only the same kind). Without it faces in the diner
# cutscene showed coloured dots ("confetti") while their textures were mid-upload (runs 94/95).
: "${VICTORY_KEEP_SHOWN:=2}"
# VICTORY_TEXTURE_GUARD=1 (guard textures instead of hashing them whole every check) saves ~10 ms
# of preparing a frame (run 106) but brought back confetti on the diner waitress in some runs
# (107, 108; 106 and 109 clean, and 109's diag found no write the guard missed): off until the race
# is found.
# Vice-Tay's memory profile for 10-14 GB NVIDIA cards: more title buffers and windows in
# the card's memory (110-120 -> 58 ms a frame steady; copy-back 38 -> 15 ms).
: "${VICTORY_CARD_BUFFERS_MB:=1536}" "${VICTORY_CARD_WINDOWS_MB:=1024}"
# Six helper threads for refreshes and copies (the CPU rule gives 3 on 12 threads).
: "${VICTORY_WIDE_WORKERS:=6}"
# The pipelines each run creates are recorded, for `play.sh warm`.
: "${VICTORY_PIPELINE_BOOK:=1}"
# 1080p output instead of the native 4K: leaves VRAM for textures on 12 GB cards.
: "${VICTORY_OUTPUT:=1920x1080}"
# The title's ray-traced light (structure builder and the programs that walk it).
# Without it shadowed areas are black and the stand-in ambient washes out the rest.
: "${VICTORY_RAYS:=1}"
# The structure builder off and the ambient light estimated from the frame (VICTORY_AMBIENT=1):
# interiors lit far better than the title's own traced light with the builder on, and faster.
# Ray walks end at the root without structures (VICTORY_BVH_MISS, part of the translation:
# warm with it), and reflections that miss take the estimate, not the sky's light (blue eyes).
: "${VICTORY_BVH_BUILDER:=0}" "${VICTORY_BVH_MISS:=1}" "${VICTORY_AMBIENT_SPECULAR:=1}"
# Hair and skinning passes: upstream skips them by name, and its list holds passes
# characters need (CS_PreSkinSim and others). Stray strand lines remain.
# List calls only written down on the title's threads and translated by ours
# (45,000-60,000 a frame): 541 -> 305 ms a frame in the opening (runs 18, 19).
# Upstream turned it off on NVIDIA for faces drawn without skin in play; the
# opening's faces are whole here. VICTORY_THIN=0 translates inside the calls.
: "${VICTORY_THIN:=1}"
: "${VICTORY_HAIR:=1}"
# (the runtime only asks whether VICTORY_HAIR is set: 0 means unset here)
[[ "$VICTORY_HAIR" == 0 ]] && unset VICTORY_HAIR
export ${!VICTORY_@}

# AUTO_A=1: Enter (A on the stand-in pad) every 10 s from 20 s to 150 s, for runs
# nobody is at: the start screen waits for A.
if [[ "${AUTO_A:-0}" == 1 && -z "${VICTORY_KEYS:-}" ]]; then
  export VICTORY_KEYS="$(seq -s, -f "%g:Enter" 20 10 150)"
fi
# BUILD=release (default) or diag (cargo xwin build --profile diag --features diagnostics).
EXE="$REPO/target/x86_64-pc-windows-msvc/${BUILD:-release}/victory.exe"
# A release download keeps victory.exe next to linux/ instead of under target/.
if [[ ! -f "$EXE" && -f "$REPO/victory.exe" ]]; then EXE="$REPO/victory.exe"; fi
EXE="${VICTORY_EXE:-$EXE}"
mkdir -p "$WORK"; cd "$WORK"   # vkd3d-proton keeps vkd3d-proton.cache in the working folder

# A wineserver whose per-process views are in a tree rather than a list
# (linux/wine/0001-*.patch, built into $WORK/wine-build): the title maps and unmaps
# physical pages thousands of times a second over ~25,000 views, and the stock
# server searched the whole list for each (40% of all CPU). Started first, it
# serves the prefix: Wine connects to a running server rather than starting its own.
# PATCHED_SERVER=0: the system's.
SERVER="${PATCHED_SERVER_PATH:-$WORK/wine-build/build64/server/wineserver}"
if [[ -x "$SERVER" && -f "$WORK/wine-build/wine-version.txt" \
      && "$(cat "$WORK/wine-build/wine-version.txt")" != "$(wine --version)" ]]; then
  echo "warning: the patched wineserver was built for $(cat "$WORK/wine-build/wine-version.txt"), not $(wine --version): using the stock one (run linux/build-wineserver.sh again)" >&2
  PATCHED_SERVER=0
fi
if [[ "${PATCHED_SERVER:-1}" != 0 && -x "$SERVER" ]]; then
  wineserver -k 2>/dev/null || true
  "$SERVER" -k 2>/dev/null || true
  sleep 1
  "$SERVER" -p5
fi

if [[ "${1:-}" == warm || "${1:-}" == shaders ]]; then
  exec wine "$EXE" "$@"
fi
TITLE="$VICTORY_GAME_DIR\\game_scarlett_master.exe"
# PERF=1: under `perf record` (user space with call chains) into $WORK/perf-<time>.data;
# with kernel.yama.ptrace_scope=1 perf profiles only what it starts itself.
if [[ "${PERF:-0}" == 1 ]]; then
  exec perf record -e "${PERF_EVENT:-task-clock:u}" -F "${PERF_FREQ:-499}" -g --user-callchains \
    -o "$WORK/perf-$(date +%H%M%S).data" -- wine "$EXE" run "$TITLE" --console "${CONSOLE:-series-x}" --execute "$@"
fi
exec wine "$EXE" run "$TITLE" --console "${CONSOLE:-series-x}" --execute "$@"
