#!/usr/bin/env bash
set -euo pipefail

rust_relevant="${RUST_RELEVANT:-true}"
extension_relevant="${EXTENSION_RELEVANT:-true}"
rust_result="${RUST_RESULT:-missing}"
extension_result="${EXTENSION_RESULT:-missing}"
result="success"
fail_stage="-"
fail_code="-"
hint="-"

domain_ok() {
  local relevant="$1"
  local res="$2"
  if [ "${relevant}" = "true" ]; then
    [ "${res}" = "success" ] && return 0
    return 1
  fi
  if [ "${res}" = "success" ] || [ "${res}" = "skipped" ]; then
    return 0
  fi
  return 1
}

mkdir -p ci-observability

echo "::group::Setup"
echo "::notice::Evaluating top-level CI gate dependencies."
echo "::endgroup::"

echo "::group::BuildAndChecks"
echo "::notice::No build/check execution in gate job."
echo "::endgroup::"

echo "::group::TestExecution"
echo "::notice::No test execution in gate job."
echo "::endgroup::"

echo "::group::PostRunSummary"
if ! domain_ok "${rust_relevant}" "${rust_result}" || ! domain_ok "${extension_relevant}" "${extension_result}"; then
  result="failure"
  fail_stage="PostRunSummary"
  fail_code="GATE_FAIL"
  hint="Inspect rust-checks and extension-checks summaries; when a domain is not relevant, upstream may be skipped."
fi

{
  echo "### ci-checks"
  echo ""
  echo "- result: ${result}"
  echo "- fail_stage: ${fail_stage}"
  echo "- fail_code: ${fail_code}"
  echo "- hint: ${hint}"
  echo "- run_id: ${GITHUB_RUN_ID:-unknown}"
  echo ""
  echo "- rust-checks: ${rust_result}"
  echo "- extension-checks: ${extension_result}"
} >> "$GITHUB_STEP_SUMMARY"

{
  echo "result=${result}"
  echo "fail_stage=${fail_stage}"
  echo "fail_code=${fail_code}"
  echo "hint=${hint}"
  echo "rust_checks=${rust_result}"
  echo "extension_checks=${extension_result}"
} > "ci-observability/ci-gate-summary.txt"

if [ "${result}" != "success" ]; then
  echo "::error::Top-level CI gate failed."
  echo "CI_SIGNAL code=${fail_code} stage=${fail_stage} result=${result} hint=${hint}"
  exit 1
fi
echo "::notice::Top-level CI gate passed."
echo "::endgroup::"
