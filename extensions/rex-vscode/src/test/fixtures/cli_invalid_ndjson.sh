#!/usr/bin/env bash
# Test fixture: emits malformed NDJSON payload.
set -u
printf '{"event":"chunk","index":0,"text":"hi"}\n'
printf '{not-valid-json}\n'
exit 0
#!/usr/bin/env bash
# Test fixture: emits malformed NDJSON payload.
set -u
printf '{"event":"chunk","index":0,"text":"hi"}\n'
printf '{not-valid-json}\n'
exit 0
