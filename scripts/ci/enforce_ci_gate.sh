#!/usr/bin/env bash
set -euo pipefail

rust_relevant="${RUST_RELEVANT:-true}"
sidecar_relevant="${SIDECAR_RELEVANT:-true}"
guidelines_relevant="${GUIDELINES_RELEVANT:-true}"
ui_relevant="${UI_RELEVANT:-true}"
rust_result="${RUST_RESULT:-missing}"
sidecar_result="${SIDECAR_RESULT:-missing}"
guidelines_result="${GUIDELINES_RESULT:-missing}"
ui_result="${UI_RESULT:-missing}"
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
if ! domain_ok "${rust_relevant}" "${rust_result}" \
  || ! domain_ok "${sidecar_relevant}" "${sidecar_result}" \
  || ! domain_ok "${guidelines_relevant}" "${guidelines_result}" \
  || ! domain_ok "${ui_relevant}" "${ui_result}"; then
  result="failure"
  fail_stage="PostRunSummary"
  fail_code="GATE_FAIL"
  if [ "${guidelines_relevant}" = "true" ] && [ "${guidelines_result}" != "success" ]; then
    fail_code="GUIDELINES_FAIL"
    hint="Guidelines verify failed; run ./scripts/ci/run_guidelines_verify.sh locally."
  elif [ "${ui_relevant}" = "true" ] && [ "${ui_result}" != "success" ]; then
    fail_code="UI_FAIL"
    hint="UI verify failed; run ./scripts/ci/run_ui_verify.sh --mode build locally (desktop on macOS)."
  else
    hint="Inspect rust-verify, sidecar-verify, guidelines-verify, and ui-verify summaries and artifacts; when a domain is not relevant, upstream verify may be skipped."
  fi
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
  echo "- rust-verify: ${rust_result}"
  echo "- sidecar-verify: ${sidecar_result}"
  echo "- guidelines-verify: ${guidelines_result}"
  echo "- ui-verify: ${ui_result}"
} >> "$GITHUB_STEP_SUMMARY"

{
  echo "result=${result}"
  echo "fail_stage=${fail_stage}"
  echo "fail_code=${fail_code}"
  echo "hint=${hint}"
  echo "rust_verify=${rust_result}"
  echo "sidecar_verify=${sidecar_result}"
  echo "guidelines_verify=${guidelines_result}"
  echo "ui_verify=${ui_result}"
} > "ci-observability/ci-gate-summary.txt"

if [ "${result}" != "success" ]; then
  echo "::error::Top-level CI gate failed."
  echo "CI_SIGNAL code=${fail_code} stage=${fail_stage} result=${result} hint=${hint}"
  exit 1
fi
echo "::notice::Top-level CI gate passed."
echo "::endgroup::"
