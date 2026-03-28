#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BOLD='\033[1m'
NC='\033[0m'

info()  { echo -e "${GREEN}==>${NC} ${BOLD}$1${NC}"; }
warn()  { echo -e "${YELLOW}==>${NC} $1"; }
error() { echo -e "${RED}==>${NC} $1"; exit 1; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

START_REX=1
for arg in "$@"; do
  case "$arg" in
    --no-start) START_REX=0 ;;
  esac
done

if ! command -v uv &> /dev/null; then
    error "uv is not installed. Install it with: curl -LsSf https://astral.sh/uv/install.sh | sh"
fi

info "Installing dependencies..."
uv sync

REX_TLS_DIR="${HOME}/.rex/tls"
CERT_PEM="${REX_TLS_DIR}/localhost.pem"
KEY_PEM="${REX_TLS_DIR}/localhost-key.pem"

mkdir -p "$REX_TLS_DIR"

if [[ -f "$CERT_PEM" && -f "$KEY_PEM" ]]; then
  info "Using existing TLS files in ${REX_TLS_DIR}"
elif command -v mkcert &> /dev/null; then
  info "Generating localhost TLS certificate with mkcert..."
  mkcert -cert-file "$CERT_PEM" -key-file "$KEY_PEM" localhost 127.0.0.1 ::1
else
  info "Generating self-signed TLS certificate with openssl..."
  OPENSSL_CONF="$(mktemp)"
  cat > "$OPENSSL_CONF" <<'OPENSSL_EOF'
[req]
distinguished_name = dn
x509_extensions = v3_req
prompt = no
[dn]
CN = localhost
[v3_req]
subjectAltName = @alt_names
[alt_names]
DNS.1 = localhost
IP.1 = 127.0.0.1
OPENSSL_EOF
  openssl req -x509 -newkey rsa:2048 -sha256 -days 825 -nodes \
    -keyout "$KEY_PEM" -out "$CERT_PEM" -config "$OPENSSL_CONF"
  rm -f "$OPENSSL_CONF"
  warn "Self-signed certificate: install mkcert (https://github.com/FiloSottile/mkcert) and re-run this script for a locally trusted cert so Claude Code and browsers accept https://localhost:8000."
fi

echo ""
info "Setup complete!"
echo ""
echo -e "Rex discovers models automatically from environment variables."
echo -e "Make sure at least one provider API key is set, for example:"
echo ""
echo "  export OPENAI_API_KEY=\"sk-...\""
echo "  export ANTHROPIC_API_KEY=\"sk-...\""
echo ""
echo -e "Rex is started with ${BOLD}HTTPS${NC} so clients (e.g. Claude Code) can use ${BOLD}HTTP/2 over TLS${NC}."
echo ""
echo -e "Point Claude Code (or other Anthropic clients) at:"
echo ""
echo -e "  ${BOLD}export ANTHROPIC_BASE_URL=\"https://localhost:8000\"${NC}"
echo ""
echo -e "To start Rex manually later (same TLS paths):"
echo ""
echo "  uv run rex start --host 127.0.0.1 --port 8000 --certfile ${CERT_PEM} --keyfile ${KEY_PEM}"
echo ""
echo -e "Learning reset over HTTPS: ${BOLD}uv run rex reset --yes --tls${NC}"
echo ""
warn "For optional overrides (custom models, routing), see config.yaml.example."
echo ""

if [[ "$START_REX" -eq 1 ]]; then
  info "Starting Rex with HTTPS..."
  if uv run rex start --host 127.0.0.1 --port 8000 --certfile "$CERT_PEM" --keyfile "$KEY_PEM"; then
    :
  else
    warn "Rex did not start (already running, or health check failed). Stop with: uv run rex stop"
  fi
fi
