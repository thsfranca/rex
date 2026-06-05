#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
GATE_SCRIPT="${ROOT_DIR}/scripts/ci/enforce_ci_gate.sh"

assert_contains() {
  local haystack="$1"
  local needle="$2"
  if [[ "${haystack}" != *"${needle}"* ]]; then
    echo "Expected output to contain: ${needle}"
    exit 1
  fi
}

run_gate_case() {
  local rust_rel="$1"
  local ext_rel="$2"
  local sidecar_rel="$3"
  local rust_res="$4"
  local ext_res="$5"
  local sidecar_res="$6"
  local expected_exit="$7"
  local expected_signal_line="$8"
  local guidelines_res="${9:-success}"
  local guidelines_rel="${10:-true}"

  local tmp_dir
  tmp_dir="$(mktemp -d)"
  local summary_file="${tmp_dir}/summary.md"
  touch "${summary_file}"

  local output
  set +e
  output="$(
    RUST_RELEVANT="${rust_rel}" \
    EXTENSION_RELEVANT="${ext_rel}" \
    SIDECAR_RELEVANT="${sidecar_rel}" \
    GUIDELINES_RELEVANT="${guidelines_rel}" \
    RUST_RESULT="${rust_res}" \
    EXTENSION_RESULT="${ext_res}" \
    SIDECAR_RESULT="${sidecar_res}" \
    GUIDELINES_RESULT="${guidelines_res}" \
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
  if [[ "${expected_exit}" -eq 0 ]]; then
    assert_contains "${output}" "::notice::Top-level CI gate passed."
  fi

  local summary_contents
  summary_contents="$(cat "${summary_file}")"
  assert_contains "${summary_contents}" "- rust-verify: ${rust_res}"
  assert_contains "${summary_contents}" "- sidecar-verify: ${sidecar_res}"
  assert_contains "${summary_contents}" "- extension-verify: ${ext_res}"
  assert_contains "${summary_contents}" "- guidelines-verify: ${guidelines_res}"
  assert_contains "${summary_contents}" "- run_id: local-test-run-id"

  rm -rf "${tmp_dir}"
}

run_gate_case "true" "true" "true" "success" "success" "success" 0 "::notice::Top-level CI gate passed."
run_gate_case "true" "true" "true" "failure" "success" "success" 1 "CI_SIGNAL code=GATE_FAIL"
run_gate_case "true" "true" "true" "success" "failure" "success" 1 "CI_SIGNAL code=GATE_FAIL"
run_gate_case "true" "true" "true" "success" "success" "failure" 1 "CI_SIGNAL code=GATE_FAIL"
run_gate_case "false" "false" "false" "skipped" "skipped" "skipped" 0 "::notice::Top-level CI gate passed."
run_gate_case "false" "false" "false" "success" "success" "success" 0 "::notice::Top-level CI gate passed."
run_gate_case "false" "true" "false" "skipped" "success" "skipped" 0 "::notice::Top-level CI gate passed."
run_gate_case "true" "false" "true" "success" "skipped" "success" 0 "::notice::Top-level CI gate passed."
run_gate_case "false" "false" "false" "skipped" "skipped" "skipped" 0 "::notice::Top-level CI gate passed." "skipped" "false"
run_gate_case "true" "true" "true" "success" "success" "success" 1 "CI_SIGNAL code=GUIDELINES_FAIL" "failure"

echo "enforce_ci_gate contract tests passed."
