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
echo -e "Then start Rex with:"
echo ""
echo "  uv run uvicorn app.main:app --host 0.0.0.0 --port 8000"
echo ""
echo -e "Point your AI coding tool's base URL to ${BOLD}http://localhost:8000/v1${NC}"
echo ""
warn "For optional overrides (custom models, routing), see config.yaml.example."
