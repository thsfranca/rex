#!/usr/bin/env bash
# Persist CI_* env vars, print failure excerpt, and exit non-zero on failure.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

result="${result:-success}"
fail_code="${fail_code:--}"
fail_stage="${fail_stage:--}"
hint="${hint:--}"

{
  echo "CI_RESULT=${result}"
  echo "CI_FAIL_CODE=${fail_code}"
  echo "CI_FAIL_STAGE=${fail_stage}"
  echo "CI_HINT=${hint}"
} >> "${GITHUB_ENV:-/dev/null}"

if [ "${result}" != "success" ]; then
  CI_RESULT="${result}" CI_FAIL_CODE="${fail_code}" CI_FAIL_STAGE="${fail_stage}" CI_HINT="${hint}" \
    "${SCRIPT_DIR}/annotate_ci_failure.sh"
  exit 1
fi
