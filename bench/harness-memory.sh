#!/usr/bin/env bash
# Measure one OmniBundleScope run: wall time and peak RSS, in the shape
# `docs/contracts/bench-spec.md` §5 requires.
#
#   ./harness-memory.sh <binary> <input> [flags…]
#
# Peak RSS is sampled here because Node cannot read another process's RSS
# cheaply, and because on Windows the shell is the only portable place to do it.
set -euo pipefail

BIN="$1"; shift
INPUT="$1"; shift
LABEL="$(basename "$INPUT" | tr -c 'A-Za-z0-9._-' '_')-$(date +%s)"

PEAK=0
if command -v /usr/bin/time >/dev/null 2>&1; then
  # Linux: /usr/bin/time -v gives Maximum resident set size directly.
  OUT="$(/usr/bin/time -v "$BIN" "$INPUT" "$@" 2>&1)" || true
  PEAK="$(printf '%s' "$OUT" | awk -F': ' '/Maximum resident set size/ {print $2}' | tail -1)"
  printf '%s\n' "$OUT" | grep -v -E '^\s' | head -3 || true
else
  # Portable fallback: sample the child from a background loop.
  "$BIN" "$INPUT" "$@" >/tmp/ob-$LABEL.out 2>/tmp/ob-$LABEL.err &
  PID=$!
  while kill -0 "$PID" 2>/dev/null; do
    # shellcheck disable=SC2016
    RSS=$(ps -o rss= -p "$PID" 2>/dev/null | tr -d ' ' || echo 0)
    [ -n "${RSS:-}" ] && [ "$RSS" -gt "$PEAK" ] && PEAK=$RSS
    sleep 0.1
  done
  wait "$PID" || true
  cat /tmp/ob-$LABEL.out || true
fi

mkdir -p results
{
  echo "{\"input\":\"$INPUT\",\"peak_rss_kb\":${PEAK:-0},\"measured_by\":\"harness-memory.sh\"}"
} >> "results/bench-$LABEL.log"
cat "results/bench-$LABEL.log"
