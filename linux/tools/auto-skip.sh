#!/usr/bin/env bash
# Runs play-linux.sh; when the NVIDIA driver crashes compiling a pipeline,
# records the shader being compiled in nvidia-skip.txt and relaunches.
HERE="$(cd "$(dirname "$0")" && pwd)"
WORK="${VICTORY_LINUX_WORK:-$(cd "$(dirname "$0")/../../.." && pwd)}"; LOG="${LOG:-$WORK/game-run.log}"
for i in $(seq 1 "${MAX_TRIES:-15}"); do
  echo "=== attempt $i, skipping: $(paste -sd, "$WORK/skip-pipelines.txt")"
  WINEDEBUG=err+all,fixme-all VICTORY_LOG=info "$(dirname "$0")/../play.sh" > "$LOG" 2>&1
  code=$?
  clean=$(sed 's/\x1b\[[0-9;]*m//g' "$LOG")
  frames=$(grep -oE 'frame presented.*frame=[0-9]+' <<<"$clean" | tail -1 | grep -oE '[0-9]+$')
  if grep -q 'vkCreate[A-Za-z]*Pipelines Exception' <<<"$clean"; then
    hash=$(grep -B200 -m1 'vkCreate[A-Za-z]*Pipelines Exception' <<<"$clean" | grep -oE 'host pipeline shaders=\[[^]]*\]|shader translated stage="[a-z]+" hash=[0-9a-f]{16}' | tail -1 | grep -oE '[0-9a-f]{16}' | tail -1)
    echo "=== driver crash at frame ${frames:-?} on shader $hash (exit $code)"
    [[ -n "$hash" ]] && ! grep -q "$hash" "$WORK/skip-pipelines.txt" && echo "$hash" >> "$WORK/skip-pipelines.txt" || { echo "=== could not add a new hash; stopping"; exit 1; }
  else
    echo "=== exited with code $code at frame ${frames:-?}, no driver crash found; stopping"
    exit "$code"
  fi
done
echo "=== gave up after ${MAX_TRIES:-15} tries"
