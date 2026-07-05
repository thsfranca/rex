#!/usr/bin/env bash
# Local MVP preflight: build, full Rust CI verify.
# Does not start the desktop app. Run from repository root.
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

cat <<'EOF'

==> MVP local preflight passed.

Next: configure JSON per docs/OPERATOR_UX.md, build apps/rex-web (npm ci && npm run build),
then run rex on macOS for desktop operator dogfood.
EOF
