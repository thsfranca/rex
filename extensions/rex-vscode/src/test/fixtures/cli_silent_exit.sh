#!/usr/bin/env bash
# Test fixture: exits cleanly without emitting a terminal event.
set -u
printf '{"event":"chunk","index":0,"text":"partial"}\n'
exit 0
