#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

error() {
  echo "start-rex: $*" >&2
  exit 1
}

HOST="127.0.0.1"
PORT="8000"
USE_TLS=1

while [[ $# -gt 0 ]]; do
  case "$1" in
    --http | --no-tls)
      USE_TLS=0
      HOST="0.0.0.0"
      shift
      ;;
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
      echo "Usage: $0 [--http|--no-tls] [--host ADDR] [--port N]"
      echo ""
      echo "  Default: HTTPS on 127.0.0.1:8000 with ~/.rex/tls/localhost.pem and localhost-key.pem"
      echo "           (create them with make setup or ./setup.sh if missing)."
      echo "  --http:  cleartext HTTP; default bind 0.0.0.0 (override with --host)."
      exit 0
      ;;
    *)
      error "unknown option: $1 (try --help)"
      ;;
  esac
done

command -v uv >/dev/null 2>&1 || error "uv not found. Install: https://docs.astral.sh/uv/"

if [[ "$USE_TLS" -eq 1 ]]; then
  REX_TLS_DIR="${HOME}/.rex/tls"
  CERT_PEM="${REX_TLS_DIR}/localhost.pem"
  KEY_PEM="${REX_TLS_DIR}/localhost-key.pem"
  [[ -f "$CERT_PEM" && -f "$KEY_PEM" ]] || error "missing TLS files. Run: make setup"
  exec uv run rex start --host "$HOST" --port "$PORT" --certfile "$CERT_PEM" --keyfile "$KEY_PEM"
else
  exec uv run rex start --host "$HOST" --port "$PORT"
fi
