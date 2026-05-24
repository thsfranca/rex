#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export PYTHONPATH="${ROOT_DIR}/sidecars/rex-agent:${PYTHONPATH:-}"
cd "${ROOT_DIR}/sidecars/rex-agent"
python3 -m pytest -q tests/ 2>/dev/null || {
  python3 -m pip install -q pytest
  python3 -m pytest -q tests/
}
