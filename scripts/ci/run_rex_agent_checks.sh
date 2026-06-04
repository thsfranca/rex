#!/usr/bin/env bash
# rex-agent tests (invoked from run_sidecar_verify.sh). No live LLM.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

PYTHON="python3"
if command -v python3.11 >/dev/null 2>&1; then
  PYTHON="python3.11"
elif command -v python3.10 >/dev/null 2>&1; then
  PYTHON="python3.10"
fi

AGENT_DIR="$ROOT/sidecars/rex-agent"
if [[ -z "${PYTHONPATH:-}" ]]; then
  echo "run_rex_agent_checks: PYTHONPATH must be set by run_sidecar_verify.sh" >&2
  exit 1
fi

echo "::notice::rex-agent pytest"
"$PYTHON" -m pytest "$AGENT_DIR/tests" -q

export REX_AGENT_BINARY="${REX_AGENT_BINARY:-$AGENT_DIR/rex-agent}"
export REX_RUN_BUILTIN_SIDECAR_SMOKE="${REX_RUN_BUILTIN_SIDECAR_SMOKE:-1}"
export REX_RUN_AGENT_SMOKE=1
echo "::notice::rex-agent UDS smoke"
cargo test -p rex-daemon --test agent_scaffold_smoke agent_ --locked -- --nocapture
