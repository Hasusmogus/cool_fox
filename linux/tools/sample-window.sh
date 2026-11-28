#!/usr/bin/env bash
# Every N seconds: the game process's use of [4 GiB, 2^37) by kind and protection,
# whether it holds /dev/ntsync, and its RSS. usage: sample-window.sh <out> [interval]
out="$1"; iv="${2:-10}"
while true; do
  P=$(pgrep -x main | head -1)
  if [ -n "$P" ]; then
    nts=$(ls -l /proc/$P/fd 2>/dev/null | grep -c ntsync)
    rss=$(awk '/VmRSS/ {print $2}' /proc/$P/status 2>/dev/null)
    awk -v lo=$((1<<32)) -v hi=$((1<<37)) -v t="$(date +%T)" -v nts="$nts" -v rss="$rss" '
      {split($1,a,"-"); s=strtonum("0x" a[1]); e=strtonum("0x" a[2]);
       if (e>lo && s<hi) { if (s<lo) s=lo; if (e>hi) e=hi; k=($6==""?"anon":"file") ":" $2; v[k]+=e-s; n[k]++; tot+=e-s } }
      END { printf "%s rss=%.1fG ntsync_fds=%s window=%.1fG", t, rss/2^20, nts, tot/2^30;
            for (k in v) printf " %s=%.1fG/%d", k, v[k]/2^30, n[k]; printf "\n" }' /proc/$P/maps >> "$out"
  fi
  sleep "$iv"
done
