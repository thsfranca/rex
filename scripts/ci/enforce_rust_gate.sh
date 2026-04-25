#!/usr/bin/env bash
set -euo pipefail

fmt_result="${FMT_RESULT:-missing}"
test_result="${TEST_RESULT:-missing}"
result="success"
fail_stage="-"
fail_code="-"
hint="-"

mkdir -p ci-observability

echo "::group::Setup"
echo "::notice::Evaluating upstream required checks."
echo "::endgroup::"

echo "::group::BuildAndChecks"
echo "::notice::No build/check execution in gate job."
echo "::endgroup::"

echo "::group::TestExecution"
echo "::notice::No test execution in gate job."
echo "::endgroup::"

echo "::group::PostRunSummary"
if [ "${fmt_result}" != "success" ] || [ "${test_result}" != "success" ]; then
  result="failure"
  fail_stage="PostRunSummary"
  fail_code="GATE_FAIL"
  hint="Inspect upstream job summaries and artifacts."
fi

{
  echo "### rust-checks"
  echo ""
  echo "- result: ${result}"
  echo "- fail_stage: ${fail_stage}"
  echo "- fail_code: ${fail_code}"
  echo "- hint: ${hint}"
  echo "- run_id: ${GITHUB_RUN_ID:-unknown}"
  echo ""
  echo "- rust-fmt-clippy: ${fmt_result}"
  echo "- rust-test: ${test_result}"
} >> "$GITHUB_STEP_SUMMARY"

{
  echo "result=${result}"
  echo "fail_stage=${fail_stage}"
  echo "fail_code=${fail_code}"
  echo "hint=${hint}"
  echo "rust_fmt_clippy=${fmt_result}"
  echo "rust_test=${test_result}"
} > "ci-observability/gate-summary.txt"

if [ "${result}" != "success" ]; then
  echo "::error::At least one required check failed."
  echo "CI_SIGNAL code=${fail_code} stage=${fail_stage} result=${result} hint=${hint}"
  exit 1
fi
echo "::notice::All required checks passed."
echo "::endgroup::"
