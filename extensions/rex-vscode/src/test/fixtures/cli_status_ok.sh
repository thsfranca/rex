#!/usr/bin/env bash
# Test fixture: emulates `rex-cli status` returning a valid snapshot.
set -u
if [[ "${1:-}" == "status" ]]; then
  printf 'daemon_version: 0.1.0-test\n'
  printf 'uptime_seconds: 1\n'
  printf 'active_model_id: test-model\n'
  exit 0
fi
printf 'unsupported subcommand: %s\n' "${1:-}" >&2
exit 2
