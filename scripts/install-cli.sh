#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONFIGURE_SHELL=false

print_bin_paths() {
  local base="${HOME}/.cargo/bin"
  cat <<EOF
Copy-paste when the editor host PATH omits ${base} (see docs/EXTENSION_LOCAL_E2E.md):

  rex.cliPath         ${base}/rex-cli
  rex.daemonBinaryPath ${base}/rex-daemon
EOF
}

print_usage() {
  cat <<'EOF'
Usage:
  ./scripts/install-cli.sh [--configure-shell] [--print-bin-path]

Options:
  --configure-shell   Add ~/.cargo/bin to ~/.zshrc if missing.
  --print-bin-path    Print absolute paths to rex-cli and rex-daemon in ~/.cargo/bin, then exit.
  -h, --help          Show this help message.
EOF
}

case "${1:-}" in
  "")
    ;;
  --configure-shell)
    CONFIGURE_SHELL=true
    ;;
  --print-bin-path)
    print_bin_paths
    exit 0
    ;;
  -h|--help)
    print_usage
    exit 0
    ;;
  *)
    echo "Unknown option: ${1}" >&2
    print_usage >&2
    exit 2
    ;;
esac

ensure_path_in_zshrc() {
  local zshrc="${HOME}/.zshrc"
  local line='export PATH="$HOME/.cargo/bin:$PATH"'

  touch "${zshrc}"
  if ! grep -Fq 'export PATH="$HOME/.cargo/bin:$PATH"' "${zshrc}"; then
    echo "${line}" >> "${zshrc}"
    echo "Updated ${zshrc} with Cargo bin PATH."
  else
    echo "${zshrc} already contains Cargo bin PATH."
  fi
}

if ! command -v cargo > /dev/null 2>&1; then
  echo "Cargo is required but was not found in PATH." >&2
  exit 127
fi

echo "Installing rex-daemon and rex-cli from ${ROOT_DIR}"

cargo install --path "${ROOT_DIR}/crates/rex-daemon" --force
cargo install --path "${ROOT_DIR}/crates/rex-cli" --force

if [[ ":${PATH}:" != *":${HOME}/.cargo/bin:"* ]]; then
  cat <<'EOF'
Note: ~/.cargo/bin is not in PATH for this shell.
Add it to your shell profile to run commands directly:
  export PATH="$HOME/.cargo/bin:$PATH"
Or re-run this script with:
  ./scripts/install-cli.sh --configure-shell
EOF
fi

if [[ "${CONFIGURE_SHELL}" == "true" ]]; then
  ensure_path_in_zshrc
  echo "Run 'source ~/.zshrc' or open a new terminal."
fi

echo "Install complete."
echo "Commands available:"
echo "  rex-daemon"
echo "  rex-cli"
echo "If PATH is not updated yet, run:"
echo "  ${HOME}/.cargo/bin/rex-daemon"
echo "  ${HOME}/.cargo/bin/rex-cli"
