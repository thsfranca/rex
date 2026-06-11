#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO_BIN="${HOME}/.cargo/bin"
REX_BIN="${CARGO_BIN}/rex"
CONFIGURE_SHELL=true

PATH_MARKER="# rex install-cli: cargo bin PATH"

print_bin_paths() {
  cat <<EOF
Copy-paste when the editor host PATH omits ${CARGO_BIN} (see docs/EXTENSION_LOCAL_E2E.md):

  rex.cliPath          ${REX_BIN}
  rex.daemonBinaryPath ${REX_BIN}
EOF
}

print_usage() {
  cat <<'EOF'
Usage:
  ./scripts/install-cli.sh [--skip-shell-path] [--print-bin-path]

Options:
  --skip-shell-path   Do not update shell profiles with ~/.cargo/bin PATH.
  --print-bin-path    Print absolute paths to rex in ~/.cargo/bin, then exit.
  -h, --help          Show this help message.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-shell-path)
      CONFIGURE_SHELL=false
      shift
      ;;
    --configure-shell)
      CONFIGURE_SHELL=true
      shift
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
done

profile_has_cargo_bin_path() {
  local file="$1"
  [[ -f "${file}" ]] && grep -Fq "${PATH_MARKER}" "${file}"
}

append_cargo_bin_path() {
  local file="$1"
  touch "${file}"
  if profile_has_cargo_bin_path "${file}"; then
    echo "${file} already contains Rex Cargo bin PATH."
    return 0
  fi
  cat >>"${file}" <<EOF

${PATH_MARKER}
export PATH="\${HOME}/.cargo/bin:\${PATH}"
EOF
  echo "Updated ${file} with Cargo bin PATH."
}

configure_shell_path() {
  local shell_name
  shell_name="$(basename "${SHELL:-}")"
  case "${shell_name}" in
    zsh)
      append_cargo_bin_path "${HOME}/.zshrc"
      if [[ "$(uname -s)" == "Darwin" ]]; then
        append_cargo_bin_path "${HOME}/.zprofile"
      fi
      ;;
    bash)
      if [[ -f "${HOME}/.bashrc" || ! -f "${HOME}/.bash_profile" ]]; then
        append_cargo_bin_path "${HOME}/.bashrc"
      fi
      append_cargo_bin_path "${HOME}/.bash_profile"
      ;;
    *)
      echo "Note: unsupported shell '${shell_name}'; add ${CARGO_BIN} to PATH manually."
      ;;
  esac
}

ensure_session_path() {
  if [[ ":${PATH}:" != *":${CARGO_BIN}:"* ]]; then
    export PATH="${CARGO_BIN}:${PATH}"
  fi
}

bootstrap_rex_config() {
  if [[ ! -x "${REX_BIN}" ]]; then
    return 0
  fi
  echo "=== Initializing Rex config layout ==="
  "${REX_BIN}" config init
}

if ! command -v cargo > /dev/null 2>&1; then
  echo "Cargo is required but was not found in PATH." >&2
  exit 127
fi

echo "Installing rex (and compatibility shims) from ${ROOT_DIR}"

cargo install --path "${ROOT_DIR}/crates/rex" --force
cargo install --path "${ROOT_DIR}/crates/rex-daemon" --force
cargo install --path "${ROOT_DIR}/crates/rex-cli" --force

ensure_session_path

if [[ "${CONFIGURE_SHELL}" == "true" ]]; then
  configure_shell_path
  echo "Open a new terminal (or source your shell profile) for PATH changes to persist."
fi

bootstrap_rex_config

echo "Install complete."
echo "Primary command:"
echo "  rex"
echo "Compatibility shims (deprecated):"
echo "  rex-daemon  (use: rex daemon)"
echo "  rex-cli     (use: rex status | rex complete)"
if [[ ":${PATH}:" != *":${CARGO_BIN}:"* ]]; then
  echo "If PATH is not updated yet, run:"
  echo "  ${REX_BIN}"
fi
