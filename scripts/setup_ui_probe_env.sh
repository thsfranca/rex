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

copy_if_missing "rex-ui-harness.toml.example" "rex-ui-harness.toml"
mkdir -p .cursor
copy_if_missing "cursor-permissions.rex-ui-harness.json.example" ".cursor/permissions.json"

HARNESS_DIR="$ROOT/crates/rex-ui-harness"
if [[ ! -d "$HARNESS_DIR/node_modules" ]]; then
  echo "Installing rex-ui-harness dependencies…"
  (cd "$HARNESS_DIR" && npm install && npm run build)
fi

echo
echo "Web UI probe env ready."
echo "  - rex-ui-harness.toml"
echo "  - .cursor/permissions.json"
echo "  - crates/rex-ui-harness (built)"
echo
echo "Add MCP server in Cursor (stdio):"
echo "  command: node"
echo "  args: [\"$HARNESS_DIR/dist/index.js\"]"
echo "  cwd: $ROOT"
echo
echo "Restart MCP after first config copy. Enable Run Mode in Cursor Settings > Agents."
