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
  local rust_changed="$6"
  local extension_changed="$7"
  local sidecar_changed="$8"
  local guidelines_changed="$9"
  local ci_changed="${10}"
  local global_changed="${11}"

  local tmp_out
  tmp_out="$(mktemp)"
  RUST_CHANGED="${rust_changed}" \
  EXTENSION_CHANGED="${extension_changed}" \
  SIDECAR_CHANGED="${sidecar_changed}" \
  GUIDELINES_CHANGED="${guidelines_changed}" \
  CI_CHANGED="${ci_changed}" \
  GLOBAL_CHANGED="${global_changed}" \
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
  rm -f "${tmp_out}"
}

assert_relevance "docs-only" false false false false \
  false false false false false false
assert_relevance "rust-only" true false false false \
  true false false false false false
assert_relevance "extension-only" false true false false \
  false true false false false false
assert_relevance "sidecar-only" false false true false \
  false false true false false false
assert_relevance "guidelines-only" false false false true \
  false false false true false false
assert_relevance "ci-scripts" true true false false \
  false false false false true false
assert_relevance "cargo-lock" true true false false \
  false false false false false true
assert_relevance "sidecar-plus-ci" true true true false \
  false false true false true false

echo "ci path relevance contract tests passed."
