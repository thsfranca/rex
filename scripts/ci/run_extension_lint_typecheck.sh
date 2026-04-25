#!/usr/bin/env bash
set -euo pipefail

ext_dir="extensions/rex-vscode"
artifacts_dir="$(pwd)/ci-observability"
mkdir -p "${artifacts_dir}"

result="success"
fail_code="-"
fail_stage="-"
hint="-"

mark_failure() {
  local stage="$1"
  local code="$2"
  local hint_value="$3"
  result="failure"
  fail_stage="${stage}"
  fail_code="${code}"
  hint="${hint_value}"
  echo "::error::${stage} failed (${code}). ${hint_value}"
  echo "CI_SIGNAL code=${code} stage=${stage} result=${result} hint=${hint_value}"
}

cd "${ext_dir}"

echo "::group::Setup"
echo "::notice::node $(node --version) / npm $(npm --version)"
if ! npm ci --no-audit --no-fund 2>&1 | tee "${artifacts_dir}/npm-ci.log"; then
  mark_failure "Setup" "ENV_SETUP_FAIL" "Run npm ci locally in extensions/rex-vscode."
fi
echo "::endgroup::"

echo "::group::BuildAndChecks"
if [ "${result}" = "success" ]; then
  if ! npm run typecheck 2>&1 | tee "${artifacts_dir}/typecheck.log"; then
    mark_failure "BuildAndChecks" "TYPECHECK_FAIL" "Run npm run typecheck locally."
  fi
fi
if [ "${result}" = "success" ]; then
  if ! npm run lint 2>&1 | tee "${artifacts_dir}/lint.log"; then
    mark_failure "BuildAndChecks" "LINT_FAIL" "Run npm run lint locally."
  fi
fi
echo "::endgroup::"

echo "::group::TestExecution"
echo "::notice::No test execution in this job."
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
