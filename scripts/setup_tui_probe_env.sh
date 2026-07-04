#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

copy_if_missing() {
  local src="$1"
  local dst="$2"
  if [[ -f "$dst" ]]; then
    echo "keep: $dst (already exists)"
  else
    cp "$src" "$dst"
    echo "created: $dst"
  fi
}

copy_if_missing "tuiwright.toml.example" "tuiwright.toml"
mkdir -p .cursor
copy_if_missing "cursor-permissions.tui-probe.json.example" ".cursor/permissions.json"

echo
echo "TUI probe env ready."
echo "  - tuiwright.toml"
echo "  - .cursor/permissions.json (MCP allowlist for tuiwright; reloads automatically)"
echo
echo "Restart the tuiwright MCP server in Cursor after the first tuiwright.toml copy."
echo "Run Mode must be enabled (Auto-review or Allowlist) in Cursor Settings > Agents."
