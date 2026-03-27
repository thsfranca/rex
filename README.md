# Rex

> This project only exists because I'm too lazy to pick the best model myself.

An OpenAI-compatible proxy that sits between AI-powered coding tools and multiple model backends (local + cloud), routing all requests to the cheapest available model.

- Works with any tool that supports a custom OpenAI API base URL (Cursor, Claude Code, Continue, Aider, etc.).
- Each user runs their own Rex instance locally.

## Features

- **Zero-config model discovery**: Rex detects available providers from environment variables and local runtimes (Ollama), queries their APIs for available models, and enriches each model with metadata (cost, context window) from LiteLLM — no config file needed. Models from `config.yaml` are merged into the registry (and override discovered models with the same name).
- **Cost-first routing**: Rex auto-selects the cheapest model as the primary (local models first, then cheapest cloud). All requests go to the primary model.
- **Fallback chains**: If the primary model fails, Rex tries the next model in cost order.
- **SSE streaming**: Full Server-Sent Events streaming support for chat completions.
- **Transparent passthrough**: Unknown endpoints are forwarded to the primary model's backend — Rex never blocks an endpoint it doesn't handle.
- **Optional YAML config**: Add custom models or override the auto-selected primary model via `config.yaml`.

## How It Works

1. On startup, Rex **discovers** available models from environment variables (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, etc.), local runtimes (Ollama), provider APIs, and models listed in `config.yaml`.
2. Rex sorts models by cost (local first, then cheapest cloud) and selects the **primary model**.
3. The **proxy** forwards incoming requests to the primary model via LiteLLM and streams the response back.
4. If the primary fails, the **fallback chain** tries remaining models in cost order.

## API

| Endpoint | Description |
|---|---|
| `POST /v1/chat/completions` | Chat completions (streaming and non-streaming) |
| `POST /v1/completions` | Legacy completions |
| `GET /v1/models` | Lists all discovered models |
| `GET /health` | Proxy status |
| `/{path}` | Transparent passthrough to the primary model's backend |

## Tech Stack

| Component | Technology |
|---|---|
| Language | Python 3.12+ |
| API framework | FastAPI |
| Model backends | LiteLLM (100+ providers) |
| HTTP client | httpx (passthrough requests) |
| Configuration | Optional YAML overrides |

## Project Structure

```
app/
  main.py                # FastAPI app entry point
  cli.py                 # CLI entry point (`rex` command)
  config.py              # Pydantic settings model + optional YAML loader
  discovery/
    providers.py         # Detects available providers from env vars
    models.py            # Queries provider APIs for available models
    metadata.py          # Enriches models with LiteLLM metadata
    registry_builder.py  # Orchestrates discovery and builds the model registry
  router/
    detector.py          # Feature detection (completion vs. chat)
    engine.py            # Routing engine (primary selection + fallback)
    registry.py          # Model registry (lookups, cost sorting)
  proxy/
    handler.py           # OpenAI-compatible request handler
    streaming.py         # SSE streaming response logic
config.yaml.example     # Example configuration (optional)
pyproject.toml           # Project dependencies (uv)
tests/                   # pytest test suite
```

## Getting Started

1. Install [uv](https://docs.astral.sh/uv/):
   ```bash
   curl -LsSf https://astral.sh/uv/install.sh | sh
   ```
2. Clone and install:
   ```bash
   git clone https://github.com/thsfranca/rex.git && cd rex
   uv sync
   ```
3. Set at least one provider API key:
   ```bash
   export OPENAI_API_KEY="sk-..."
   ```
4. Start Rex:
   ```bash
   uv run rex
   ```
5. Point your AI coding tool's base URL to `http://localhost:8000/v1`.

Rex discovers available models automatically from environment variables and local runtimes. No config file needed. See `config.yaml.example` for optional overrides.

### CLI options

```
rex --host 127.0.0.1 --port 9000 --config path/to/config.yaml
```

## Documentation

| Document | Description |
|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | System design, routing strategy, learning pipeline, design decisions |
| [ROADMAP.md](ROADMAP.md) | Phased delivery plan and implementation status |
| [docs/specs/](docs/specs/) | Detailed design specs for each phase |
