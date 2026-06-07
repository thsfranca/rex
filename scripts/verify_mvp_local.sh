#!/usr/bin/env bash
# Local MVP preflight: build, full Rust CI verify, full extension CI checks.
# Does not start rex-daemon. Run from repository root.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

echo "==> cargo build --workspace"
cargo build --workspace

echo "==> Sidecar stub package builds"
cargo build -p rex-sidecar-stub --quiet

echo "==> Rust verify (fmt, clippy, tests) — see scripts/ci/run_rust_verify.sh"
"${ROOT_DIR}/scripts/ci/run_rust_verify.sh"

echo "==> Builtin sidecar verify — see scripts/ci/run_sidecar_verify.sh"
"${ROOT_DIR}/scripts/ci/run_sidecar_verify.sh"

echo "==> MVP product-path smoke (sidecar + brokered HTTP fixture + fs.read; no live LLM)"
cargo test -p rex-daemon mvp_product_path -- --nocapture

echo "==> Extension checks — see scripts/ci/run_extension_checks.sh"
"${ROOT_DIR}/scripts/ci/run_extension_checks.sh"

cat <<'EOF'

==> MVP local preflight passed.

Next (live LLM, after Ollama is up): ./scripts/verify_native_tools_live.sh

Then: configure JSON per docs/EXTENSION_LOCAL_E2E.md section 3, run rex daemon,
and follow the E2E doc for editor dogfood (status bar, REX: Open Chat, agent mode,
cancel, brokered fs.read).
EOF
