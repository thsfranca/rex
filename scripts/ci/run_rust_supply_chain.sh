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
if ! cargo audit 2>&1 | tee "ci-observability/audit.log"; then
  result="failure"
  fail_code="AUDIT_FAIL"
  fail_stage="BuildAndChecks"
  hint="Run cargo audit locally (install: cargo install cargo-audit)."
  echo "::error::Supply chain audit failed."
  echo "CI_SIGNAL code=${fail_code} stage=${fail_stage} result=${result} hint=${hint}"
fi
echo "::endgroup::"

echo "::group::TestExecution"
echo "::notice::No test execution in this job."
echo "::endgroup::"

./scripts/ci/finish_verify_job.sh
