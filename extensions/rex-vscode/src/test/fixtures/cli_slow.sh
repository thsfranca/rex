#!/usr/bin/env bash
# Test fixture: streams slowly to exercise cancellation via AbortSignal.
set -u
printf '{"event":"chunk","index":0,"text":"slow"}\n'
sleep 5
printf '{"event":"done","index":1}\n'
