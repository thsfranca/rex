#!/usr/bin/env bash
set -euo pipefail

cat <<'EOF'
Manual test steps (use two terminals):

Terminal 1:
  rex-daemon

Terminal 2:
  rex-cli complete "hello from rex"

Expected behavior:
- Terminal 2 prints text in incremental chunks.
- Output ends with a newline once the done chunk arrives.
EOF
