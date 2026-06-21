#!/usr/bin/env bash
# First status call fails; later calls succeed (exercise unavailable -> ready probe).
set -euo pipefail
state_file="${REX_TEST_STATUS_PHASE_FILE:-}"
if [ -z "$state_file" ]; then
  echo "daemon unavailable" >&2
  exit 1
fi
phase="$(cat "$state_file" 2>/dev/null || echo 0)"
if [ "$phase" = "0" ]; then
  echo 1 >"$state_file"
  echo "daemon unavailable" >&2
  exit 1
fi
printf 'daemon_version: 0.1.0-recover\nuptime_seconds: 2\nactive_model_id: recovered\n'
printf 'lifecycle_state: idle\nidle_seconds: 0\nseconds_until_shutdown: 300\n'
exit 0
