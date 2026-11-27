#!/usr/bin/env bash
# One command to play: sets Wine up the first time, translates the game's shaders
# once, builds the pipelines earlier runs found, then starts the game.
#
#   ./start.sh /path/to/your/dump     the first time (the folder holding game_scarlett_master.exe)
#   ./start.sh                        afterwards (the folder is remembered)
#
# Everything it writes (Wine prefix, caches, saves, logs) goes to
# ~/.local/share/victory-linux-nvidia, or VICTORY_LINUX_WORK, so it survives
# unpacking a newer release. SKIP_WARM=1 skips building pipelines ahead.
set -euo pipefail
REPO="$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)"
export VICTORY_LINUX_WORK="${VICTORY_LINUX_WORK:-${XDG_DATA_HOME:-$HOME/.local/share}/victory-linux-nvidia}"
WORK="$VICTORY_LINUX_WORK"
mkdir -p "$WORK"

say() { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$*" >&2; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

# --- What the system needs ---------------------------------------------------
command -v wine >/dev/null || die "Wine is not installed. Install it with your package manager
  (Arch: sudo pacman -S wine-staging winetricks   Fedora: sudo dnf install wine winetricks
   Ubuntu/Debian: see https://gitlab.winehq.org/wine/wine/-/wikis/Download for a recent Wine)."
WINE_VERSION="$(wine --version 2>/dev/null || true)"
v="${WINE_VERSION#wine-}"; major="${v%%.*}"
if [[ "$major" =~ ^[0-9]+$ ]] && (( major < 10 )); then
  warn "$WINE_VERSION is old; this build was tested with wine-staging 11.22. Expect problems."
fi
if command -v nvidia-smi >/dev/null; then
  gpu="$(nvidia-smi --query-gpu=name,memory.total --format=csv,noheader 2>/dev/null | head -1 || true)"
  [[ -n "$gpu" ]] && say "GPU: $gpu"
else
  warn "No NVIDIA driver found (nvidia-smi). This build is tuned and tested for NVIDIA RTX cards only."
fi

# --- The dump ------------------------------------------------------------------
SAVED="$WORK/dump-path.txt"
DUMP="${1:-${DUMP:-}}"
if [[ -z "$DUMP" && -f "$SAVED" ]]; then DUMP="$(cat "$SAVED")"; fi
if [[ -z "$DUMP" ]]; then
  if [[ -n "${DISPLAY:-}${WAYLAND_DISPLAY:-}" ]] && command -v kdialog >/dev/null; then
    DUMP="$(kdialog --title "Your game dump folder (with game_scarlett_master.exe)" --getexistingdirectory "$HOME" 2>/dev/null || true)"
  elif [[ -n "${DISPLAY:-}${WAYLAND_DISPLAY:-}" ]] && command -v zenity >/dev/null; then
    DUMP="$(zenity --file-selection --directory --title "Your game dump folder (with game_scarlett_master.exe)" 2>/dev/null || true)"
  fi
fi
if [[ -z "$DUMP" && -t 0 ]]; then
  read -rp "Path to your game dump folder (the one holding game_scarlett_master.exe): " DUMP
fi
[[ -n "$DUMP" ]] || die "Give the path to your dump: ./start.sh /path/to/your/dump"
DUMP="${DUMP/#\~/$HOME}"
asked="$DUMP"
DUMP="$(cd "$DUMP" 2>/dev/null && pwd)" || die "No such folder: $asked"
[[ -f "$DUMP/game_scarlett_master.exe" ]] || die "$DUMP has no game_scarlett_master.exe. Point it at the folder of your dump that holds it."
[[ -f "$DUMP/shaders.rpf" ]] || die "$DUMP has no shaders.rpf: the dump looks incomplete."
echo "$DUMP" > "$SAVED"
export DUMP

# --- The Wine prefix -------------------------------------------------------------
export WINEPREFIX="$WORK/.wineprefix"
export WINEDEBUG="${WINEDEBUG:--all}"
SYS="$WINEPREFIX/drive_c/windows/system32"
if [[ ! -f "$WINEPREFIX/system.reg" ]]; then
  say "Creating the Wine prefix (first launch only)..."
  WINEDLLOVERRIDES="mscoree=;mshtml=" wineboot -i >/dev/null 2>&1 || die "wineboot failed: is Wine working?"
  wineserver -w
fi
# vkd3d-proton (Direct3D 12) and DXVK's dxgi: the release carries the tested ones
# in dlls/; a source checkout takes Proton Experimental's when Steam has it.
DLLS="$REPO/dlls"
if [[ ! -f "$DLLS/d3d12core.dll" ]]; then
  P="$HOME/.local/share/Steam/steamapps/common/Proton - Experimental/files/lib/wine"
  if [[ -f "$P/vkd3d-proton/x86_64-windows/d3d12core.dll" && -f "$P/dxvk/x86_64-windows/dxgi.dll" ]]; then
    DLLS="$WORK/dlls-proton"; mkdir -p "$DLLS"
    cp -f "$P"/vkd3d-proton/x86_64-windows/d3d12{,core}.dll "$P"/dxvk/x86_64-windows/dxgi.dll "$DLLS"/
  else
    DLLS=""
  fi
fi
if [[ -n "$DLLS" ]]; then
  for d in d3d12 d3d12core dxgi; do
    cmp -s "$DLLS/$d.dll" "$SYS/$d.dll" || cp -f "$DLLS/$d.dll" "$SYS/$d.dll"
  done
elif [[ ! -f "$SYS/d3d12core.dll" ]]; then
  die "vkd3d-proton is missing: use a release download (it carries it in dlls/) or see linux/README.md."
fi
# Microsoft's HLSL compiler (Wine's own lacks what the runtime's shaders need).
# It cannot be redistributed: winetricks fetches it from Microsoft.
# A new prefix holds Wine's own placeholder of it ("Wine builtin DLL"), not Microsoft's.
if [[ ! -f "$SYS/d3dcompiler_47.dll" ]] || grep -q "Wine builtin DLL" "$SYS/d3dcompiler_47.dll"; then
  command -v winetricks >/dev/null || die "winetricks is needed once, to install d3dcompiler_47. Install it with your package manager and run ./start.sh again."
  say "Installing d3dcompiler_47 with winetricks (first launch only)..."
  winetricks -q d3dcompiler_47 >/dev/null 2>&1 || die "winetricks d3dcompiler_47 failed; run 'WINEPREFIX=$WINEPREFIX winetricks d3dcompiler_47' to see why."
fi

if [[ ! -x "$WORK/wine-build/build64/server/wineserver" && ! -f "$WORK/.server-tip" ]]; then
  say "Tip: linux/build-wineserver.sh builds a faster Wine server for this game (one time, ~10 min;"
  say "     the stock one spends 35-40% of the CPU on the game's memory mappings)."
  touch "$WORK/.server-tip"
fi

PLAY="$REPO/linux/play.sh"
# --- Shaders, once per runtime version --------------------------------------------------
EXE="$REPO/victory.exe"; [[ -f "$EXE" ]] || EXE="$REPO/target/x86_64-pc-windows-msvc/${BUILD:-release}/victory.exe"
[[ -f "$EXE" ]] || die "victory.exe not found: unpack the whole release, or build it (see README.md)."
stamp="$(sha256sum "$EXE" | cut -c1-16)"
if [[ "$(cat "$WORK/.shaders-done" 2>/dev/null)" != "$stamp" ]]; then
  say "Translating the game's shaders (once per version; this can take several minutes)..."
  log="$WORK/shaders.log"
  if "$PLAY" shaders "Z:${DUMP//\//\\}\\shaders.rpf" > "$log" 2>&1; then
    grep -vE 'MESA|pci id|dri2|^info:|^warn:|fixme' "$log" | tail -3
    echo "$stamp" > "$WORK/.shaders-done"
  else
    status=$?
    grep -E 'err:module|error|Error|panicked' "$log" | tail -5 >&2 || true
    die "translating the shaders failed (exit $status); the whole output is in $log"
  fi
fi

# --- Pipelines found by earlier runs -------------------------------------------------
if [[ "${SKIP_WARM:-0}" != 1 && -f "$WORK/victory-cache/pipelines/known-pipelines.bin" ]]; then
  say "Building pipelines found by earlier runs (only new ones; the first time after a run or an"
  say "update this can take up to ~30 min, later launches a few seconds; SKIP_WARM=1 skips it)..."
  "$PLAY" warm 2>&1 | grep -E 'pipelines known|made in' || true
elif [[ ! -f "$WORK/victory-cache/pipelines/known-pipelines.bin" ]]; then
  say "First run: pipelines are compiled while you play, so it stutters and drops frames. Quit after"
  say "the opening cutscene or so; the next ./start.sh builds them ahead and later runs are smooth."
fi

say "Starting the game ($DUMP)..."
exec "$PLAY"
