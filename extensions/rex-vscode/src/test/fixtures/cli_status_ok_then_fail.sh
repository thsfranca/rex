#!/usr/bin/env bash
# First status call succeeds; later calls fail (exercise ready -> unavailable probe).
set -euo pipefail
state_file="${REX_TEST_STATUS_PHASE_FILE:-}"
if [ -z "$state_file" ]; then
  printf 'daemon_version: 0.1.0\nuptime_seconds: 1\nactive_model_id: flip\n'
  exit 0
fi
phase="$(cat "$state_file" 2>/dev/null || echo 0)"
if [ "$phase" = "0" ]; then
  echo 1 >"$state_file"
  printf 'daemon_version: 0.1.0\nuptime_seconds: 1\nactive_model_id: flip\n'
  exit 0
fi
echo "daemon unavailable" >&2
exit 1
