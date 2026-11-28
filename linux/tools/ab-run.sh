#!/usr/bin/env bash
# One measured run: launches the title with the given environment, waits for the
# opening cutscene, measures ~150 s, stops, and prints the mean frame time and GPU load.
# usage: ab-run.sh <name> [VAR=value ...]
set -u
REPO="$(cd "$(dirname "$0")/../.." && pwd)"; HERE="${VICTORY_LINUX_WORK:-$(dirname "$REPO")}"; L="$HERE/logs"; mkdir -p "$L/shots"; name="$1"; shift
pkill -x nvidia-smi 2>/dev/null; nohup "$REPO/linux/tools/mem-sample.sh" "$L/mem-$name.log" >/dev/null 2>&1 & mempid=$!
nohup nvidia-smi dmon -s pucm -d 2 -o T > "$L/gpu-$name.log" 2>&1 &
( cd "$HERE" && env AUTO_A=1 BUILD=release WINEDEBUG=err+all,fixme-all VICTORY_LOG=info "$@" "${LAUNCHER:-$REPO/linux/play.sh}" > "$L/$name.log" 2>&1 ) &
until sed 's/\x1b\[[0-9;]*m//g' "$L/$name.log" 2>/dev/null | grep -aE 'INFO victory_gpu::host: [0-9.]+ fps' | tail -1 | grep -qE ' [0-9]{3,} draws/frame'; do
  sleep 3; grep -qaE 'unhandled exception|title exits' "$L/$name.log" 2>/dev/null && { echo "$name: ended before the cutscene"; exit 1; }
done
start=$(date +%T)
# Two screenshots mid-measurement (KDE spectacle; SHOT_CROP=WxH+X+Y crops to the game window) (logs/shots/<name>-1.png, -2.png) for a look at the picture.
shot() { timeout 20 spectacle -b -n -f -o "$L/shots/$name-$1.png" >/dev/null 2>&1 && { [[ -z "${SHOT_CROP:-}" ]] || magick "$L/shots/$name-$1.png" -crop "$SHOT_CROP" +repage -resize 1600x "$L/shots/$name-$1-game.png" 2>/dev/null; }; }
# Varied moments (first 15-70 s in, second 35-75 s after it) so runs catch different scenes.
# SHOT_A / SHOT_B: fixed shot times (seconds in, seconds after the first) to catch the same moment every run.
a=${SHOT_A:-$((15 + RANDOM % 56))}; b=${SHOT_B:-$((35 + RANDOM % 41))}; echo "shots at +${a}s and +$((a + b))s" > "$L/shots/$name.when"
sleep $a; shot 1; sleep $b; shot 2; sleep $((150 - a - b)); stop=$(date +%T)
WINEPREFIX="$HERE/.wineprefix" "$HERE/wine-build/build64/server/wineserver" -k 2>/dev/null; sleep 2; pkill -x nvidia-smi; kill $mempid 2>/dev/null
sed 's/\x1b\[[0-9;]*m//g' "$L/$name.log" | grep -aE 'INFO victory_gpu::host: [0-9.]+ fps' | awk -v n="$name" '{match($0,/ ([0-9.]+) fps, ([0-9.]+) ms\/frame/,m); match($0,/([0-9]+) draws\/frame/,d); if (d[1]+0 > 500) {ms+=m[2]; fr++; dr+=d[1]; a[fr]=m[2]}} END {if (fr) {t=0; c=0; for (i=fr-9; i<=fr; i++) if (i>0) {t+=a[i]; c++}; printf "%s: STEADY (last %d lines) %.1f ms/frame (%.2f fps); all %d lines mean %.0f ms (incl. the heavy cutscene start), %.0f draws/frame\n", n, c, t/c, 1000/(t/c), fr, ms/fr, dr/fr}}'
awk -v a="$start" -v b="$stop" 'NR>2 && $2 ~ /^[0-9]+$/ && $1>=a && $1<=b {n++; sm+=$6; if ($6<=10) lo++; pw+=$3} END {if (n) printf "  GPU: avg busy %.0f%%, idle %d%% of the time, %.0f W\n", sm/n, 100*lo/n, pw/n}' "$L/gpu-$name.log"
awk -v a="$start" -v b="$stop" 'NR>2 && $2 ~ /^[0-9]+$/ && $1>=a && $1<=b {n++; v+=$14; if ($14>m) m=$14} END {if (n) printf "  VRAM: avg %.1f GB, peak %.1f GB\n", v/n/1024, m/1024}' "$L/gpu-$name.log"
awk -v a="$start" -v b="$stop" 'NR>1 && $1>=a && $1<=b {n++; g+=$2; h+=$3; u+=$4; if ($4>mu) mu=$4; if ($5>ms) ms=$5} END {if (n) printf "  RAM: game private %.1f GB, shared %.1f GB, system used avg %.1f GB peak %.1f GB, swap peak %.1f GB\n", g/n/1024, h/n/1024, u/n/1024, mu/1024, ms/1024}' "$L/mem-$name.log"
