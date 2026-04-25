#!/usr/bin/env bash
# Test fixture: emits done then an extra terminal event.
set -u
printf '{"event":"chunk","index":0,"text":"hello"}\n'
printf '{"event":"done","index":1}\n'
printf '{"event":"error","message":"should be ignored","code":"unknown"}\n'
exit 0
#!/usr/bin/env bash
# Test fixture: emits done then an extra terminal event.
set -u
printf '{"event":"chunk","index":0,"text":"hello"}\n'
printf '{"event":"done","index":1}\n'
printf '{"event":"error","message":"should be ignored","code":"unknown"}\n'
exit 0
