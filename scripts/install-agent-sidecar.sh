#!/usr/bin/env bash
# Install rex-agent Python sidecar and proto stubs (operator default).
# Uses $REX_ROOT/venv (default ~/.rex/venv) — not system site-packages.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LIB_DIR="${ROOT_DIR}/scripts/lib"
# shellcheck source=lib/python_sidecar.sh
source "${LIB_DIR}/python_sidecar.sh"

if ! command -v rex >/dev/null 2>&1; then
  echo "rex must be on PATH before installing rex-agent." >&2
  echo "Run ./scripts/install-cli.sh first." >&2
  exit 127
fi

echo "=== Installing rex-agent Python sidecar ==="
rex proto install

if ! python_sidecar_install "${ROOT_DIR}"; then
  echo "rex-agent install failed — see messages above." >&2
  exit 1
fi

echo "rex-agent installed (venv: $(python_sidecar__rex_root)/venv, wrapper: ${HOME}/.cargo/bin/rex-agent)."
