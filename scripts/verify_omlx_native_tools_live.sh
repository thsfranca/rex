#!/usr/bin/env bash
# Operator E2E: R038 native broker tool calling on managed or direct oMLX.
#
# Invoke only when oMLX is running with a tool-capable MLX model. Not wired to PR CI (RC-10).
# Configuration comes from Rex fixture templates under fixtures/omlx_native_tools_e2e/
# (only REX_ROOT is set for the isolated harness layout).
#
# Usage:
#   ./scripts/verify_omlx_native_tools_live.sh [direct|managed|autostart]
# Default scenario: direct (explicit openai_compat.base_url at http://127.0.0.1:8000/v1).
#
# Blocked: public stream subprocess removed; use desktop operator path or rex-ui-harness.
set -euo pipefail

echo "verify_omlx_native_tools_live: blocked — use bare \`rex\` (desktop) per docs/OPERATOR_UX.md or rex-ui-harness (docs/WEB_UI_AGENT_VALIDATION.md)." >&2
exit 1

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

FIXTURE_DIR="${ROOT_DIR}/fixtures/omlx_native_tools_e2e"
NATIVE_FIXTURE_DIR="${ROOT_DIR}/fixtures/native_tools_e2e"
TARGET_DIR="${CARGO_TARGET_DIR:-${ROOT_DIR}/target}"
REX_BIN="${TARGET_DIR}/debug/rex"

SCENARIO="${1:-direct}"
case "${SCENARIO}" in
  direct|managed|autostart) ;;
  *)
    echo "verify_omlx_native_tools_live: unknown scenario '${SCENARIO}' (use direct|managed|autostart)" >&2
    exit 2
    ;;
esac

FIXTURE_TEMPLATE="${FIXTURE_DIR}/config.${SCENARIO}.json"
[[ -f "${FIXTURE_TEMPLATE}" ]] || {
  echo "verify_omlx_native_tools_live: missing fixture ${FIXTURE_TEMPLATE}" >&2
  exit 2
}

MARKER="rex-native-e2e-marker"
OMLX_PROBE_URL="http://127.0.0.1:8000/v1/models"
OMLX_MODEL="qwen2.5-coder-32b"
READINESS_TIMEOUT_SECS=120
TURN_TIMEOUT_SECS=120

DAEMON_PID=""
REX_ROOT=""
WORKSPACE=""
DAEMON_SOCKET=""
SIDECAR_SOCKET=""
DAEMON_LOG=""

fail() {
  echo "verify_omlx_native_tools_live: FAIL — $*" >&2
  exit 1
}

info() {
  echo "==> $*"
}

cleanup() {
  if [[ -n "${DAEMON_PID}" ]]; then
    kill "${DAEMON_PID}" 2>/dev/null || true
    wait "${DAEMON_PID}" 2>/dev/null || true
  fi
  if [[ -n "${DAEMON_SOCKET}" ]]; then
    rm -f "${DAEMON_SOCKET}"
  fi
  if [[ -n "${SIDECAR_SOCKET}" ]]; then
    rm -f "${SIDECAR_SOCKET}"
  fi
}
trap cleanup EXIT

ndjson_collect_text() {
  local ndjson_file="$1"
  python3 - "${ndjson_file}" <<'PY'
import json
import sys

path = sys.argv[1]
chunks: list[str] = []
errors: list[str] = []
with open(path, encoding="utf-8") as fh:
    for raw in fh:
        line = raw.strip()
        if not line:
            continue
        try:
            obj = json.loads(line)
        except json.JSONDecodeError as exc:
            errors.append(f"invalid ndjson: {exc}")
            continue
        event = obj.get("event")
        if event == "chunk":
            chunks.append(obj.get("text") or "")
        elif event == "error":
            errors.append(obj.get("message") or obj.get("code") or "unknown error")
        elif event == "done" and obj.get("error"):
            errors.append(str(obj.get("error")))
if errors:
    print("\n".join(errors), file=sys.stderr)
    sys.exit(2)
print("".join(chunks), end="")
PY
}

run_complete_ndjson() {
  local mode="$1"
  local prompt="$2"
  local out_file="$3"
  if ! env REX_ROOT="${REX_ROOT}" "${REX_BIN}" complete --format ndjson --mode "${mode}" -- "${prompt}" >"${out_file}" 2>&1; then
    fail "${mode} turn failed (see ${out_file})"
  fi
}

wait_for_daemon_ready() {
  local deadline=$((SECONDS + READINESS_TIMEOUT_SECS))
  while (( SECONDS < deadline )); do
    if env REX_ROOT="${REX_ROOT}" "${REX_BIN}" __rex_internal_status >/dev/null 2>&1; then
      return 0
    fi
    if [[ -n "${DAEMON_PID}" ]] && ! kill -0 "${DAEMON_PID}" 2>/dev/null; then
      fail "daemon exited before ready (see ${DAEMON_LOG})"
    fi
    sleep 0.25
  done
  fail "daemon not ready within ${READINESS_TIMEOUT_SECS}s (see ${DAEMON_LOG})"
}

assert_plan_native_protocol() {
  local log_slice="$1"
  if ! grep -qE 'broker\.inference=ok .*mode=plan .*protocol=1' <<<"${log_slice}"; then
    fail "expected broker.inference native protocol=1 on plan turn"
  fi
}

info "Building rex CLI"
cargo build -p rex --locked >/dev/null
[[ -x "${REX_BIN}" ]] || fail "rex binary missing at ${REX_BIN}"

info "Probing oMLX at ${OMLX_PROBE_URL}"
if ! curl -sf "${OMLX_PROBE_URL}" >/dev/null; then
  fail "oMLX not reachable at ${OMLX_PROBE_URL} (start oMLX or use scenario managed with inference.omlx.mode: managed)"
fi

PYTHON="python3"
info "Installing rex-agent sidecar (editable)"
"${PYTHON}" -m pip install -q -e "${ROOT_DIR}/sidecars/rex-agent" || fail "pip install rex-agent failed"

REX_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/rex-omlx-e2e-root.XXXXXX")"
WORKSPACE="$(mktemp -d "${TMPDIR:-/tmp}/rex-omlx-e2e-ws.XXXXXX")"
DAEMON_SOCKET="${WORKSPACE}/rex-daemon.sock"
SIDECAR_SOCKET="${WORKSPACE}/rex-sidecar.sock"
DAEMON_LOG="${WORKSPACE}/daemon.log"

mkdir -p "${WORKSPACE}/workspace"
cp "${NATIVE_FIXTURE_DIR}/workspace/native_fixture.txt" "${WORKSPACE}/workspace/native_fixture.txt"

info "Writing ${REX_ROOT}/config.json from fixture scenario=${SCENARIO}"
python3 - "${FIXTURE_TEMPLATE}" "${REX_ROOT}/config.json" "${WORKSPACE}/workspace" "${DAEMON_SOCKET}" "${SIDECAR_SOCKET}" <<'PY'
import json
import sys

template_path, out_path, workspace, daemon_sock, sidecar_sock = sys.argv[1:6]
raw = open(template_path, encoding="utf-8").read()
raw = raw.replace("__WORKSPACE__", workspace)
raw = raw.replace("__DAEMON_SOCKET__", daemon_sock)
raw = raw.replace("__SIDECAR_SOCKET__", sidecar_sock)
cfg = json.loads(raw)
with open(out_path, "w", encoding="utf-8") as fh:
    json.dump(cfg, fh, indent=2)
    fh.write("\n")
PY

env REX_ROOT="${REX_ROOT}" "${REX_BIN}" proto install || fail "rex proto install failed"

if [[ "${SCENARIO}" == "autostart" ]]; then
  info "Autostart via internal status (R071 + managed oMLX from config)"
  env REX_ROOT="${REX_ROOT}" "${REX_BIN}" __rex_internal_status >/dev/null
  cp "${REX_ROOT}/daemon.log" "${DAEMON_LOG}" 2>/dev/null || true
else
  info "Starting internal daemon (log: ${DAEMON_LOG})"
  env REX_ROOT="${REX_ROOT}" "${REX_BIN}" __rex_internal_daemon >>"${DAEMON_LOG}" 2>&1 &
  DAEMON_PID=$!
  wait_for_daemon_ready
fi

PLAN_NDJSON="${WORKSPACE}/plan.ndjson"
PLAN_PROMPT='Read native_fixture.txt and quote the exact file contents in your answer.'
log_offset_before_plan="$(wc -c <"${DAEMON_LOG}" | tr -d ' ')"
info "Plan turn: ${PLAN_PROMPT}"
run_complete_ndjson plan "${PLAN_PROMPT}" "${PLAN_NDJSON}"
plan_text="$(ndjson_collect_text "${PLAN_NDJSON}")" || fail "plan NDJSON parse error"
[[ "${plan_text}" == *"${MARKER}"* ]] || fail "plan answer missing marker '${MARKER}'"

log_slice_plan="$(tail -c +"$((log_offset_before_plan + 1))" "${DAEMON_LOG}" 2>/dev/null || true)"
assert_plan_native_protocol "${log_slice_plan}"

cat <<EOF

==> verify_omlx_native_tools_live passed (oMLX operator E2E, scenario=${SCENARIO}).

Plan native tool path exercised on oMLX (${OMLX_MODEL}).
Daemon log: ${DAEMON_LOG}
EOF
