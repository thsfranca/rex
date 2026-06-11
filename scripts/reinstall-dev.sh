#!/usr/bin/env bash
# Fast local reinstall for Rex development: put rex on PATH and install the VS Code/Cursor extension.
#
# Composes scripts/install-cli.sh and scripts/install-extension.sh. Does not start rex daemon.
# Full operator checklist: docs/EXTENSION_LOCAL_E2E.md
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALL_CLI="${ROOT_DIR}/scripts/install-cli.sh"
INSTALL_EXT="${ROOT_DIR}/scripts/install-extension.sh"
CARGO_BIN="${HOME}/.cargo/bin"
REX_BIN="${CARGO_BIN}/rex"

CLI_ONLY=false
EXTENSION_ONLY=false
SKIP_RUST=false
SKIP_SHELL_PATH=false
EXT_ARGS=()

print_usage() {
  cat <<'EOF'
Usage:
  ./scripts/reinstall-dev.sh [options] [-- extension-installer-flags...]

Rebuild and reinstall Rex for local testing:
  1. Install rex (and shims) to ~/.cargo/bin and configure shell PATH
  2. Install rex-sidecar-stub to ~/.cargo/bin
  3. Install rex-agent Python sidecar and run rex config init
  4. Build and install the VS Code/Cursor extension VSIX

Options:
  --cli-only          Install Rust binaries only; skip the extension.
  --extension-only    Install the extension only; skip Rust binaries.
  --skip-rust         Skip Rust installs (same as --extension-only for binaries).
  --skip-shell-path   Pass --skip-shell-path to install-cli.sh.
  --configure-shell   Deprecated alias; shell PATH is configured by default.
  -h, --help          Show this help.

Extension flags (pass after -- or as trailing args):
  --editor auto|cursor|vscode
  --verify            Run extension lint, typecheck, and tests before packaging.
  --no-reload         Skip Developer: Reload Window after VSIX install.
  --only-install      Reinstall an existing rex-vscode.vsix without rebuilding it.

Examples:
  ./scripts/reinstall-dev.sh
  ./scripts/reinstall-dev.sh --extension-only --only-install
  ./scripts/reinstall-dev.sh -- --editor vscode --no-reload

After install:
  rex daemon          # or enable rex.daemonAutoStart in editor settings
  rex status
  REX: Open Chat      # Command Palette

The extension auto-discovers ~/.cargo/bin/rex when rex.cliPath is unset.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --cli-only)
      CLI_ONLY=true
      shift
      ;;
    --extension-only|--skip-rust)
      EXTENSION_ONLY=true
      SKIP_RUST=true
      shift
      ;;
    --skip-shell-path)
      SKIP_SHELL_PATH=true
      shift
      ;;
    --configure-shell)
      shift
      ;;
    -h|--help)
      print_usage
      exit 0
      ;;
    --)
      shift
      EXT_ARGS+=("$@")
      break
      ;;
    --editor)
      EXT_ARGS+=("$1")
      shift
      if [[ $# -gt 0 ]]; then
        EXT_ARGS+=("$1")
        shift
      else
        echo "--editor requires a value (auto, cursor, or vscode)" >&2
        exit 2
      fi
      ;;
    --verify|--no-reload|--only-install|--ci)
      EXT_ARGS+=("$1")
      shift
      ;;
    --no-agent|--agent)
      echo "Option $1 is no longer supported; rex-agent install is always attempted (warns when pip is missing)." >&2
      exit 2
      ;;
    *)
      echo "Unknown option: $1" >&2
      print_usage >&2
      exit 2
      ;;
  esac
done

if [[ "${CLI_ONLY}" == "true" && "${EXTENSION_ONLY}" == "true" ]]; then
  echo "Cannot use --cli-only and --extension-only together." >&2
  exit 2
fi

install_rust_binaries() {
  local cli_flags=()
  if [[ "${SKIP_SHELL_PATH}" == "true" ]]; then
    cli_flags+=(--skip-shell-path)
  fi

  echo "=== Installing rex CLI to ~/.cargo/bin ==="
  chmod +x "${INSTALL_CLI}"
  if ((${#cli_flags[@]} > 0)); then
    "${INSTALL_CLI}" "${cli_flags[@]}"
  else
    "${INSTALL_CLI}"
  fi

  if ! command -v cargo >/dev/null 2>&1; then
    echo "Cargo is required but was not found in PATH." >&2
    exit 127
  fi

  echo "=== Installing rex-sidecar-stub to ~/.cargo/bin ==="
  cargo install --path "${ROOT_DIR}/crates/rex-sidecar-stub" --force
}

install_extension() {
  echo "=== Installing REX VS Code/Cursor extension ==="
  chmod +x "${INSTALL_EXT}"
  if ((${#EXT_ARGS[@]} > 0)); then
    "${INSTALL_EXT}" "${EXT_ARGS[@]}"
  else
    "${INSTALL_EXT}"
  fi
}

print_next_steps() {
  cat <<EOF

=== Reinstall complete ===

Quick test:
  rex daemon            # separate terminal; listens on /tmp/rex.sock by default
  rex status
  # In editor: REX: Open Chat (extension auto-finds ${REX_BIN})

Config:
  rex config show       # operator init defaults to rex-agent + mock web search

If rex-agent was skipped (no pip), run: rex proto install && pip install -e sidecars/rex-agent

If the status bar still shows unavailable, set rex.cliPath to ${REX_BIN}
(see ./scripts/install-cli.sh --print-bin-path).

Full checklist: ${ROOT_DIR}/docs/EXTENSION_LOCAL_E2E.md
EOF
}

if [[ "${EXTENSION_ONLY}" != "true" ]]; then
  install_rust_binaries
fi

if [[ "${CLI_ONLY}" != "true" ]]; then
  install_extension
fi

print_next_steps
