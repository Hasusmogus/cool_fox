# cool_fox on Linux

The runtime is a Windows program; on Linux it runs under Wine, with
vkd3d-proton for Direct3D 12. This folder holds what that takes: the launcher,
a patch for Wine's server, and the tools used to find what was slow or broken.
Developed and measured on Arch Linux, KDE Plasma (Wayland), an NVIDIA RTX 5070
(driver 620.73), Wine-staging 11.22.

Nothing here contains game files. Everything a run writes (the Wine prefix,
caches, saves, logs, profiles) goes to a work folder outside the repository:
the repository's parent by default, or `VICTORY_LINUX_WORK`.

## Build

The runtime is cross-compiled to `x86_64-pc-windows-msvc` with
[cargo-xwin](https://github.com/rust-cross/cargo-xwin) (it downloads the MSVC
CRT and Windows SDK, accepting Microsoft's license):

```sh
rustup toolchain install          # the version rust-toolchain.toml pins, with the MSVC target
cargo install --locked cargo-xwin
cargo xwin build --release --locked -p victory --target x86_64-pc-windows-msvc
# or, with per-reason counts of dropped draws:
cargo xwin build --locked --profile diag -p victory --features diagnostics --target x86_64-pc-windows-msvc
```

`victory-shader` (the translator) also builds and tests natively:
`cargo test -p victory-shader --target x86_64-unknown-linux-gnu`.
`cargo run -p victory-shader --example translate_one -- <container>` translates
one shader container (from a run's `known-shaders.bin`, see
`tools/extract-shader.py`) without Wine.

## One-time setup of the Wine prefix

```sh
export WINEPREFIX=$WORK/.wineprefix
wineboot -i
# vkd3d-proton's d3d12/d3d12core and DXVK's dxgi, from a Proton build
P=~/.local/share/Steam/steamapps/common/"Proton - Experimental"/files/lib/wine
cp "$P"/vkd3d-proton/x86_64-windows/d3d12*.dll "$P"/dxvk/x86_64-windows/dxgi.dll "$WINEPREFIX"/drive_c/windows/system32/
# Microsoft's HLSL compiler: Wine's own lacks what the runtime's shaders use
winetricks -q d3dcompiler_47
```

Use a recent vkd3d-proton: the one in GE-Proton11-11 (October 2026) crashed the
NVIDIA 620.73 driver's compiler (illegal instruction in `vkCreateComputePipelines`)
on several of the title's compute shaders; Proton Experimental's of 14 November
did not.

## The patched wineserver (strongly recommended)

The title streams by mapping and unmapping 64 KiB physical pages thousands of
times a second, over some 25,000 views. Wine's server kept each process's views
in a list and searched it on every map and unmap: 35-40% of all CPU time went
there. `wine/0001-server-index-process-views-by-base-address.patch` keeps them in
a tree as well. Build the server from the same sources as the installed Wine (the
protocol must match), for Wine-staging 11.22:

```sh
cd $WORK && mkdir wine-build && cd wine-build
git clone --depth 1 --branch wine-11.22 https://gitlab.winehq.org/wine/wine.git
git clone --depth 1 --branch v11.22 https://gitlab.winehq.org/wine/wine-staging.git
cd wine && ../wine-staging/staging/patchinstall.py --backend=git-apply --all
git apply /path/to/cool_fox/linux/wine/0001-server-index-process-views-by-base-address.patch
mkdir ../build64 && cd ../build64
../wine/configure --enable-win64 --without-mingw --without-x   # (and --without-* for what you lack)
make -j$(nproc) server/wineserver
```

`play.sh` starts it before the title when `$WORK/wine-build/build64/server/wineserver`
exists (`PATCHED_SERVER=0` uses the system's). Installed Wine prefers its own
server binary over `WINESERVER`, but connects to a server already running for the
prefix.

## Running

```sh
DUMP=/path/to/your/dump linux/play.sh             # the title (AUTO_A=1 presses A for you)
DUMP=... linux/play.sh shaders 'Z:\path\to\dump\shaders.rpf'   # translate every shader ahead
DUMP=... linux/play.sh warm                       # build every pipeline earlier runs created
```

Run `warm` again after changing switches that change translation (`VICTORY_RAYS`
does): otherwise the driver compiles those pipelines while the title waits.

Defaults worth knowing (`play.sh` sets them; export one to override):
`VICTORY_RAYS=1 VICTORY_BVH_BUILDER=1` (the title's ray-traced light),
`VICTORY_HAIR=1` (hair and skinning passes), `VICTORY_OUTPUT=1920x1080`.

`/tmp` is often RAM (tmpfs): keep logs of long runs on disk.

## Tools

- `tools/perf-report.py <perf.data> <victory.exe> <run.log> [--comm title-main]`:
  a profile of a `PERF=1` run with the runtime's functions named (from its PDB, at
  the address Wine loaded it: run with `WINEDEBUG=+loaddll`) and the title's code
  by offset.
- `tools/sample-window.sh`: the process's use of [4 GiB, 2^37) and its RSS over time
  (title memory by address must fit there).
- `tools/sample-server.sh`: the wineserver's CPU share over time.
- `tools/wine-alloc-report.py`: big allocations by thread from a `WINEDEBUG=+virtual,+threadname` log.
- `tools/extract-shader.py`: shader containers out of a run's `known-shaders.bin` by hash.
- `tools/auto-skip.sh`: reruns the title, adding each pipeline a driver crashes on to the skip list.

## What was fixed for Linux

- The host pipeline library under vkd3d-proton (an empty library was asked for
  with a dangling pointer).
- Root signatures of the runtime's own shaders made from their `RS` strings
  (Wine's compiler embeds none), which left GPU copies of title memory off, and
  with them title memory by address and the ray-tracing structure builder.
- A 16 MiB reservation per draw leaked while that set-up kept failing, until the
  process ran out of address space below 2^37.
- Wave64 compute programs in groups of several console waves translate (pair
  barriers instead of group barriers).
