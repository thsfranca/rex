#!/usr/bin/env bash
# Contract tests for scripts/ci/evaluate_ci_relevance.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EVALUATE_SCRIPT="${SCRIPT_DIR}/evaluate_ci_relevance.sh"

assert_relevance() {
  local case_name="$1"
  local expected_rust="$2"
  local expected_sidecar="$3"
  local expected_guidelines="$4"
  local expected_rust_codeql="$5"
  local expected_python_codeql="$6"
  local rust_verify_changed="${7:-false}"
  local sidecar_changed="${8:-false}"
  local guidelines_changed="${9:-false}"
  local rust_codeql_changed="${10:-false}"
  local python_codeql_changed="${11:-false}"

  local tmp_out
  tmp_out="$(mktemp)"
  RUST_VERIFY_CHANGED="${rust_verify_changed}" \
  SIDECAR_CHANGED="${sidecar_changed}" \
  GUIDELINES_CHANGED="${guidelines_changed}" \
  RUST_CODEQL_CHANGED="${rust_codeql_changed}" \
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
  assert_output sidecar_relevant "${expected_sidecar}"
  assert_output guidelines_relevant "${expected_guidelines}"
  assert_output rust_codeql_relevant "${expected_rust_codeql}"
  assert_output python_codeql_relevant "${expected_python_codeql}"
  rm -f "${tmp_out}"
}

assert_relevance "docs-only" false false false false false
assert_relevance "rust-source" true false false true false \
  true false false true false
assert_relevance "sidecar-only" false true false false true \
  false true false false true
assert_relevance "guidelines-only" false false true false false \
  false false true false false
assert_relevance "ci-scripts" false false false false false
assert_relevance "cargo-lock" true false false false false \
  true false false false false
assert_relevance "sidecar-plus-ci-script" false true false false true \
  false true false false true
assert_relevance "rust-ci-script-only" false false false false false

echo "ci path relevance contract tests passed."
