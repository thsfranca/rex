#!/usr/bin/env bash
# Contract tests for scripts/ci/evaluate_ci_relevance.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EVALUATE_SCRIPT="${SCRIPT_DIR}/evaluate_ci_relevance.sh"

assert_relevance() {
  local case_name="$1"
  local expected_rust="$2"
  local expected_ext="$3"
  local expected_sidecar="$4"
  local expected_guidelines="$5"
  local expected_rust_codeql="$6"
  local expected_ext_codeql="$7"
  local expected_python_codeql="$8"
  local rust_verify_changed="${9:-false}"
  local extension_verify_changed="${10:-false}"
  local sidecar_changed="${11:-false}"
  local guidelines_changed="${12:-false}"
  local rust_codeql_changed="${13:-false}"
  local extension_codeql_changed="${14:-false}"
  local python_codeql_changed="${15:-false}"

  local tmp_out
  tmp_out="$(mktemp)"
  RUST_VERIFY_CHANGED="${rust_verify_changed}" \
  EXTENSION_VERIFY_CHANGED="${extension_verify_changed}" \
  SIDECAR_CHANGED="${sidecar_changed}" \
  GUIDELINES_CHANGED="${guidelines_changed}" \
  RUST_CODEQL_CHANGED="${rust_codeql_changed}" \
  EXTENSION_CODEQL_CHANGED="${extension_codeql_changed}" \
  PYTHON_CODEQL_CHANGED="${python_codeql_changed}" \
  GITHUB_OUTPUT="${tmp_out}" \
  bash "${EVALUATE_SCRIPT}"

  assert_output() {
    local key="$1"
    local expected="$2"
    local actual
    actual="$(grep -E "^${key}=" "${tmp_out}" | cut -d= -f2-)"
    if [ "${actual}" != "${expected}" ]; then
      echo "case ${case_name}: expected ${key}=${expected}, got ${actual}"
      cat "${tmp_out}"
      rm -f "${tmp_out}"
      exit 1
    fi
  }

  assert_output rust_relevant "${expected_rust}"
  assert_output extension_relevant "${expected_ext}"
  assert_output sidecar_relevant "${expected_sidecar}"
  assert_output guidelines_relevant "${expected_guidelines}"
  assert_output rust_codeql_relevant "${expected_rust_codeql}"
  assert_output extension_codeql_relevant "${expected_ext_codeql}"
  assert_output python_codeql_relevant "${expected_python_codeql}"
  rm -f "${tmp_out}"
}

assert_relevance "docs-only" false false false false false false false
assert_relevance "rust-source" true false false false true false false \
  true false false false true false false
assert_relevance "extension-source" false true false false false true false \
  false true false false false true false
assert_relevance "sidecar-only" false false true false false false true \
  false false true false false false true
assert_relevance "guidelines-only" false false false true false false false \
  false false false true false false false
assert_relevance "ci-scripts" false false false false false false false
assert_relevance "cargo-lock" true false false false false false false \
  true false false false false false false
assert_relevance "sidecar-plus-ci-script" false false true false false false true \
  false false true false false false true
assert_relevance "rust-ci-script-only" false false false false false false false

echo "ci path relevance contract tests passed."
