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

echo ""
info "Setup complete!"
echo ""
echo -e "Rex discovers models automatically from environment variables."
echo -e "Make sure at least one provider API key is set, for example:"
echo ""
echo "  export OPENAI_API_KEY=\"sk-...\""
echo "  export ANTHROPIC_API_KEY=\"sk-...\""
echo ""
echo -e "Start Rex with ${BOLD}make start${NC} (background) or ${BOLD}make serve${NC} (foreground)."
echo ""
echo -e "Point clients at ${BOLD}http://127.0.0.1:8000/v1${NC} (or the host/port you chose)."
echo ""
echo -e "Stop: ${BOLD}make stop${NC}. Learning reset: ${BOLD}uv run rex reset --yes${NC}"
echo ""
warn "For optional overrides (custom models, routing), see config.yaml.example."
echo ""
