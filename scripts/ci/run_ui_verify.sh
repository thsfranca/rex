#!/usr/bin/env bash
# Web UI verify: rex-web build + rex-ui-harness CI scenarios (static or desktop).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
mkdir -p ci-observability

MODE=""
while [ $# -gt 0 ]; do
  case "$1" in
    --mode)
      MODE="${2:-}"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

if [ "${MODE}" != "static" ] && [ "${MODE}" != "desktop" ]; then
  echo "Usage: $0 --mode static|desktop" >&2
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

if [ "${result}" = "success" ] && [ "${MODE}" = "static" ]; then
  if ! (cd "${HARNESS_DIR}" && npx playwright install chromium) 2>&1 | tee "ci-observability/ui-playwright-install.log"; then
    mark_failure "BuildAndChecks" "ENV_SETUP_FAIL" "playwright install chromium failed."
  fi
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

  if ! node "${HARNESS_DIR}/dist/run-ci.js" "${RUN_ARGS[@]}" > >(tee "ci-observability/ui-harness.log") 2>&1; then
    mark_failure "TestExecution" "UI_FAIL" "UI harness scenarios failed; run ./scripts/ci/run_ui_verify.sh --mode ${MODE} locally."
  fi
fi
echo "::endgroup::"

./scripts/ci/finish_verify_job.sh
if [ "${result}" != "success" ]; then
  exit 1
fi
