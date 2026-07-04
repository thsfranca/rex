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
if ! cargo fmt --all -- --check 2>&1 | tee "ci-observability/fmt.log"; then
  result="failure"
  fail_code="FMT_FAIL"
  fail_stage="BuildAndChecks"
  hint="Run cargo fmt locally."
  echo "::error::Formatting check failed."
  echo "CI_SIGNAL code=${fail_code} stage=${fail_stage} result=${result} hint=${hint}"
elif ! cargo clippy --workspace --all-targets --locked $(ci_workspace_excludes) -- -D warnings 2>&1 | tee "ci-observability/clippy.log"; then
  result="failure"
  fail_code="CLIPPY_FAIL"
  fail_stage="BuildAndChecks"
  hint="Fix clippy warnings locally."
  echo "::error::Lint check failed."
  echo "CI_SIGNAL code=${fail_code} stage=${fail_stage} result=${result} hint=${hint}"
fi
echo "::endgroup::"

echo "::group::TestExecution"
echo "::notice::No test execution in this job."
echo "::endgroup::"

./scripts/ci/finish_verify_job.sh
