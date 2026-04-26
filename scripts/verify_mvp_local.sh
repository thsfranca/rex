#!/usr/bin/env bash
# Local MVP preflight: build, full Rust CI verify, full extension CI checks.
# Does not start rex-daemon. Run from repository root.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

echo "==> cargo build --workspace"
cargo build --workspace

echo "==> Rust verify (fmt, clippy, tests) — see scripts/ci/run_rust_verify.sh"
"${ROOT_DIR}/scripts/ci/run_rust_verify.sh"

echo "==> Extension checks — see scripts/ci/run_extension_checks.sh"
"${ROOT_DIR}/scripts/ci/run_extension_checks.sh"

cat <<'EOF'

==> MVP local preflight passed.

Next: start rex-daemon (or enable rex.daemonAutoStart), then follow docs/EXTENSION_LOCAL_E2E.md
for editor steps: status bar, REX: Open Chat, and a short mock prompt.
EOF
