#!/usr/bin/env bash
# Operator E2E: R039 live Ollama smoke (ask NDJSON + brokered read/policy).
#
# Opt-in only — export REX_OLLAMA_LIVE=1. Not wired to default PR CI (RC-10 / RC-S6).
# Mirrors broker read/policy assertions in crates/rex-daemon/tests/mvp_product_path.rs.
# Explicit out: plan-mode native tool loop (see scripts/verify_native_tools_live.sh / R038).
#
# Defaults: http://127.0.0.1:11434/v1; model qwen2.5-coder:7b or llama3.2 (first available).
set -euo pipefail

if [[ "${REX_OLLAMA_LIVE:-}" != "1" ]]; then
  echo "verify_ollama_live: opt-in required — export REX_OLLAMA_LIVE=1 (not default PR CI; see docs/ECONOMICS_VALIDATION.md)." >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

TARGET_DIR="${CARGO_TARGET_DIR:-${ROOT_DIR}/target}"
REX_BIN="${TARGET_DIR}/debug/rex"
AGENT_LAUNCHER="${ROOT_DIR}/sidecars/rex-agent/rex-agent"

READ_MARKER="broker-read-ok"
OLLAMA_BASE="http://127.0.0.1:11434/v1"
OLLAMA_HOST="127.0.0.1:11434"
OLLAMA_MODEL=""
OLLAMA_MODEL_CANDIDATES=("qwen2.5-coder:7b" "llama3.2")
READINESS_TIMEOUT_SECS=90
TURN_TIMEOUT_SECS=120

DAEMON_PID=""
REX_ROOT=""
WORKSPACE=""
DAEMON_SOCKET=""
SIDECAR_SOCKET=""
DAEMON_LOG=""

fail() {
  echo "verify_ollama_live: FAIL — $*" >&2
  exit 1
}

info() {
  echo "==> $*"
}

cleanup() {
  if [[ -n "${DAEMON_PID}" ]]; then
    kill "${DAEMON_PID}" 2>/dev/null || true
    wait "${DAEMON_PID}" 2>/dev/null || true
    DAEMON_PID=""
  fi
  if [[ -n "${DAEMON_SOCKET}" ]]; then
    rm -f "${DAEMON_SOCKET}"
  fi
  if [[ -n "${SIDECAR_SOCKET}" ]]; then
    rm -f "${SIDECAR_SOCKET}"
  fi
  if [[ -n "${WORKSPACE}" && -d "${WORKSPACE}" ]]; then
    rm -rf "${WORKSPACE}"
    WORKSPACE=""
  fi
  if [[ -n "${REX_ROOT}" && -d "${REX_ROOT}" ]]; then
    rm -rf "${REX_ROOT}"
    REX_ROOT=""
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
saw_done = False
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
        elif event == "done":
            saw_done = True
            if obj.get("error"):
                errors.append(str(obj.get("error")))
        elif event == "error":
            errors.append(obj.get("message") or obj.get("code") or "unknown error")
if errors:
    print("\n".join(errors), file=sys.stderr)
    sys.exit(2)
if not saw_done:
    print("missing terminal done event", file=sys.stderr)
    sys.exit(3)
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
    if env REX_ROOT="${REX_ROOT}" "${REX_BIN}" status >/dev/null 2>&1; then
      return 0
    fi
    if ! kill -0 "${DAEMON_PID}" 2>/dev/null; then
      fail "rex daemon exited before ready (see ${DAEMON_LOG})"
    fi
    sleep 0.25
  done
  fail "daemon not ready within ${READINESS_TIMEOUT_SECS}s (see ${DAEMON_LOG})"
}

assert_deny_output() {
  local text="$1"
  local lower
  lower="$(printf '%s' "${text}" | tr '[:upper:]' '[:lower:]')"
  if [[ "${lower}" != *"protected_path"* && "${text}" != *"fs.read error"* ]]; then
    fail "expected .env read deny (protected_path or fs.read error), got: ${text}"
  fi
}

pick_ollama_model() {
  local tags candidate
  if ! tags="$(curl -sf "http://${OLLAMA_HOST}/api/tags")"; then
    fail "Ollama not reachable at http://${OLLAMA_HOST} (start: ollama serve)"
  fi
  for candidate in "${OLLAMA_MODEL_CANDIDATES[@]}"; do
    if grep -q "\"name\":\"${candidate}\"" <<<"${tags}"; then
      OLLAMA_MODEL="${candidate}"
      return 0
    fi
  done
  fail "no pinned model in ollama list; pull one of: ${OLLAMA_MODEL_CANDIDATES[*]}"
}

info "Building rex CLI"
if ! cargo build -p rex --locked >/dev/null; then
  fail "cargo build -p rex failed"
fi
[[ -x "${REX_BIN}" ]] || fail "rex binary missing at ${REX_BIN}"

info "Probing Ollama at ${OLLAMA_BASE}"
pick_ollama_model
info "Using Ollama model ${OLLAMA_MODEL}"

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

REX_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/rex-ollama-live-root.XXXXXX")"
WORKSPACE="$(mktemp -d "${TMPDIR:-/tmp}/rex-ollama-live-ws.XXXXXX")"
DAEMON_SOCKET="${WORKSPACE}/rex-daemon.sock"
SIDECAR_SOCKET="${WORKSPACE}/rex-sidecar.sock"
DAEMON_LOG="${WORKSPACE}/daemon.log"

info "Preparing workspace fixture under ${WORKSPACE}/workspace"
mkdir -p "${WORKSPACE}/workspace"
printf '%s\n' "${READ_MARKER}" >"${WORKSPACE}/workspace/hello.txt"
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

info "Starting rex daemon (log: ${DAEMON_LOG})"
env REX_ROOT="${REX_ROOT}" "${REX_BIN}" daemon >>"${DAEMON_LOG}" 2>&1 &
DAEMON_PID=$!

info "Waiting for daemon + sidecar ready (timeout ${READINESS_TIMEOUT_SECS}s)"
wait_for_daemon_ready

ASK_NDJSON="${WORKSPACE}/ask.ndjson"
ASK_PROMPT="Reply with one short greeting sentence."
info "Ask NDJSON smoke: ${ASK_PROMPT}"
run_complete_ndjson ask "${ASK_PROMPT}" "${ASK_NDJSON}"
ask_text="$(ndjson_collect_text "${ASK_NDJSON}")" || fail "ask NDJSON parse error (see ${ASK_NDJSON})"
if [[ -z "${ask_text}" ]]; then
  fail "ask NDJSON smoke produced no streamed text (see ${ASK_NDJSON})"
fi
info "Ask NDJSON smoke streamed text and terminal done"

AGENT_READ_NDJSON="${WORKSPACE}/agent_read.ndjson"
AGENT_READ_PROMPT="inspect __rex_read:hello.txt"
info "Agent brokered read (allowed): ${AGENT_READ_PROMPT}"
run_complete_ndjson agent "${AGENT_READ_PROMPT}" "${AGENT_READ_NDJSON}"
agent_read_text="$(ndjson_collect_text "${AGENT_READ_NDJSON}")" || fail "agent read NDJSON parse error"
if [[ "${agent_read_text}" != *"${READ_MARKER}"* ]]; then
  fail "brokered read missing marker '${READ_MARKER}'; got: ${agent_read_text}"
fi
info "Brokered __rex_read:hello.txt returned fixture marker"

AGENT_DENY_NDJSON="${WORKSPACE}/agent_deny.ndjson"
AGENT_DENY_PROMPT="inspect __rex_read:.env"
info "Agent brokered read (.env deny): ${AGENT_DENY_PROMPT}"
run_complete_ndjson agent "${AGENT_DENY_PROMPT}" "${AGENT_DENY_NDJSON}"
agent_deny_text="$(ndjson_collect_text "${AGENT_DENY_NDJSON}")" || fail "agent deny NDJSON parse error"
assert_deny_output "${agent_deny_text}"
info "Brokered __rex_read:.env denied by access policy"

cat <<EOF

==> verify_ollama_live passed (R039 operator E2E).

Ask NDJSON + brokered read/policy exercised on direct Ollama (${OLLAMA_MODEL}).
Daemon log: ${DAEMON_LOG}
EOF
