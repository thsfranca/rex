#!/usr/bin/env bash
# R017: Python rex-agent unit tests + integration smoke (no live LLM).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

if ! command -v python3 >/dev/null 2>&1; then
  echo "run_rex_agent_checks: python3 not found" >&2
  exit 1
fi

AGENT_DIR="$ROOT/sidecars/rex-agent"
REX_AGENT_ROOT="${REX_AGENT_ROOT:-$(mktemp -d "${TMPDIR:-/tmp}/rex-agent-ci.XXXXXX")}"
export REX_ROOT="$REX_AGENT_ROOT"
export REX_PROTO_SRC="$ROOT/proto"

echo "run_rex_agent_checks: REX_ROOT=$REX_ROOT"

cargo build -p rex --locked
cargo build -p rex-sidecar-stub --locked

python3 -m pip install -q grpcio-tools grpcio protobuf

REX_BIN="${CARGO_TARGET_DIR:-$ROOT/target}/debug/rex"
if [[ ! -x "$REX_BIN" ]]; then
  REX_BIN="$ROOT/target/debug/rex"
fi
"$REX_BIN" proto install

python3 -m pip install -q pytest
export PYTHONPATH="$AGENT_DIR/src:$("$REX_BIN" proto path):${PYTHONPATH:-}"

python3 -m pytest "$AGENT_DIR/tests" -q

export REX_AGENT_BINARY="${REX_AGENT_BINARY:-$AGENT_DIR/rex-agent}"
cargo test -p rex-daemon --test agent_scaffold_smoke agent_ --locked -- --nocapture

echo "run_rex_agent_checks: OK"
