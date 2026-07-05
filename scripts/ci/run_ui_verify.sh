#!/usr/bin/env bash
# Web UI verify: rex-web build + rex-ui-harness CI scenarios (build or desktop).
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

if ! command -v node >/dev/null 2>&1; then
  mark_failure "Setup" "ENV_SETUP_FAIL" "node not found."
elif ! command -v npm >/dev/null 2>&1; then
  mark_failure "Setup" "ENV_SETUP_FAIL" "npm not found."
fi

if [ "${result}" = "success" ] && [ "${MODE}" = "desktop" ]; then
  if ! command -v cargo >/dev/null 2>&1; then
    mark_failure "Setup" "ENV_SETUP_FAIL" "cargo not found (required for desktop mode)."
  fi
  if ! command -v protoc >/dev/null 2>&1; then
    if command -v brew >/dev/null 2>&1; then
      echo "::notice::Installing protobuf via brew for desktop build."
      brew install protobuf 2>&1 | tee "ci-observability/ui-protoc.log" || \
        mark_failure "Setup" "ENV_SETUP_FAIL" "protoc missing; install protobuf-compiler."
    else
      mark_failure "Setup" "ENV_SETUP_FAIL" "protoc not found (required for rex-desktop build)."
    fi
  fi
fi
echo "::endgroup::"

echo "::group::BuildAndChecks"
if [ "${result}" = "success" ] && [ "${SKIP_WEB_BUILD}" = "false" ]; then
  if ! (cd "${WEB_DIR}" && npm ci && npm run build) 2>&1 | tee "ci-observability/ui-web-build.log"; then
    mark_failure "BuildAndChecks" "UI_BUILD_FAIL" "rex-web npm ci/build failed; see ci-observability/ui-web-build.log."
  fi
elif [ "${result}" = "success" ] && [ "${MODE}" = "desktop" ]; then
  require_dist "${WEB_DIR}/dist" "rex-web"
  if [ "${result}" = "success" ]; then
    if ! (cd "${WEB_DIR}" && npm ci) 2>&1 | tee -a "ci-observability/ui-web-build.log"; then
      mark_failure "BuildAndChecks" "UI_BUILD_FAIL" "rex-web npm ci failed (dist reused from ui-build)."
    fi
  fi
fi

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

if [ "${result}" = "success" ] && [ "${MODE}" = "build" ]; then
  : # npm build only; no playwright install or desktop cargo build
fi

if [ "${result}" = "success" ] && [ "${MODE}" = "desktop" ]; then
  export REX_SIDECAR_HARNESS="${REX_SIDECAR_HARNESS:-direct}"
  if ! cargo build -p rex -p rex-desktop --features e2e-testing --locked 2>&1 | tee "ci-observability/ui-desktop-build.log"; then
    mark_failure "BuildAndChecks" "UI_BUILD_FAIL" "cargo build -p rex -p rex-desktop --features e2e-testing failed."
  fi
fi
echo "::endgroup::"

echo "::group::TestExecution"
if [ "${result}" = "success" ]; then
  DESKTOP_SOCKET=""
  RUN_ARGS=(--mode "${MODE}")
  if [ "${MODE}" = "desktop" ]; then
    DESKTOP_SOCKET="${TMPDIR:-/tmp}/rex-playwright-${GITHUB_RUN_ID:-local}-$$.sock"
    export REX_ROOT="${ROOT}/fixtures/ui_probe/rex_root"
    export REX_SIDECAR_HARNESS="${REX_SIDECAR_HARNESS:-direct}"
    export TAURI_PLAYWRIGHT_SOCKET="${DESKTOP_SOCKET}"
    RUN_ARGS+=(--socket "${DESKTOP_SOCKET}")
  fi

  HARNESS_LOG="ci-observability/ui-harness.log"
  set +e
  node "${HARNESS_DIR}/dist/run-ci.js" "${RUN_ARGS[@]}" 2>&1 | tee "${HARNESS_LOG}"
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
      mark_failure "TestExecution" "UI_FAIL" "UI harness failed: ${fail_summary} Run ./scripts/ci/run_ui_verify.sh --mode ${MODE} locally."
    else
      mark_failure "TestExecution" "UI_FAIL" "UI harness scenarios failed; see ${HARNESS_LOG}. Run ./scripts/ci/run_ui_verify.sh --mode ${MODE} locally."
    fi
  fi
fi
echo "::endgroup::"

./scripts/ci/finish_verify_job.sh
