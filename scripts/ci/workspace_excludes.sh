#!/usr/bin/env bash
# Shared cargo --exclude flags for macOS-only crates (Tauri).
set -euo pipefail

ci_workspace_excludes() {
  if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "--exclude" "rex-desktop"
  fi
}
