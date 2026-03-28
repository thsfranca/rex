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
  warn "Self-signed certificate: install mkcert (https://github.com/FiloSottile/mkcert) and re-run this script for a locally trusted cert, or use cleartext HTTP (e.g. make start ARGS=--http) if your client cannot trust this cert."
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
echo -e "Start Rex from the repo with ${BOLD}make start${NC} (HTTPS) or ${BOLD}make serve${NC} (foreground). HTTP: ${BOLD}make start ARGS=--http${NC} or ${BOLD}make serve-http${NC}."
echo ""
echo -e "Point clients at Rex's URL (scheme and host must match your start command). See README.md: ${BOLD}Client base URL and TLS${NC}."
echo ""
if command -v mkcert &> /dev/null; then
  MKCERT_CA="$(mkcert -CAROOT)/rootCA.pem"
  echo -e "Many editors and CLI tools use Node.js for HTTPS. Trust the mkcert CA in that stack (the OS trust store alone is not always enough):"
  echo ""
  echo -e "  ${BOLD}export NODE_EXTRA_CA_CERTS=\"${MKCERT_CA}\"${NC}"
  echo ""
  echo -e "Python clients may use ${BOLD}SSL_CERT_FILE${NC} or ${BOLD}REQUESTS_CA_BUNDLE${NC} with the same PEM. Set variables in the environment that starts your client."
  echo ""
fi
echo -e "Stop: ${BOLD}make stop${NC}. Learning reset (with Rex running on HTTPS): ${BOLD}uv run rex reset --yes --tls${NC}"
echo ""
warn "For optional overrides (custom models, routing), see config.yaml.example."
echo ""
