#!/usr/bin/env bash
# Build the REX VS Code/Cursor extension from source, package a VSIX, install it
# into the editor CLI profile, and try to reload the most recently focused window.
#
# Requires: Node.js 20+, npm, and either the Cursor or VS Code shell CLI on PATH
# (or use --editor / REX_EXTENSION_EDITOR / macOS app paths below).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXT_DIR="${ROOT_DIR}/extensions/rex-vscode"
VSIX_NAME="rex-vscode.vsix"
VSIX_PATH="${EXT_DIR}/${VSIX_NAME}"

EDITOR_MODE="auto"
VERIFY=false
NO_RELOAD=false
ONLY_INSTALL=false
USE_NPM_CI=false

MAC_CURSOR_CLI="/Applications/Cursor.app/Contents/Resources/app/bin/cursor"
MAC_VSCODE_CLI="/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code"

print_usage() {
  cat <<'EOF'
Usage:
  ./scripts/install-extension.sh [options]

Builds extensions/rex-vscode, produces rex-vscode.vsix, installs it with the
editor CLI (cursor or code), then runs Developer: Reload Window on the last
active window when supported.

Options:
  --editor auto|cursor|vscode   Which CLI to use (default: auto).
  --verify                      Run lint, typecheck, and tests before packaging.
  --ci                          Use npm ci instead of npm install (stricter).
  --no-reload                   Skip workbench.action.reloadWindow after install.
  --only-install                Skip npm/build; install an existing VSIX only.
  -h, --help                    Show this help.

Environment:
  REX_EXTENSION_EDITOR   Full path to the editor CLI (overrides --editor).

Examples:
  ./scripts/install-extension.sh
  ./scripts/install-extension.sh --editor cursor
  REX_EXTENSION_EDITOR=/path/to/cursor ./scripts/install-extension.sh --verify
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --editor)
      EDITOR_MODE="${2:-}"
      shift 2
      ;;
    --verify)
      VERIFY=true
      shift
      ;;
    --ci)
      USE_NPM_CI=true
      shift
      ;;
    --no-reload)
      NO_RELOAD=true
      shift
      ;;
    --only-install)
      ONLY_INSTALL=true
      shift
      ;;
    -h|--help)
      print_usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      print_usage >&2
      exit 2
      ;;
  esac
done

if ! command -v node >/dev/null 2>&1; then
  echo "Node.js is required but was not found in PATH." >&2
  exit 127
fi

if ! command -v npm >/dev/null 2>&1; then
  echo "npm is required but was not found in PATH." >&2
  exit 127
fi

resolve_editor_cli() {
  if [[ -n "${REX_EXTENSION_EDITOR:-}" ]]; then
    if [[ -f "${REX_EXTENSION_EDITOR}" && -x "${REX_EXTENSION_EDITOR}" ]]; then
      echo "${REX_EXTENSION_EDITOR}"
      return 0
    fi
    if command -v "${REX_EXTENSION_EDITOR}" >/dev/null 2>&1; then
      command -v "${REX_EXTENSION_EDITOR}"
      return 0
    fi
    echo "REX_EXTENSION_EDITOR not found or not executable: ${REX_EXTENSION_EDITOR}" >&2
    exit 127
  fi

  local want="${EDITOR_MODE}"
  if [[ "${want}" == "auto" ]]; then
    # Prefer the host that spawned this terminal when both CLIs exist on PATH.
    case "${TERM_PROGRAM:-}" in
      Cursor)
        if command -v cursor >/dev/null 2>&1; then
          command -v cursor
          return 0
        fi
        if [[ -x "${MAC_CURSOR_CLI}" ]]; then
          echo "${MAC_CURSOR_CLI}"
          return 0
        fi
        ;;
      vscode)
        if command -v code >/dev/null 2>&1; then
          command -v code
          return 0
        fi
        if [[ -x "${MAC_VSCODE_CLI}" ]]; then
          echo "${MAC_VSCODE_CLI}"
          return 0
        fi
        ;;
    esac
    if command -v cursor >/dev/null 2>&1; then
      command -v cursor
      return
    fi
    if [[ -x "${MAC_CURSOR_CLI}" ]]; then
      echo "${MAC_CURSOR_CLI}"
      return
    fi
    if command -v code >/dev/null 2>&1; then
      command -v code
      return
    fi
    if [[ -x "${MAC_VSCODE_CLI}" ]]; then
      echo "${MAC_VSCODE_CLI}"
      return
    fi
    cat <<'EOF' >&2
Could not find Cursor or VS Code on PATH.

Fix one of:
  - Cursor: Command Palette → "Shell Command: Install 'cursor' command in PATH"
  - VS Code: Command Palette → "Shell Command: Install 'code' command in PATH"
  - Or set REX_EXTENSION_EDITOR to the full path of the CLI binary.
EOF
    exit 127
  fi

  if [[ "${want}" == "cursor" ]]; then
    if command -v cursor >/dev/null 2>&1; then
      command -v cursor
      return
    fi
    if [[ -x "${MAC_CURSOR_CLI}" ]]; then
      echo "${MAC_CURSOR_CLI}"
      return
    fi
    echo "cursor CLI not found. Install the shell command or set REX_EXTENSION_EDITOR." >&2
    exit 127
  fi

  if [[ "${want}" == "vscode" ]]; then
    if command -v code >/dev/null 2>&1; then
      command -v code
      return
    fi
    if [[ -x "${MAC_VSCODE_CLI}" ]]; then
      echo "${MAC_VSCODE_CLI}"
      return
    fi
    echo "code CLI not found. Install the shell command or set REX_EXTENSION_EDITOR." >&2
    exit 127
  fi

  echo "Invalid --editor value: ${want} (use auto, cursor, or vscode)" >&2
  exit 2
}

EDITOR_CLI="$(resolve_editor_cli)"

if [[ "${ONLY_INSTALL}" != "true" ]]; then
  echo "Using editor CLI: ${EDITOR_CLI}"
  echo "Installing dependencies in ${EXT_DIR}"
  cd "${EXT_DIR}"
  if [[ "${USE_NPM_CI}" == "true" ]]; then
    npm ci --no-audit --no-fund
  else
    npm install --no-audit --no-fund
  fi

  if [[ "${VERIFY}" == "true" ]]; then
    echo "Running lint, typecheck, and tests..."
    npm run lint
    npm run typecheck
    npm test
  fi

  echo "Building and packaging VSIX..."
  npm run package
  cd "${ROOT_DIR}"
else
  echo "Skipping build (--only-install)."
  if [[ ! -f "${VSIX_PATH}" ]]; then
    echo "Missing VSIX at ${VSIX_PATH}. Run without --only-install first." >&2
    exit 1
  fi
fi

echo "Installing ${VSIX_NAME} into the editor profile used by:"
echo "  ${EDITOR_CLI}"
"${EDITOR_CLI}" --install-extension "${VSIX_PATH}" --force

if [[ "${NO_RELOAD}" == "true" ]]; then
  echo "Skipping reload (--no-reload)."
else
  echo "Requesting window reload (Developer: Reload Window) on the last active window..."
  if "${EDITOR_CLI}" --command workbench.action.reloadWindow; then
    echo "Reload command sent. If nothing happens, run Developer: Reload Window from the Command Palette."
  else
    echo "Reload command failed or is unsupported for this CLI build." >&2
    echo "Manually run: Developer: Reload Window (or restart the editor)." >&2
  fi
fi

echo "Done. Extension id: rex.rex-vscode (publisher rex, package rex-vscode)."
