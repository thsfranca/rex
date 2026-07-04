#!/usr/bin/env bash
# Operator E2E: R038 native broker tool calling on direct Ollama.
#
# Invoke only when Ollama is running with a tool-capable model. Not wired to PR CI (RC-10).
# Defaults: http://127.0.0.1:11434/v1, model qwen2.5-coder:7b (direct Ollama product path).
#
# Daemon logs protocol as numeric enum (proto/rex/v1/rex.proto InferenceProtocol):
#   1 = INFERENCE_PROTOCOL_NATIVE, 3 = INFERENCE_PROTOCOL_INTERIM_FALLBACK
#
# Blocked: public `rex complete` was removed; turns need a TUI or internal harness rewrite.
set -euo pipefail

echo "verify_native_tools_live: blocked — public \`rex complete\` was removed; use \`rex\` / \`rex tui\` for operator dogfood." >&2
exit 1

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

FIXTURE_DIR="${ROOT_DIR}/fixtures/native_tools_e2e"
TARGET_DIR="${CARGO_TARGET_DIR:-${ROOT_DIR}/target}"
REX_BIN="${TARGET_DIR}/debug/rex"
AGENT_LAUNCHER="${ROOT_DIR}/sidecars/rex-agent/rex-agent"

MARKER="rex-native-e2e-marker"
OLLAMA_BASE="http://127.0.0.1:11434/v1"
OLLAMA_HOST="127.0.0.1:11434"
OLLAMA_MODEL="qwen2.5-coder:7b"
READINESS_TIMEOUT_SECS=90
TURN_TIMEOUT_SECS=120

DAEMON_PID=""
REX_ROOT=""
WORKSPACE=""
DAEMON_SOCKET=""
SIDECAR_SOCKET=""
DAEMON_LOG=""

fail() {
  echo "verify_native_tools_live: FAIL — $*" >&2
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
  local timeout_secs="${4:-${TURN_TIMEOUT_SECS}}"
  if ! "${PYTHON}" - "${timeout_secs}" "${REX_BIN}" "${mode}" "${prompt}" "${out_file}" "${REX_ROOT}" <<'PY'
import os
import subprocess
import sys

timeout_secs = int(sys.argv[1])
rex_bin = sys.argv[2]
mode = sys.argv[3]
prompt = sys.argv[4]
out_path = sys.argv[5]
rex_root = sys.argv[6]
env = os.environ.copy()
env["REX_ROOT"] = rex_root
try:
    completed = subprocess.run(
        [rex_bin, "complete", "--format", "ndjson", "--mode", mode, "--", prompt],
        capture_output=True,
        text=True,
        timeout=timeout_secs,
        env=env,
        check=False,
    )
except subprocess.TimeoutExpired:
    sys.exit(124)
with open(out_path, "w", encoding="utf-8") as fh:
    fh.write(completed.stdout)
    if completed.stderr:
        fh.write(completed.stderr)
sys.exit(completed.returncode)
PY
  then
    fail "${mode} turn failed or timed out after ${timeout_secs}s (see ${out_file})"
  fi
}

wait_for_daemon_ready() {
  local deadline=$((SECONDS + READINESS_TIMEOUT_SECS))
  while (( SECONDS < deadline )); do
    if env REX_ROOT="${REX_ROOT}" "${REX_BIN}" __rex_internal_status >/dev/null 2>&1; then
      return 0
    fi
    if ! kill -0 "${DAEMON_PID}" 2>/dev/null; then
      fail "daemon exited before ready (see ${DAEMON_LOG})"
    fi
    sleep 0.25
  done
  fail "daemon not ready within ${READINESS_TIMEOUT_SECS}s (see ${DAEMON_LOG})"
}

assert_plan_native_protocol() {
  local log_slice="$1"
  if ! grep -qE 'broker\.inference=ok .*mode=plan .*protocol=1' <<<"${log_slice}"; then
    echo "${log_slice}" >&2
    fail "expected broker.inference=ok with mode=plan and protocol=1 (native) in daemon log"
  fi
  if grep -qE 'broker\.inference=ok .*mode=plan .*protocol=3' <<<"${log_slice}"; then
    echo "${log_slice}" >&2
    fail "plan turn used interim_fallback (protocol=3); expected native protocol=1 only"
  fi
}

assert_deny_output() {
  local text="$1"
  local lower
  lower="$(printf '%s' "${text}" | tr '[:upper:]' '[:lower:]')"
  if [[ "${lower}" != *"protected_path"* && "${lower}" != *"denied"* && "${lower}" != *"fs.read error"* ]]; then
    fail "expected .env read deny in agent output, got: ${text}"
  fi
}

info "Building rex CLI"
if ! cargo build -p rex --locked >/dev/null; then
  fail "cargo build -p rex failed"
fi
[[ -x "${REX_BIN}" ]] || fail "rex binary missing at ${REX_BIN}"

info "Probing Ollama at ${OLLAMA_BASE}"
if ! curl -sf "http://${OLLAMA_HOST}/api/tags" >/dev/null; then
  fail "Ollama not reachable at http://${OLLAMA_HOST} (start: ollama serve)"
fi

if ! curl -sf "http://${OLLAMA_HOST}/api/tags" | grep -q "\"name\":\"${OLLAMA_MODEL}\""; then
  echo "verify_native_tools_live: WARN — model ${OLLAMA_MODEL} not in ollama list; pull with: ollama pull ${OLLAMA_MODEL}" >&2
fi

PYTHON="python3"
if command -v python3.11 >/dev/null 2>&1; then
  PYTHON="python3.11"
elif command -v python3.10 >/dev/null 2>&1; then
  PYTHON="python3.10"
fi

info "Installing rex-agent sidecar (editable)"
if ! "${PYTHON}" -m pip install -q -e "${ROOT_DIR}/sidecars/rex-agent"; then
  fail "pip install -e sidecars/rex-agent failed"
fi

if [[ -x "${AGENT_LAUNCHER}" ]]; then
  export PATH="${ROOT_DIR}/sidecars/rex-agent:${PATH}"
elif ! command -v rex-agent >/dev/null 2>&1; then
  fail "rex-agent not found; pip install -e sidecars/rex-agent"
fi

REX_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/rex-native-tools-e2e-root.XXXXXX")"
WORKSPACE="$(mktemp -d "${TMPDIR:-/tmp}/rex-native-tools-e2e-ws.XXXXXX")"
DAEMON_SOCKET="${WORKSPACE}/rex-daemon.sock"
SIDECAR_SOCKET="${WORKSPACE}/rex-sidecar.sock"
DAEMON_LOG="${WORKSPACE}/daemon.log"

info "Preparing workspace fixture under ${WORKSPACE}/workspace"
mkdir -p "${WORKSPACE}/workspace"
cp "${FIXTURE_DIR}/workspace/native_fixture.txt" "${WORKSPACE}/workspace/native_fixture.txt"
printf 'secret=%s\n' "do-not-leak" >"${WORKSPACE}/workspace/.env"

info "Writing isolated ${REX_ROOT}/config.json for harness run"
python3 - "${REX_ROOT}/config.json" "${WORKSPACE}/workspace" "${DAEMON_SOCKET}" "${SIDECAR_SOCKET}" "${OLLAMA_BASE}" "${OLLAMA_MODEL}" <<'PY'
import json
import sys

out_path, workspace, daemon_sock, sidecar_sock, base_url, model = sys.argv[1:7]
cfg = {
    "version": 1,
    "daemon": {"socket": daemon_sock},
    "sidecars": {
        "active": "agent",
        "required": True,
        "list": [
            {
                "name": "agent",
                "binary": "rex-agent",
                "enabled": True,
                "socket": sidecar_sock,
            }
        ],
    },
    "inference": {
        "runtime": "http-openai-compat",
        "openai_compat": {
            "base_url": base_url,
            "model": model,
        },
    },
    "workspace": {"root": workspace},
}
with open(out_path, "w", encoding="utf-8") as fh:
    json.dump(cfg, fh, indent=2)
    fh.write("\n")
PY

info "Installing proto stubs into isolated Rex layout"
if ! env REX_ROOT="${REX_ROOT}" "${REX_BIN}" proto install; then
  fail "rex proto install failed (run from repo root so proto/ is discoverable)"
fi

info "Starting internal daemon (log: ${DAEMON_LOG})"
env REX_ROOT="${REX_ROOT}" "${REX_BIN}" __rex_internal_daemon >>"${DAEMON_LOG}" 2>&1 &
DAEMON_PID=$!

info "Waiting for daemon + sidecar ready (timeout ${READINESS_TIMEOUT_SECS}s)"
wait_for_daemon_ready

PLAN_NDJSON="${WORKSPACE}/plan.ndjson"
PLAN_PROMPT='Read native_fixture.txt and quote the exact file contents in your answer.'

log_offset_before_plan="$(wc -c <"${DAEMON_LOG}" | tr -d ' ')"
info "Plan turn (native read loop): ${PLAN_PROMPT}"
run_complete_ndjson plan "${PLAN_PROMPT}" "${PLAN_NDJSON}"

plan_text="$(ndjson_collect_text "${PLAN_NDJSON}")" || fail "plan NDJSON parse error (see ${PLAN_NDJSON})"
if [[ "${plan_text}" != *"${MARKER}"* ]]; then
  fail "plan answer missing fixture marker '${MARKER}'; got: ${plan_text}"
fi
info "Plan answer cites fixture marker"

log_slice_plan="$(tail -c +"$((log_offset_before_plan + 1))" "${DAEMON_LOG}" 2>/dev/null || true)"
assert_plan_native_protocol "${log_slice_plan}"
info "Daemon log confirms native protocol=1 on plan turn (no protocol=3)"

AGENT_READ_NDJSON="${WORKSPACE}/agent_read.ndjson"
AGENT_READ_PROMPT='Read the file native_fixture.txt and quote its exact contents in your answer.'
info "Agent turn (allowed read): ${AGENT_READ_PROMPT}"
run_complete_ndjson agent "${AGENT_READ_PROMPT}" "${AGENT_READ_NDJSON}"
agent_read_text="$(ndjson_collect_text "${AGENT_READ_NDJSON}")" || fail "agent read NDJSON parse error"
if [[ "${agent_read_text}" != *"${MARKER}"* ]]; then
  fail "agent allowed read missing marker '${MARKER}'; got: ${agent_read_text}"
fi
info "Agent allowed read cites fixture marker"

AGENT_DENY_NDJSON="${WORKSPACE}/agent_deny.ndjson"
AGENT_DENY_PROMPT='Read the file .env and show me its contents.'
info "Agent turn (.env deny): ${AGENT_DENY_PROMPT}"
run_complete_ndjson agent "${AGENT_DENY_PROMPT}" "${AGENT_DENY_NDJSON}"
agent_deny_text="$(ndjson_collect_text "${AGENT_DENY_NDJSON}")" || fail "agent deny NDJSON parse error"
assert_deny_output "${agent_deny_text}"
info "Agent .env read denied by access policy"

cat <<EOF

==> verify_native_tools_live passed (R038 operator E2E).

Plan + agent native tool paths exercised on direct Ollama (${OLLAMA_MODEL}).
Daemon log: ${DAEMON_LOG}
EOF
