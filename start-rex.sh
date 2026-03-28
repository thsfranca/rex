#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

error() {
  echo "start-rex: $*" >&2
  exit 1
}

HOST="0.0.0.0"
PORT="8000"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --port)
      [[ $# -ge 2 ]] || error "--port needs a value"
      PORT="$2"
      shift 2
      ;;
    --host)
      [[ $# -ge 2 ]] || error "--host needs a value"
      HOST="$2"
      shift 2
      ;;
    -h | --help)
      echo "Usage: $0 [--host ADDR] [--port N]"
      echo ""
      echo "  Start Rex on HTTP. Default: 0.0.0.0:8000"
      exit 0
      ;;
    *)
      error "unknown option: $1 (try --help)"
      ;;
  esac
done

command -v uv >/dev/null 2>&1 || error "uv not found. Install: https://docs.astral.sh/uv/"

exec uv run rex start --host "$HOST" --port "$PORT"
