#!/usr/bin/env bash
# Re-print failure context then fail the job (used after continue-on-error verify steps).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
"${ROOT}/scripts/ci/annotate_ci_failure.sh"
exit 1
