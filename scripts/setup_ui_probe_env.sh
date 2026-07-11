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
else
  (cd "$HARNESS_DIR" && npm run build)
fi

echo "Building rex-web assets…"
(cd "$ROOT/apps/rex-web" && npm install && npm run build)

echo "Installing Electron shell (apps/rex-desktop)…"
(cd "$ROOT/apps/rex-desktop" && npm install)

if [[ "$(uname -s)" == "Darwin" ]]; then
  echo "Building rex CLI…"
  cargo build -p rex
fi

echo
echo "Web UI probe env ready."
echo "  - rex-ui-harness.toml (launch.mode=desktop on macOS)"
echo "  - fixtures/ui_probe/rex_root (mock inference + direct harness)"
echo "  - crates/rex-ui-harness (built)"
echo "  - apps/rex-desktop (Electron shell)"
echo
echo "Desktop compositor proof: ./scripts/ci/run_electron_compositor_proof.sh"
echo "Add MCP server in Cursor (stdio):"
echo "  command: node"
echo "  args: [\"$HARNESS_DIR/dist/index.js\"]"
echo "  cwd: $ROOT"
echo
echo "Restart MCP after first config copy. Enable Run Mode in Cursor Settings > Agents."
echo "Bare rex launches Electron loading apps/rex-web (daemon IPC bridge completes in W127)."
