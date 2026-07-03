#!/usr/bin/env bash
# Read-only operator install preflight — prints a dependency summary table.
# Use --strict to exit non-zero when a required tool is missing or unsupported.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LIB_DIR="${ROOT_DIR}/scripts/lib"
STRICT=false

print_usage() {
  cat <<'EOF'
Usage:
  ./scripts/install-preflight.sh [--strict]

Checks Rust, protoc, and Python (rex-agent) availability.
Does not modify the system. With --strict, exits non-zero when a check fails.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --strict)
      STRICT=true
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

# shellcheck source=lib/python_sidecar.sh
source "${LIB_DIR}/python_sidecar.sh"

overall_ok=true

check_row() {
  local name="$1"
  local status="$2"
  local detail="$3"
  printf ' %-12s %-8s %s\n' "${name}" "${status}" "${detail}"
  if [[ "${status}" != "ok" ]]; then
    overall_ok=false
  fi
}

echo "=== Rex operator install preflight ==="

# Rust
if command -v cargo >/dev/null 2>&1 && command -v rustc >/dev/null 2>&1; then
  check_row "cargo" "ok" "$(cargo --version 2>/dev/null | head -n1)"
else
  check_row "cargo" "missing" "install rustup — https://rustup.rs"
fi

# protoc
if command -v protoc >/dev/null 2>&1; then
  check_row "protoc" "ok" "$(protoc --version 2>/dev/null | head -n1)"
else
  check_row "protoc" "missing" "brew install protobuf"
fi

# Python for rex-agent
if python_sidecar_preflight; then
  check_row "python" "ok" "${REX_PYTHON_DETAIL}"
else
  if [[ "${REX_PYTHON_STATUS}" == "too_old" ]]; then
    check_row "python" "too_old" "${REX_PYTHON_DETAIL} (need >= 3.10; brew install python@3.12)"
  else
    check_row "python" "missing" "${REX_PYTHON_DETAIL} (need >= 3.10)"
  fi
fi

echo ""
if [[ "${overall_ok}" == "true" ]]; then
  echo "Preflight: pass"
  exit 0
fi

echo "Preflight: issues found — fix rows above before ./scripts/install-cli.sh"
if [[ "${STRICT}" == "true" ]]; then
  exit 1
fi
exit 0
