#!/usr/bin/env bash
# Test fixture: emits a minimal chunk + done NDJSON stream.
set -u
printf '{"event":"chunk","index":0,"text":"hello "}\n'
printf '{"event":"chunk","index":1,"text":"world"}\n'
printf '{"event":"done","index":2}\n'
exit 0
