#!/usr/bin/env bash
# Deprecated entrypoint: use run_sidecar_verify.sh (same checks, CI-standard contract).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
exec "${ROOT}/scripts/ci/run_sidecar_verify.sh"
