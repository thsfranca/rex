#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=scripts/ci/workspace_excludes.sh
source "${ROOT_DIR}/scripts/ci/workspace_excludes.sh"

mkdir -p ci-observability
result="success"
fail_code="-"
fail_stage="-"
hint="-"

echo "::group::Setup"
echo "::notice::CI stage Setup complete; dependencies and toolchain are ready."
echo "::endgroup::"

echo "::group::BuildAndChecks"
echo "::notice::No build-only checks in this job."
echo "::endgroup::"

echo "::group::TestExecution"
# Integration tests (e.g. mvp_product_path) spawn rex-sidecar-stub; build before the test run.
cargo build -p rex-sidecar-stub --locked
cargo build -p rex --locked
if command -v cargo-nextest >/dev/null 2>&1; then
  echo "::notice::Using cargo-nextest (CI or local install)."
  test_cmd=(cargo nextest run --workspace --all-targets --locked $(ci_workspace_excludes))
else
  echo "::notice::Using cargo test (install cargo-nextest for faster runs)."
  test_cmd=(cargo test --workspace --all-targets --locked $(ci_workspace_excludes))
fi
if ! "${test_cmd[@]}" 2>&1 | tee "ci-observability/test.log"; then
  result="failure"
  fail_code="TEST_FAIL"
  fail_stage="TestExecution"
  hint="Run cargo test --workspace --all-targets --locked locally (or: cargo install cargo-nextest && cargo nextest run --workspace --all-targets --locked)."
  echo "::error::Test execution failed."
  echo "CI_SIGNAL code=${fail_code} stage=${fail_stage} result=${result} hint=${hint}"
elif ! ./scripts/ci/test_ci_path_relevance.sh 2>&1 | tee "ci-observability/path-relevance-test.log"; then
  result="failure"
  fail_code="TEST_FAIL"
  fail_stage="TestExecution"
  hint="Run scripts/ci/test_ci_path_relevance.sh locally."
  echo "::error::CI path relevance contract tests failed."
  echo "CI_SIGNAL code=${fail_code} stage=${fail_stage} result=${result} hint=${hint}"
elif ! ./scripts/ci/test_ci_path_filters_sync.sh 2>&1 | tee "ci-observability/path-filters-sync-test.log"; then
  result="failure"
  fail_code="TEST_FAIL"
  fail_stage="TestExecution"
  hint="Run scripts/ci/test_ci_path_filters_sync.sh locally."
  echo "::error::CI path filter sync check failed."
  echo "CI_SIGNAL code=${fail_code} stage=${fail_stage} result=${result} hint=${hint}"
elif ! ./scripts/ci/test_enforce_ci_gate.sh 2>&1 | tee "ci-observability/gate-script-test.log"; then
  result="failure"
  fail_code="TEST_FAIL"
  fail_stage="TestExecution"
  hint="Run scripts/ci/test_enforce_ci_gate.sh locally."
  echo "::error::Gate script contract tests failed."
  echo "CI_SIGNAL code=${fail_code} stage=${fail_stage} result=${result} hint=${hint}"
elif ! ./scripts/ci/test_extract_log_excerpt.sh 2>&1 | tee "ci-observability/extract-log-excerpt-test.log"; then
  result="failure"
  fail_code="TEST_FAIL"
  fail_stage="TestExecution"
  hint="Run scripts/ci/test_extract_log_excerpt.sh locally."
  echo "::error::Log excerpt contract tests failed."
  echo "CI_SIGNAL code=${fail_code} stage=${fail_stage} result=${result} hint=${hint}"
elif ! ./scripts/ci/test_scripts_syntax.sh 2>&1 | tee "ci-observability/scripts-syntax-test.log"; then
  result="failure"
  fail_code="TEST_FAIL"
  fail_stage="TestExecution"
  hint="Run scripts/ci/test_scripts_syntax.sh locally."
  echo "::error::Scripts syntax check failed."
  echo "CI_SIGNAL code=${fail_code} stage=${fail_stage} result=${result} hint=${hint}"
fi
echo "::endgroup::"

./scripts/ci/finish_verify_job.sh
