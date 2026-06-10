#!/usr/bin/env bash
# Back-compat wrapper — prefer ./scripts/reinstall-dev.sh
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
chmod +x "${ROOT_DIR}/scripts/reinstall-dev.sh"
exec "${ROOT_DIR}/scripts/reinstall-dev.sh" "$@"
