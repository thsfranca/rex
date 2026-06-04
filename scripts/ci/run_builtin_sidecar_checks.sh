#!/usr/bin/env bash
# CI gate for all builtin sidecars (rex-sidecar-stub + rex-agent).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

MANIFEST="$ROOT/scripts/ci/builtin_sidecars.txt"
if [[ ! -f "$MANIFEST" ]]; then
  echo "run_builtin_sidecar_checks: missing manifest $MANIFEST" >&2
  exit 1
fi

echo "run_builtin_sidecar_checks: manifest"
grep -v '^#' "$MANIFEST" | grep -v '^[[:space:]]*$' || true

"$ROOT/scripts/ci/run_stub_sidecar_checks.sh"
"$ROOT/scripts/ci/run_rex_agent_checks.sh"

echo "run_builtin_sidecar_checks: OK"
