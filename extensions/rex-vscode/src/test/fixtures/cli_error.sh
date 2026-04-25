#!/usr/bin/env bash
# Test fixture: emits a single NDJSON error event.
set -u
printf '{"event":"error","message":"daemon unavailable"}\n'
exit 1
