#!/usr/bin/env bash
# Electron compositor proof (ADR 0043 / W126): chrome + fullscreen WebGL ≥5s.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
APP_DIR="${ROOT}/apps/rex-desktop-electron"

if [ "$(uname -s)" != "Darwin" ]; then
  echo "Electron compositor proof requires macOS (v1 host)." >&2
  exit 1
fi

if ! command -v node >/dev/null 2>&1; then
  echo "node not found." >&2
  exit 1
fi

if ! command -v npm >/dev/null 2>&1; then
  echo "npm not found." >&2
  exit 1
fi

cd "${APP_DIR}"

if [ ! -d node_modules ]; then
  echo "::group::npm install (rex-desktop-electron)"
  npm install
  echo "::endgroup::"
fi

echo "::group::compositor-proof (co-visibility)"
npm run compositor-proof
echo "::endgroup::"

echo "::group::compositor-proof (bury expect-fail)"
npm run compositor-proof:bury-expect-fail
echo "::endgroup::"

echo "electron compositor proof: ok"
