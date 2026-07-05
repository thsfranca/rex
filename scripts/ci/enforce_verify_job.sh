#!/usr/bin/env bash
# Re-print failure context then fail the job (used after continue-on-error verify steps).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

if [[ "${CI_RESULT:-}" == "success" || -z "${CI_RESULT:-}" ]]; then
  export CI_RESULT="failure"
  export CI_FAIL_CODE="${CI_FAIL_CODE:--}"
  export CI_FAIL_STAGE="${CI_FAIL_STAGE:--}"
  export CI_HINT="${CI_HINT:-Verify step failed; see uploaded ci-observability logs.}"
  {
    echo "CI_RESULT=${CI_RESULT}"
    echo "CI_FAIL_CODE=${CI_FAIL_CODE}"
    echo "CI_FAIL_STAGE=${CI_FAIL_STAGE}"
    echo "CI_HINT=${CI_HINT}"
  } >> "${GITHUB_ENV:-/dev/null}"
fi

"${ROOT}/scripts/ci/annotate_ci_failure.sh"
exit 1
