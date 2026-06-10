#!/usr/bin/env bash
# Fast local reinstall for Rex development: put rex on PATH and install the VS Code/Cursor extension.
#
# Composes scripts/install-cli.sh and scripts/install-extension.sh. Does not start rex daemon.
# Full operator checklist: docs/EXTENSION_LOCAL_E2E.md
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALL_CLI="${ROOT_DIR}/scripts/install-cli.sh"
INSTALL_EXT="${ROOT_DIR}/scripts/install-extension.sh"

CLI_ONLY=false
EXTENSION_ONLY=false
SKIP_RUST=false
INSTALL_AGENT=false
CONFIGURE_SHELL=false
EXT_ARGS=()

print_usage() {
  cat <<'EOF'
Usage:
  ./scripts/reinstall-dev.sh [options] [-- extension-installer-flags...]

Rebuild and reinstall Rex for local testing:
  1. Install rex (and shims) to ~/.cargo/bin
  2. Install rex-sidecar-stub to ~/.cargo/bin (default sidecar in rex config init)
  3. Build and install the VS Code/Cursor extension VSIX

Options:
  --cli-only          Install Rust binaries only; skip the extension.
  --extension-only    Install the extension only; skip Rust binaries.
  --skip-rust         Skip Rust installs (same as --extension-only for binaries).
  --agent             Also: rex proto install + pip install -e sidecars/rex-agent
                      (product sidecar for plan/agent testing).
  --configure-shell   Pass --configure-shell to install-cli.sh (~/.cargo/bin in ~/.zshrc).
  -h, --help          Show this help.

Extension flags (pass after -- or as trailing args):
  --editor auto|cursor|vscode
  --verify            Run extension lint, typecheck, and tests before packaging.
  --no-reload         Skip Developer: Reload Window after VSIX install.
  --only-install      Reinstall an existing rex-vscode.vsix without rebuilding it.

Examples:
  ./scripts/reinstall-dev.sh
  ./scripts/reinstall-dev.sh --agent
  ./scripts/reinstall-dev.sh --extension-only --only-install
  ./scripts/reinstall-dev.sh -- --editor vscode --no-reload

After install:
  rex daemon          # or enable rex.daemonAutoStart in editor settings
  rex status
  REX: Open Chat      # Command Palette

If the status bar shows REX unavailable, set rex.cliPath to ~/.cargo/bin/rex
(see ./scripts/install-cli.sh --print-bin-path).
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
    --agent)
      INSTALL_AGENT=true
      shift
      ;;
    --configure-shell)
      CONFIGURE_SHELL=true
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
  if [[ "${CONFIGURE_SHELL}" == "true" ]]; then
    cli_flags+=(--configure-shell)
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

install_agent_sidecar() {
  if ! command -v pip >/dev/null 2>&1 && ! command -v pip3 >/dev/null 2>&1; then
    echo "pip is required for --agent but was not found in PATH." >&2
    exit 127
  fi
  local pip_cmd="pip"
  if ! command -v pip >/dev/null 2>&1; then
    pip_cmd="pip3"
  fi

  if ! command -v rex >/dev/null 2>&1 && [[ -x "${HOME}/.cargo/bin/rex" ]]; then
    export PATH="${HOME}/.cargo/bin:${PATH}"
  fi
  if ! command -v rex >/dev/null 2>&1; then
    echo "rex must be on PATH before --agent setup. Run without --extension-only first." >&2
    exit 127
  fi

  echo "=== Installing rex-agent Python sidecar ==="
  rex proto install
  "${pip_cmd}" install -e "${ROOT_DIR}/sidecars/rex-agent"
  echo "rex-agent installed. Set sidecars.active to agent in \$REX_ROOT/config.json"
  echo "or use rex.productAgentConfig (default true) with daemon auto-start."
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
  local bin_base="${HOME}/.cargo/bin"
  cat <<EOF

=== Reinstall complete ===

Binaries (add to editor settings if PATH is missing in the GUI):
  rex.cliPath          ${bin_base}/rex
  rex.daemonBinaryPath ${bin_base}/rex

Quick test:
  rex daemon            # separate terminal; listens on /tmp/rex.sock by default
  rex status
  # In editor: REX: Open Chat

Config (first time):
  rex config init
  # Edit ~/.rex/config.json — see docs/EXTENSION_LOCAL_E2E.md §3

Full checklist: ${ROOT_DIR}/docs/EXTENSION_LOCAL_E2E.md
EOF
}

if [[ "${EXTENSION_ONLY}" != "true" ]]; then
  install_rust_binaries
  if [[ "${INSTALL_AGENT}" == "true" ]]; then
    install_agent_sidecar
  fi
fi

if [[ "${CLI_ONLY}" != "true" ]]; then
  install_extension
fi

print_next_steps
