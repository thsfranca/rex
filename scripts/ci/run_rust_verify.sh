#!/usr/bin/env bash
set -euo pipefail

# Sequential Rust CI: fmt + clippy, then workspace tests. Run from repository root.
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

"${ROOT_DIR}/scripts/ci/run_rust_fmt_clippy.sh"
"${ROOT_DIR}/scripts/ci/run_rust_tests.sh"
