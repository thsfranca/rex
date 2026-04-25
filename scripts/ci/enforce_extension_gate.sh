#!/usr/bin/env bash
set -euo pipefail

extension_relevant="${EXTENSION_RELEVANT:-true}"
verify_result="${EXTENSION_VERIFY_RESULT:-missing}"
result="success"
fail_stage="-"
fail_code="-"
hint="-"

mkdir -p ci-observability

echo "::group::Setup"
echo "::notice::Evaluating upstream extension checks."
echo "::endgroup::"

echo "::group::BuildAndChecks"
echo "::notice::No build/check execution in gate job."
echo "::endgroup::"

echo "::group::TestExecution"
echo "::notice::No test execution in gate job."
echo "::endgroup::"

echo "::group::PostRunSummary"
if [ "${extension_relevant}" != "true" ]; then
  result="skip"
  hint="Extension checks were not relevant for this change set."
elif [ "${verify_result}" != "success" ]; then
  result="failure"
  fail_stage="PostRunSummary"
  fail_code="GATE_FAIL"
  hint="Inspect upstream extension job summaries and artifacts."
fi

{
  echo "### extension-checks"
  echo ""
  echo "- result: ${result}"
  echo "- fail_stage: ${fail_stage}"
  echo "- fail_code: ${fail_code}"
  echo "- hint: ${hint}"
  echo "- run_id: ${GITHUB_RUN_ID:-unknown}"
  echo ""
  echo "- extension-verify: ${verify_result}"
} >> "$GITHUB_STEP_SUMMARY"

{
  echo "result=${result}"
  echo "fail_stage=${fail_stage}"
  echo "fail_code=${fail_code}"
  echo "hint=${hint}"
  echo "extension_verify=${verify_result}"
} > "ci-observability/extension-gate-summary.txt"

if [ "${result}" != "success" ]; then
  if [ "${result}" = "skip" ]; then
    echo "::notice::Extension checks skipped as non-relevant."
    echo "::endgroup::"
    exit 0
  fi
  echo "::error::At least one required extension check failed."
  echo "CI_SIGNAL code=${fail_code} stage=${fail_stage} result=${result} hint=${hint}"
  exit 1
fi
echo "::notice::All required extension checks passed."
echo "::endgroup::"
