#!/usr/bin/env bash
# Packs a release: victory.exe, the tested vkd3d-proton/DXVK DLLs, start.sh and linux/.
#
#   linux/tools/make-release.sh <version> <victory.exe> <dll folder> <out folder>
#
# <dll folder> holds d3d12.dll, d3d12core.dll (vkd3d-proton), dxgi.dll (DXVK),
# LICENSE-vkd3d-proton, LICENSE-dxvk and SOURCES.txt (where each came from). Build
# victory.exe with paths remapped (README: "Releases") so no home folder is inside.
set -euo pipefail
[[ $# == 4 ]] || { sed -n 2,9p "$0"; exit 2; }
VER="$1"; EXE="$2"; DLLS="$3"; OUT="$4"
REPO="$(cd "$(dirname "$0")/../.." && pwd)"
N="victory-linux-nvidia-$VER"
for f in d3d12.dll d3d12core.dll dxgi.dll LICENSE-vkd3d-proton LICENSE-dxvk SOURCES.txt; do
  [[ -f "$DLLS/$f" ]] || { echo "missing $DLLS/$f" >&2; exit 1; }
done
if strings "$EXE" | grep -q "/home/"; then echo "victory.exe holds a /home/ path: build it with paths remapped" >&2; exit 1; fi
rm -rf "${OUT:?}/$N"; mkdir -p "$OUT/$N/dlls"
git -C "$REPO" archive HEAD start.sh linux README.md LICENSE victory.example.toml | tar -x -C "$OUT/$N"
cp "$EXE" "$OUT/$N/victory.exe"
cp "$DLLS"/{d3d12.dll,d3d12core.dll,dxgi.dll,LICENSE-vkd3d-proton,LICENSE-dxvk,SOURCES.txt} "$OUT/$N/dlls/"
cp "$REPO/linux/RELEASE-README.txt" "$OUT/$N/README-FIRST.txt"
chmod +x "$OUT/$N/start.sh" "$OUT/$N"/linux/*.sh "$OUT/$N"/linux/tools/*.sh
(cd "$OUT" && tar -czf "$N.tar.gz" "$N" && sha256sum "$N.tar.gz" > "$N.tar.gz.sha256")
echo "$OUT/$N.tar.gz"
