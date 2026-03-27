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

if [ ! -f config.yaml ]; then
    info "No config.yaml found. Creating one now."
    echo ""
    echo -e "Rex needs at least one model backend configured."
    echo -e "Enter your details below (press Enter to accept defaults)."
    echo ""

    read -rp "Model name [openai/gpt-4o]: " MODEL_NAME
    MODEL_NAME="${MODEL_NAME:-openai/gpt-4o}"

    read -rp "Provider [openai]: " PROVIDER
    PROVIDER="${PROVIDER:-openai}"

    read -rp "API key (leave empty for local models): " API_KEY

    read -rp "API base URL (leave empty for provider default): " API_BASE

    IS_LOCAL="false"
    CONTEXT_WINDOW=128000
    MAX_LATENCY=2000
    COST_IN=0.005
    COST_OUT=0.015

    if [ -z "$API_KEY" ]; then
        IS_LOCAL="true"
        COST_IN=0
        COST_OUT=0
        MAX_LATENCY=100
        CONTEXT_WINDOW=8192
    fi

    CONFIG="server:
  host: \"0.0.0.0\"
  port: 8000

models:
  - name: \"${MODEL_NAME}\"
    provider: \"${PROVIDER}\"
    context_window: ${CONTEXT_WINDOW}
    cost_per_1k_input: ${COST_IN}
    cost_per_1k_output: ${COST_OUT}
    strengths:
      - completion
      - debugging
      - refactoring
      - generation
      - general
    max_latency_ms: ${MAX_LATENCY}
    is_local: ${IS_LOCAL}"

    if [ -n "$API_KEY" ]; then
        CONFIG="${CONFIG}
    api_key: \"${API_KEY}\""
    fi

    if [ -n "$API_BASE" ]; then
        CONFIG="${CONFIG}
    api_base: \"${API_BASE}\""
    fi

    CONFIG="${CONFIG}

routing:
  completion_model: \"${MODEL_NAME}\"
  default_model: \"${MODEL_NAME}\""

    echo "$CONFIG" > config.yaml
    info "Created config.yaml with ${MODEL_NAME}"
    echo ""
    warn "You can add more models later by editing config.yaml."
    warn "See config.yaml.example for a multi-model setup."
else
    info "Using existing config.yaml"
fi

echo ""
info "Setup complete! Start Rex with:"
echo ""
echo "  uv run uvicorn app.main:app --host 0.0.0.0 --port 8000"
echo ""
echo -e "Then point your AI coding tool's base URL to ${BOLD}http://localhost:8000/v1${NC}"
