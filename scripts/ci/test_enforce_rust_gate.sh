#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
GATE_SCRIPT="${ROOT_DIR}/scripts/ci/enforce_rust_gate.sh"

assert_contains() {
  local haystack="$1"
  local needle="$2"
  if [[ "${haystack}" != *"${needle}"* ]]; then
    echo "Expected output to contain: ${needle}"
    exit 1
  fi
}

run_gate_case() {
  local fmt_result="$1"
  local test_result="$2"
  local expected_exit="$3"
  local expected_summary_line="$4"
  local expected_signal_line="$5"

  local tmp_dir
  tmp_dir="$(mktemp -d)"
  local summary_file="${tmp_dir}/summary.md"
  touch "${summary_file}"

  local output
  set +e
  output="$(
    FMT_RESULT="${fmt_result}" \
    TEST_RESULT="${test_result}" \
    GITHUB_RUN_ID="local-test-run-id" \
    GITHUB_STEP_SUMMARY="${summary_file}" \
    bash "${GATE_SCRIPT}" 2>&1
  )"
  local exit_code=$?
  set -e

  if [[ "${exit_code}" -ne "${expected_exit}" ]]; then
    echo "Unexpected exit code. expected=${expected_exit} actual=${exit_code}"
    echo "${output}"
    exit 1
  fi

  assert_contains "${output}" "${expected_signal_line}"

  local summary_contents
  summary_contents="$(cat "${summary_file}")"
  assert_contains "${summary_contents}" "${expected_summary_line}"
  assert_contains "${summary_contents}" "- run_id: local-test-run-id"
  assert_contains "${summary_contents}" "- fail_stage:"
  assert_contains "${summary_contents}" "- fail_code:"
  assert_contains "${summary_contents}" "- hint:"

  rm -rf "${tmp_dir}"
}

run_gate_case "success" "success" 0 "- result: success" "::notice::All required checks passed."
run_gate_case "failure" "success" 1 "- fail_code: GATE_FAIL" "CI_SIGNAL code=GATE_FAIL stage=PostRunSummary result=failure hint=Inspect upstream job summaries and artifacts."
run_gate_case "success" "failure" 1 "- rust-test: failure" "CI_SIGNAL code=GATE_FAIL stage=PostRunSummary result=failure hint=Inspect upstream job summaries and artifacts."

echo "enforce_rust_gate contract tests passed."
