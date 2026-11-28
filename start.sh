#!/usr/bin/env bash
# One command to play on Linux: see linux/start.sh (./start.sh /path/to/your/dump).
exec "$(dirname "$(readlink -f "$0")")/linux/start.sh" "$@"
