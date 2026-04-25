#!/usr/bin/env bash
# Build REX Rust binaries, install rex-cli/rex-daemon via install-cli.sh, then
# build and install the VS Code/Cursor extension (pass-through args go to install-extension.sh).
# Does not start rex-daemon; see docs/EXTENSION_LOCAL_E2E.md for daemon steps.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

cargo build --workspace

chmod +x "${ROOT_DIR}/scripts/install-cli.sh"
"${ROOT_DIR}/scripts/install-cli.sh"

cat <<EOF

--- REX extension dev ---

If the extension uses user-managed daemon mode (default), start the daemon in another terminal, for example:
  cargo run -p rex-daemon
Or enable rex.daemonAutoStart in editor settings (see docs/EXTENSION_LOCAL_E2E.md).

Full checklist: ${ROOT_DIR}/docs/EXTENSION_LOCAL_E2E.md

Installing VSIX (install-extension.sh)...
EOF

chmod +x "${ROOT_DIR}/scripts/install-extension.sh"
exec "${ROOT_DIR}/scripts/install-extension.sh" "$@"
