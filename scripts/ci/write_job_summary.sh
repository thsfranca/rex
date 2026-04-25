#!/usr/bin/env bash
set -euo pipefail

job_name="${1:-}"

if [ -z "${job_name}" ]; then
  echo "::error::write_job_summary.sh requires a job name argument."
  exit 1
fi

echo "::group::PostRunSummary"
{
  echo "### ${job_name}"
  echo ""
  echo "- result: ${CI_RESULT:-unknown}"
  echo "- fail_stage: ${CI_FAIL_STAGE:--}"
  echo "- fail_code: ${CI_FAIL_CODE:--}"
  echo "- hint: ${CI_HINT:--}"
  echo "- run_id: ${GITHUB_RUN_ID:-unknown}"
} >> "$GITHUB_STEP_SUMMARY"
echo "::notice::Summary written for ${job_name}."
echo "::endgroup::"
