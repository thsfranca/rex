#!/usr/bin/env bash
# Test fixture: first two `rex-cli status` calls fail, then succeeds (needs REX_TEST_STATUS_STATE_FILE).
set -u
if [[ "${1:-}" != "status" ]]; then
  printf 'unsupported subcommand: %s\n' "${1:-}" >&2
  exit 2
fi
f="${REX_TEST_STATUS_STATE_FILE:?}"
c=$(cat "$f" 2>/dev/null || echo 0)
c=$((c + 1))
printf '%s' "$c" > "$f"
if [ "$c" -lt 3 ]; then
  echo "not ready" >&2
  exit 1
fi
printf 'daemon_version: 1.0.0-flaky\n'
printf 'uptime_seconds: 0\n'
printf 'active_model_id: mock\n'
exit 0
