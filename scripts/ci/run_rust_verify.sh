#!/usr/bin/env bash
set -euo pipefail

# CI/tests: direct in-process inference harness (no sidecar spawn required).
export REX_SIDECAR_HARNESS="${REX_SIDECAR_HARNESS:-direct}"
export REX_SIDECAR_ENABLED="${REX_SIDECAR_ENABLED:-0}"

# Sequential Rust CI: fmt + clippy, then workspace tests. Run from repository root.
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

"${ROOT_DIR}/scripts/ci/run_rust_fmt_clippy.sh"
"${ROOT_DIR}/scripts/ci/run_rust_tests.sh"
"${ROOT_DIR}/scripts/ci/run_sidecar_verify.sh"
