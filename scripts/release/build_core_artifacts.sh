#!/usr/bin/env bash
# Build REX core release artifacts locally (same binaries as CI cargo-dist).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

if ! command -v dist >/dev/null 2>&1; then
  echo "cargo-dist CLI 'dist' not found. Install with: cargo install cargo-dist --locked" >&2
  exit 127
fi

TAG="${1:-}"
if [[ -z "${TAG}" ]]; then
  VERSION="$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name=="rex") | .version')"
  TAG="v${VERSION}"
  echo "No tag argument; using workspace version tag: ${TAG}"
fi

echo "Running dist build for tag ${TAG}"
dist build --tag="${TAG}" "$@"
