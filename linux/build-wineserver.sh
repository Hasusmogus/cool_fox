#!/usr/bin/env bash
# Builds Wine's server with linux/wine/0001-*.patch (process views in a tree: the
# stock server spends 35-40% of all CPU searching a list of the game's ~25,000
# memory views) for the Wine you have installed, into $WORK/wine-build. play.sh
# and start.sh use it from then on, while `wine --version` still matches.
#
# Needs git, a C compiler, make, flex and bison (Arch: base-devel git; Fedora:
# gcc make flex bison git; Ubuntu: build-essential flex bison git). About 10 min.
set -euo pipefail
REPO="$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)"
WORK="${VICTORY_LINUX_WORK:-${XDG_DATA_HOME:-$HOME/.local/share}/victory-linux-nvidia}"
for c in git make gcc flex bison; do
  command -v "$c" >/dev/null || { echo "error: $c is needed (see the top of this script)" >&2; exit 1; }
done
full="$(wine --version)"          # e.g. "wine-11.22 (Staging)"
ver="${full#wine-}"; ver="${ver%% *}"
staging=0; [[ "$full" == *Staging* || "$full" == *staging* ]] && staging=1
echo "Building the patched server for $full"

B="$WORK/wine-build"
rm -rf "$B.new"; mkdir -p "$B.new"; cd "$B.new"
git clone -q --depth 1 --branch "wine-$ver" https://gitlab.winehq.org/wine/wine.git
if (( staging )); then
  git clone -q --depth 1 --branch "v$ver" https://gitlab.winehq.org/wine/wine-staging.git
  (cd wine && ../wine-staging/staging/patchinstall.py --backend=git-apply --all >/dev/null)
fi
(cd wine && git apply "$REPO/linux/wine/0001-server-index-process-views-by-base-address.patch")
mkdir build64 && cd build64
# Only the server is built: everything optional is left out.
../wine/configure -q --enable-win64 --without-mingw --without-x --without-freetype \
  --without-gstreamer --without-vulkan --without-wayland --without-alsa --without-pulse \
  --without-cups --without-sane --without-gphoto --without-v4l2 --without-pcap \
  --without-netapi --without-krb5 --without-gssapi --without-capi --without-usb \
  --without-sdl --without-udev --without-unwind --without-dbus --without-opencl \
  --without-osmesa --without-fontconfig --without-gnutls --without-xinerama
make -s -j"$(nproc)" server/wineserver
echo "$full" > "$B.new/wine-version.txt"
rm -rf "$B"; mv "$B.new" "$B"
echo "Done: $B/build64/server/wineserver (for $full). Rebuild after updating Wine."
