# Rex

> This project only exists because I'm too lazy to pick the best model myself.

An OpenAI-compatible proxy that sits between AI-powered coding tools and multiple model backends (local + cloud), routing each request to the cheapest model that fits the task.

- Works with any tool that supports a custom OpenAI API base URL (Cursor, Claude Code, Continue, Aider, etc.).
- Each user runs their own Rex instance locally.

## Features

- **Zero-config model discovery**: Rex detects available providers from environment variables and local runtimes (Ollama), queries their APIs for available models, and enriches each model with metadata (cost, context window) from LiteLLM — no config file needed.
- **Task-aware routing**: Rex classifies each request (debugging, refactoring, code review, etc.) and picks the cheapest model that meets the task's requirements (context window, capabilities). Tasks with no special needs stay on the primary (cheapest) model.
- **Fallback chains**: If the selected model fails, Rex tries the next model in cost order.
- **SSE streaming**: Full Server-Sent Events streaming support for chat completions.
- **Enrichment pipeline**: Rex transforms complex requests (generation, refactoring, migration, code review, test generation) by injecting task decomposition instructions before the model call. Each enricher is opt-in via config.
- **Transparent passthrough**: Unknown endpoints are forwarded to the primary model's backend — Rex never blocks an endpoint it doesn't handle.
- **Optional YAML config**: Add custom models, override routing, or enable enrichments via `config.yaml`.

## How It Works

1. On startup, Rex **discovers** available models from environment variables (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, etc.), local runtimes (Ollama), and provider APIs — enriches each with metadata (cost, context window, capabilities) and merges with any `config.yaml` overrides.
2. Rex sorts models by cost (local first, then cheapest cloud) and selects the **primary model**.
3. For each request, the **classifier** identifies the task type (debugging, refactoring, code review, etc.) and the **router** picks the cheapest model that meets the task's requirements. If the primary already qualifies, it stays on primary.
4. If the selected model fails, the **fallback chain** tries remaining models in cost order.

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
  config.py              # Pydantic settings model + optional YAML loader
  adapters/
    base.py              # Client adapter interface and NormalizedRequest
    default.py           # Default adapter (generic feature detection)
    registry.py          # Selects adapter by User-Agent header
  discovery/
    providers.py         # Detects available providers from env vars
    models.py            # Queries provider APIs for available models
    metadata.py          # Enriches models with LiteLLM metadata
    registry_builder.py  # Orchestrates discovery and builds the model registry
  enrichment/
    context.py           # EnrichmentContext dataclass
    pipeline.py          # Enricher protocol and pipeline runner
    task_decomposition.py # Task decomposition enricher
  router/
    categories.py        # Task categories and routing requirements
    classifier.py        # Heuristic task classifier (keyword + structural)
    detector.py          # Feature detection (completion vs. chat)
    engine.py            # Routing engine (task-aware selection + fallback)
    registry.py          # Model registry (lookups, cost sorting, filtering)
  proxy/
    handler.py           # OpenAI-compatible request handler
    streaming.py         # SSE streaming response logic
config.yaml.example     # Example configuration (optional)
pyproject.toml           # Project dependencies (uv)
tests/                   # pytest test suite
```

## Getting Started

### Quick Setup

```bash
./setup.sh
```

The script installs dependencies and shows how to start Rex.

### Manual Setup

1. Install [uv](https://docs.astral.sh/uv/):
   ```bash
   curl -LsSf https://astral.sh/uv/install.sh | sh
   ```
2. Install dependencies:
   ```bash
   uv sync
   ```
3. Set at least one provider API key:
   ```bash
   export OPENAI_API_KEY="sk-..."
   ```
4. Start the Rex proxy:
   ```bash
   uv run uvicorn app.main:app --host 0.0.0.0 --port 8000
   ```
5. Point your AI coding tool's base URL to `http://localhost:8000/v1`.

Rex discovers available models automatically from environment variables and local runtimes. No config file needed. See `config.yaml.example` for optional overrides.

## Documentation

| Document | Description |
|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | System design, routing strategy, learning pipeline, design decisions |
| [ROADMAP.md](ROADMAP.md) | Phased delivery plan and implementation status |
| [docs/specs/](docs/specs/) | Detailed design specs for each phase |
