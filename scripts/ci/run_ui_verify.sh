#!/usr/bin/env bash
# Web UI verify: rex-web build + harness (build) or Electron compositor proof (desktop).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
mkdir -p ci-observability

MODE=""
SKIP_WEB_BUILD=false
SKIP_HARNESS_BUILD=false

while [ $# -gt 0 ]; do
  case "$1" in
    --mode)
      MODE="${2:-}"
      shift 2
      ;;
    --skip-web-build)
      SKIP_WEB_BUILD=true
      shift
      ;;
    --skip-harness-build)
      SKIP_HARNESS_BUILD=true
      shift
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

if [ "${MODE}" != "build" ] && [ "${MODE}" != "desktop" ]; then
  echo "Usage: $0 --mode build|desktop [--skip-web-build] [--skip-harness-build]" >&2
  exit 1
fi

if [ "${MODE}" = "desktop" ] && [ "$(uname -s)" != "Darwin" ]; then
  echo "Desktop mode requires macOS." >&2
  exit 1
fi

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

require_dist() {
  local dist_dir="$1"
  local label="$2"
  if [ ! -d "${dist_dir}" ] || [ -z "$(ls -A "${dist_dir}" 2>/dev/null || true)" ]; then
    mark_failure "BuildAndChecks" "UI_BUILD_FAIL" "Missing ${label} dist at ${dist_dir}; run ./scripts/ci/build_ui_artifacts.sh or omit --skip-* flags."
  fi
}

echo "::group::Setup"
WEB_DIR="${ROOT}/apps/rex-web"
HARNESS_DIR="${ROOT}/crates/rex-ui-harness"
DESKTOP_DIR="${ROOT}/apps/rex-desktop"

if ! command -v node >/dev/null 2>&1; then
  mark_failure "Setup" "ENV_SETUP_FAIL" "node not found."
elif ! command -v npm >/dev/null 2>&1; then
  mark_failure "Setup" "ENV_SETUP_FAIL" "npm not found."
fi

if [ "${result}" = "success" ] && [ "${MODE}" = "desktop" ]; then
  if ! command -v cargo >/dev/null 2>&1; then
    mark_failure "Setup" "ENV_SETUP_FAIL" "cargo not found (required for desktop mode)."
  fi
fi
echo "::endgroup::"

echo "::group::BuildAndChecks"
if [ "${result}" = "success" ] && [ "${SKIP_WEB_BUILD}" = "false" ]; then
  if ! (cd "${WEB_DIR}" && npm ci && npm run build) 2>&1 | tee "ci-observability/ui-web-build.log"; then
    mark_failure "BuildAndChecks" "UI_BUILD_FAIL" "rex-web npm ci/build failed; see ci-observability/ui-web-build.log."
  elif ! "${ROOT}/scripts/ci/lint_ui_tokens.sh" 2>&1 | tee -a "ci-observability/ui-web-build.log"; then
    mark_failure "BuildAndChecks" "UI_BUILD_FAIL" "UI token lint failed; see ci-observability/ui-web-build.log."
  elif ! (cd "${WEB_DIR}" && npm test) 2>&1 | tee -a "ci-observability/ui-web-build.log"; then
    mark_failure "BuildAndChecks" "UI_BUILD_FAIL" "rex-web vitest failed; see ci-observability/ui-web-build.log."
  fi
elif [ "${result}" = "success" ] && [ "${MODE}" = "desktop" ]; then
  require_dist "${WEB_DIR}/dist" "rex-web"
  if [ "${result}" = "success" ]; then
    if ! (cd "${WEB_DIR}" && npm ci) 2>&1 | tee -a "ci-observability/ui-web-build.log"; then
      mark_failure "BuildAndChecks" "UI_BUILD_FAIL" "rex-web npm ci failed (dist reused from ui-build)."
    fi
  fi
fi

if [ "${MODE}" = "build" ]; then
  if [ "${result}" = "success" ] && [ "${SKIP_HARNESS_BUILD}" = "false" ]; then
    if ! (cd "${HARNESS_DIR}" && npm ci && npm run build) 2>&1 | tee "ci-observability/ui-harness-build.log"; then
      mark_failure "BuildAndChecks" "UI_BUILD_FAIL" "rex-ui-harness npm ci/build failed."
    fi
  elif [ "${result}" = "success" ]; then
    require_dist "${HARNESS_DIR}/dist" "rex-ui-harness"
    if [ "${result}" = "success" ]; then
      if ! (cd "${HARNESS_DIR}" && npm ci) 2>&1 | tee -a "ci-observability/ui-harness-build.log"; then
        mark_failure "BuildAndChecks" "UI_BUILD_FAIL" "rex-ui-harness npm ci failed (dist reused from ui-build)."
      fi
    fi
  fi
fi

if [ "${result}" = "success" ] && [ "${MODE}" = "desktop" ]; then
  if ! cargo build -p rex --locked 2>&1 | tee "ci-observability/ui-desktop-build.log"; then
    mark_failure "BuildAndChecks" "UI_BUILD_FAIL" "cargo build -p rex failed."
  fi
  if [ "${result}" = "success" ]; then
    if ! (cd "${DESKTOP_DIR}" && npm ci) 2>&1 | tee -a "ci-observability/ui-desktop-build.log"; then
      mark_failure "BuildAndChecks" "UI_BUILD_FAIL" "apps/rex-desktop npm ci failed."
    fi
  fi
fi
echo "::endgroup::"

echo "::group::TestExecution"
if [ "${result}" = "success" ] && [ "${MODE}" = "build" ]; then
  HARNESS_LOG="ci-observability/ui-harness.log"
  set +e
  node "${HARNESS_DIR}/dist/run-ci.js" --mode build 2>&1 | tee "${HARNESS_LOG}"
  harness_exit=${PIPESTATUS[0]}
  set -e
  if [ "${harness_exit}" -ne 0 ]; then
    fail_summary="$(
      "${ROOT}/scripts/ci/extract_ui_harness_failure.sh" "${HARNESS_LOG}" 5 \
        | head -3 \
        | tr '\n' ' ' \
        | sed 's/[[:space:]]*$//'
    )"
    if [ -n "${fail_summary}" ]; then
      mark_failure "TestExecution" "UI_FAIL" "UI harness failed: ${fail_summary} Run ./scripts/ci/run_ui_verify.sh --mode build locally."
    else
      mark_failure "TestExecution" "UI_FAIL" "UI harness scenarios failed; see ${HARNESS_LOG}. Run ./scripts/ci/run_ui_verify.sh --mode build locally."
    fi
  fi
fi

if [ "${result}" = "success" ] && [ "${MODE}" = "desktop" ]; then
  # Desktop gate is Electron compositor proof until harness desktop transport is on Electron (W129).
  set +e
  "${ROOT}/scripts/ci/run_electron_compositor_proof.sh" 2>&1 | tee "ci-observability/electron-compositor-proof.log"
  proof_exit=${PIPESTATUS[0]}
  set -e
  if [ "${proof_exit}" -ne 0 ]; then
    mark_failure "TestExecution" "UI_FAIL" "Electron compositor proof failed; run ./scripts/ci/run_electron_compositor_proof.sh locally on macOS."
  fi
fi
echo "::endgroup::"

result="${result}" fail_code="${fail_code}" fail_stage="${fail_stage}" hint="${hint}" \
  ./scripts/ci/finish_verify_job.sh
