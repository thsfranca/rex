#!/usr/bin/env bash
set -euo pipefail

cat <<'EOF'
Manual test steps (macOS):

  cd apps/rex-web && npm run build && cd -
  rex

Expected behavior:
- Desktop app opens (daemon auto-starts when needed).
- Status appears in the header strip.
- Submit a prompt in the composer; assistant text streams in the transcript.
- Approval modals appear when agent policy requires confirmation.

See docs/OPERATOR_UX.md for session flags (--continue, --last, --debug).
EOF
