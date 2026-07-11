#!/usr/bin/env bash
# Electron compositor proof (ADR 0043 / W126): chrome + fullscreen WebGL ≥5s.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
APP_DIR="${ROOT}/apps/rex-desktop-electron"
cd "$ROOT"
mkdir -p ci-observability

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

if [ "$(uname -s)" != "Darwin" ]; then
  mark_failure "Setup" "ENV_SETUP_FAIL" "Electron compositor proof requires macOS (v1 host)."
elif ! command -v node >/dev/null 2>&1; then
  mark_failure "Setup" "ENV_SETUP_FAIL" "node not found."
elif ! command -v npm >/dev/null 2>&1; then
  mark_failure "Setup" "ENV_SETUP_FAIL" "npm not found."
fi

echo "::group::Setup"
if [ "${result}" = "success" ]; then
  cd "${APP_DIR}"
  if [ ! -d node_modules ]; then
    if ! npm install 2>&1 | tee "${ROOT}/ci-observability/electron-compositor-npm.log"; then
      mark_failure "Setup" "NPM_CI_FAIL" "npm install failed in apps/rex-desktop-electron."
    fi
  fi
fi
echo "::endgroup::"

echo "::group::TestExecution"
PROOF_LOG="${ROOT}/ci-observability/electron-compositor-proof.log"
if [ "${result}" = "success" ]; then
  set +e
  npm run compositor-proof 2>&1 | tee "${PROOF_LOG}"
  proof_exit=${PIPESTATUS[0]}
  set -e
  if [ "${proof_exit}" -ne 0 ]; then
    mark_failure "TestExecution" "UI_FAIL" "Compositor proof failed; run ./scripts/ci/run_electron_compositor_proof.sh locally on macOS."
  fi
fi

if [ "${result}" = "success" ]; then
  set +e
  npm run compositor-proof:bury-expect-fail 2>&1 | tee -a "${PROOF_LOG}"
  bury_exit=${PIPESTATUS[0]}
  set -e
  if [ "${bury_exit}" -ne 0 ]; then
    mark_failure "TestExecution" "UI_FAIL" "Bury expect-fail check did not fail as required; see ${PROOF_LOG}."
  fi
fi
echo "::endgroup::"

cd "$ROOT"
result="${result}" fail_code="${fail_code}" fail_stage="${fail_stage}" hint="${hint}" \
  ./scripts/ci/finish_verify_job.sh
