#!/usr/bin/env bash
# Test fixture: emulates a daemon that crashes immediately on start.
set -u
printf 'fake daemon crashed\n' >&2
exit 1
