#!/usr/bin/env bash
set -euo pipefail

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
if ! cargo test --workspace --all-targets --locked 2>&1 | tee "ci-observability/test.log"; then
  result="failure"
  fail_code="TEST_FAIL"
  fail_stage="TestExecution"
  hint="Run cargo test locally."
  echo "::error::Test execution failed."
  echo "CI_SIGNAL code=${fail_code} stage=${fail_stage} result=${result} hint=${hint}"
elif ! ./scripts/ci/test_enforce_rust_gate.sh 2>&1 | tee "ci-observability/gate-script-test.log"; then
  result="failure"
  fail_code="TEST_FAIL"
  fail_stage="TestExecution"
  hint="Run scripts/ci/test_enforce_rust_gate.sh locally."
  echo "::error::Gate script contract tests failed."
  echo "CI_SIGNAL code=${fail_code} stage=${fail_stage} result=${result} hint=${hint}"
fi
echo "::endgroup::"

{
  echo "CI_RESULT=${result}"
  echo "CI_FAIL_CODE=${fail_code}"
  echo "CI_FAIL_STAGE=${fail_stage}"
  echo "CI_HINT=${hint}"
} >> "${GITHUB_ENV:-/dev/null}"

if [ "${result}" != "success" ]; then
  exit 1
fi
