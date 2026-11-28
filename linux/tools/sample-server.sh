#!/usr/bin/env bash
# Every N seconds: the wineserver's CPU share since the last sample (utime+stime), and
# which binary it is. usage: sample-server.sh <out> [interval]
out="$1"; iv="${2:-10}"; hz=$(getconf CLK_TCK); last=0
while true; do
  P=$(pgrep -x wineserver | head -1)
  if [ -n "$P" ]; then
    t=$(awk '{print $14+$15}' /proc/$P/stat 2>/dev/null)
    if [ -n "$t" ] && [ "$last" != 0 ]; then
      echo "$(date +%T) server_cpu=$(awk -v d=$((t-last)) -v hz=$hz -v iv=$iv 'BEGIN{printf "%.0f%%", 100*d/hz/iv}') exe=$(readlink /proc/$P/exe | sed 's|.*/wine-build/.*|patched|;s|.*/usr/bin/.*|system|')" >> "$out"
    fi
    last=$t
  fi
  sleep "$iv"
done
