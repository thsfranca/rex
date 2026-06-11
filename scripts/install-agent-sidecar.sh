#!/usr/bin/env bash
# Install rex-agent Python sidecar and proto stubs (operator default).
# Exits 0 when pip is missing (warn only); exits non-zero on pip install failure.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! command -v rex >/dev/null 2>&1; then
  echo "rex must be on PATH before installing rex-agent." >&2
  exit 127
fi

if ! command -v pip >/dev/null 2>&1 && ! command -v pip3 >/dev/null 2>&1; then
  echo "WARNING: pip not found — skipped rex-agent install." >&2
  echo "Install Python 3.10+ and pip, then run: rex proto install && pip install -e sidecars/rex-agent" >&2
  exit 0
fi

pip_cmd="pip"
if ! command -v pip >/dev/null 2>&1; then
  pip_cmd="pip3"
fi

echo "=== Installing rex-agent Python sidecar ==="
rex proto install
"${pip_cmd}" install -e "${ROOT_DIR}/sidecars/rex-agent"
echo "rex-agent installed."
