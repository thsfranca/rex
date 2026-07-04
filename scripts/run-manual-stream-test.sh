#!/usr/bin/env bash
set -euo pipefail

cat <<'EOF'
Manual test steps:

  rex

Expected behavior:
- Interactive TUI opens (daemon auto-starts when needed).
- Health and phase appear in the header.
- Submit a prompt in the composer; assistant text streams in the transcript.
- Esc cancels an in-flight turn; Ctrl+C twice exits.

Public `rex daemon`, `rex status`, and `rex complete` were removed.
EOF
