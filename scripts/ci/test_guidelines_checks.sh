#!/usr/bin/env bash
# Smoke tests for guideline conformance scripts (R026).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GUIDELINES_DIR="${ROOT_DIR}/scripts/ci/guidelines"

echo "guidelines smoke: run_guidelines_verify on clean tree"
bash "${ROOT_DIR}/scripts/ci/run_guidelines_verify.sh" >/dev/null

echo "guidelines smoke: individual checks"
for check in "${GUIDELINES_DIR}"/check_*.sh; do
  echo "  ${check}"
  bash "${check}" >/dev/null
done

echo "guidelines smoke tests passed."
