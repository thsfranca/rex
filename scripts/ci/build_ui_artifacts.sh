#!/usr/bin/env bash
# Shared UI build: rex-web + rex-ui-harness dist artifacts for ui-verify matrix legs.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
mkdir -p ci-observability

WEB_DIR="${ROOT}/apps/rex-web"
HARNESS_DIR="${ROOT}/crates/rex-ui-harness"

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

echo "::group::Setup"
if ! command -v node >/dev/null 2>&1; then
  mark_failure "Setup" "ENV_SETUP_FAIL" "node not found."
elif ! command -v npm >/dev/null 2>&1; then
  mark_failure "Setup" "ENV_SETUP_FAIL" "npm not found."
fi
echo "::endgroup::"

echo "::group::BuildAndChecks"
if [ "${result}" = "success" ]; then
  if ! (cd "${WEB_DIR}" && npm ci && npm run build) 2>&1 | tee "ci-observability/ui-web-build.log"; then
    mark_failure "BuildAndChecks" "UI_BUILD_FAIL" "rex-web npm ci/build failed; see ci-observability/ui-web-build.log."
  fi
fi

if [ "${result}" = "success" ]; then
  if ! (cd "${HARNESS_DIR}" && npm ci && npm run build) 2>&1 | tee "ci-observability/ui-harness-build.log"; then
    mark_failure "BuildAndChecks" "UI_BUILD_FAIL" "rex-ui-harness npm ci/build failed."
  fi
fi
echo "::endgroup::"

./scripts/ci/finish_verify_job.sh
