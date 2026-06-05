#!/usr/bin/env bash
# Builtin sidecar verify (rex-sidecar-stub + rex-agent). CI contract: Setup → BuildAndChecks → TestExecution.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
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

echo "::group::Setup"
MANIFEST="${ROOT}/scripts/ci/builtin_sidecars.txt"
if [[ ! -f "$MANIFEST" ]]; then
  mark_failure "Setup" "ENV_SETUP_FAIL" "Missing scripts/ci/builtin_sidecars.txt."
else
  echo "::notice::Builtin sidecars:"
  grep -v '^#' "$MANIFEST" | grep -v '^[[:space:]]*$' || true
fi

PYTHON="python3"
if command -v python3.11 >/dev/null 2>&1; then
  PYTHON="python3.11"
elif command -v python3.10 >/dev/null 2>&1; then
  PYTHON="python3.10"
elif ! command -v python3 >/dev/null 2>&1; then
  mark_failure "Setup" "ENV_SETUP_FAIL" "python3 not found (rex-agent requires 3.10+)."
fi

if [ "${result}" = "success" ]; then
  if ! "${PYTHON}" -m pip install -q grpcio-tools grpcio protobuf "langgraph>=0.2.0" "langchain-core>=0.3.0" pytest "ruff>=0.8" 2>&1 | tee "ci-observability/sidecar-pip.log"; then
    mark_failure "Setup" "ENV_SETUP_FAIL" "pip install failed; see ci-observability/sidecar-pip.log."
  fi
fi

REX_AGENT_ROOT="${REX_AGENT_ROOT:-$(mktemp -d "${TMPDIR:-/tmp}/rex-agent-ci.XXXXXX")}"
export REX_ROOT="$REX_AGENT_ROOT"
export REX_PROTO_SRC="$ROOT/proto"
echo "::endgroup::"

echo "::group::BuildAndChecks"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
REX_BIN="$TARGET_DIR/debug/rex"
STUB_BIN="$TARGET_DIR/debug/rex-sidecar-stub"
AGENT_LAUNCHER="$ROOT/sidecars/rex-agent/rex-agent"

if [ "${result}" = "success" ]; then
  if ! cargo build -p rex-sidecar-stub -p rex --locked 2>&1 | tee "ci-observability/sidecar-build.log"; then
    mark_failure "BuildAndChecks" "BUILD_FAIL" "cargo build -p rex-sidecar-stub -p rex failed."
  fi
fi

if [ "${result}" = "success" ]; then
  if [[ ! -x "$REX_BIN" ]]; then
    mark_failure "BuildAndChecks" "BUILD_FAIL" "rex binary missing at ${REX_BIN} (build rex before proto install)."
  elif ! "$REX_BIN" proto install 2>&1 | tee "ci-observability/sidecar-proto.log"; then
    mark_failure "BuildAndChecks" "ENV_SETUP_FAIL" "rex proto install failed."
  fi
fi

if [ "${result}" = "success" ]; then
  if [[ ! -x "$STUB_BIN" ]]; then
    mark_failure "BuildAndChecks" "BUILD_FAIL" "rex-sidecar-stub binary missing at ${STUB_BIN}."
  elif [[ ! -f "$AGENT_LAUNCHER" ]]; then
    mark_failure "BuildAndChecks" "BUILD_FAIL" "rex-agent launcher missing at ${AGENT_LAUNCHER}."
  fi
fi
echo "::endgroup::"

echo "::group::TestExecution"
if [ "${result}" = "success" ]; then
  if ! "${ROOT}/scripts/ci/run_stub_sidecar_checks.sh" 2>&1 | tee "ci-observability/stub-sidecar.log"; then
    mark_failure "TestExecution" "SIDECAR_FAIL" "rex-sidecar-stub checks failed; run ./scripts/ci/run_stub_sidecar_checks.sh locally."
  fi
fi
if [ "${result}" = "success" ]; then
  export PYTHONPATH="$ROOT/sidecars/rex-agent/src:$("$REX_BIN" proto path):${PYTHONPATH:-}"
  if ! "${ROOT}/scripts/ci/run_rex_agent_checks.sh" 2>&1 | tee "ci-observability/rex-agent.log"; then
    mark_failure "TestExecution" "SIDECAR_FAIL" "rex-agent checks failed; run ./scripts/ci/run_rex_agent_checks.sh locally."
  fi
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
