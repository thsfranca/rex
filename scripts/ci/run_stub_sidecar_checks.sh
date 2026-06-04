#!/usr/bin/env bash
# rex-sidecar-stub tests (invoked from run_sidecar_verify.sh). No live LLM.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
STUB_BIN="${REX_SIDECAR_BINARY:-$TARGET_DIR/debug/rex-sidecar-stub}"
if [[ ! -x "$STUB_BIN" ]]; then
  echo "run_stub_sidecar_checks: missing binary at $STUB_BIN" >&2
  exit 1
fi

echo "::notice::rex-sidecar-stub unit tests"
cargo test -p rex-sidecar-stub --locked

export REX_SIDECAR_BINARY="$STUB_BIN"
export REX_RUN_BUILTIN_SIDECAR_SMOKE=1
echo "::notice::rex-sidecar-stub UDS smoke"
cargo test -p rex-daemon --test stub_sidecar_smoke stub_ --locked -- --nocapture
