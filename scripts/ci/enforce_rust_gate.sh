#!/usr/bin/env bash
set -euo pipefail

fmt_result="${FMT_RESULT:-missing}"
test_result="${TEST_RESULT:-missing}"

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
{
  echo "### rust-checks"
  echo ""
  echo "- rust-fmt-clippy: ${fmt_result}"
  echo "- rust-test: ${test_result}"
} >> "$GITHUB_STEP_SUMMARY"

if [ "${fmt_result}" != "success" ] || [ "${test_result}" != "success" ]; then
  echo "::error::At least one required check failed."
  echo "CI_SIGNAL code=GATE_FAIL stage=PostRunSummary result=failure hint=Inspect upstream job summaries and artifacts."
  exit 1
fi
echo "::notice::All required checks passed."
echo "::endgroup::"
