#!/usr/bin/env bash
# Test fixture: emulates `rex-cli status` reporting daemon unavailable.
set -u
printf 'daemon unavailable\n' >&2
exit 1
