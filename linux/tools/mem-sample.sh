#!/usr/bin/env bash
# Every 2 s (cheap counters only; smaps_rollup would stall the game ~0.3 s):
#   game_anon_mb  the game's private memory (RssAnon of victory.exe)
#   shmem_mb      shared memory system-wide (mostly the title's memory; RSS counts it once per view, ~2.5x)
#   sys_used_mb   MemTotal - MemAvailable          swap_mb  swap in use
# usage: mem-sample.sh <out.log>   (stops when killed)
out="$1"; echo "#time game_anon_mb shmem_mb sys_used_mb swap_mb (RAM $(awk '/^MemTotal/{print int($2/1024)}' /proc/meminfo) MB)" > "$out"
while :; do
  an=0; for p in $(pgrep -f 'victory\.exe' 2>/dev/null); do r=$(awk '/^RssAnon/{print $2}' /proc/$p/status 2>/dev/null); an=$((an + ${r:-0})); done
  echo "$(date +%T) $((an/1024)) $(awk '/^MemTotal/{t=$2}/^MemAvailable/{a=$2}/^Shmem:/{h=$2}/^SwapTotal/{s=$2}/^SwapFree/{f=$2}END{printf "%d %d %d", h/1024, (t-a)/1024, (s-f)/1024}' /proc/meminfo)" >> "$out"; sleep 2
done
