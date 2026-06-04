#!/usr/bin/env bash
# Builtin rex-sidecar-stub: build, unit tests, UDS smoke (no live LLM).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
STUB_BIN="$TARGET_DIR/debug/rex-sidecar-stub"

echo "run_stub_sidecar_checks: building rex-sidecar-stub"
cargo build -p rex-sidecar-stub --locked
if [[ ! -x "$STUB_BIN" ]]; then
  echo "run_stub_sidecar_checks: missing binary at $STUB_BIN" >&2
  exit 1
fi

echo "run_stub_sidecar_checks: unit tests"
cargo test -p rex-sidecar-stub --locked

export REX_SIDECAR_BINARY="$STUB_BIN"
export REX_RUN_BUILTIN_SIDECAR_SMOKE=1
echo "run_stub_sidecar_checks: integration smoke"
cargo test -p rex-daemon --test stub_sidecar_smoke stub_ --locked -- --nocapture

echo "run_stub_sidecar_checks: OK"
