# Rex

> This project only exists because I'm too lazy to pick the best model myself.

An OpenAI- and Anthropic-compatible proxy that sits between AI-powered coding tools and multiple model backends (local + cloud), routing each request to the cheapest model that fits the task.

- Works with any tool that supports a custom OpenAI or Anthropic API base URL (Cursor, Claude Code, Continue, Aider, etc.).
- Each user runs their own Rex instance locally.

## Features

- **Config-first model registry**: Define your models and providers in `~/.rex/config.yaml`. Rex supplements the config with auto-discovered providers (environment variables, Ollama) and enriches each model with metadata (cost, context window) from LiteLLM.
- **Remote provider discovery**: Configure remote LiteLLM proxies or custom API endpoints as providers. Rex probes their model list endpoints at startup and merges discovered models with local and manually configured models. Auth via direct API key or environment variable reference.
- **Task-aware routing**: Rex classifies each request (debugging, refactoring, code review, etc.) and picks the cheapest model that meets the task's requirements (context window, capabilities). Tasks with no special needs stay on the primary (cheapest) model.
- **Fallback chains**: If the selected model fails, Rex tries the next model in cost order.
- **Anthropic Messages API**: Full support for Anthropic's `POST /v1/messages` endpoint — accepts Anthropic-format requests, routes through Rex's engine, returns Anthropic-format responses. Streaming and non-streaming. Claude Code and other Anthropic clients connect by setting `ANTHROPIC_BASE_URL=http://localhost:8000`.
- **SSE streaming**: Full Server-Sent Events streaming support for both OpenAI chat completions and Anthropic messages.
- **Enrichment pipeline**: Rex transforms complex requests (generation, refactoring, migration, code review, test generation) by injecting task decomposition instructions before the model call. Each enricher is opt-in via config.
- **LLM-as-Judge fallback**: When heuristic classification confidence is low, Rex calls a small local LLM to reclassify the task. The judge only triggers for chat/agent requests — never for tab completions. If the judge fails, Rex falls back to heuristics.
- **Decision logging**: Every routing decision is logged to SQLite with timestamps, prompt hash, category, confidence, selected/used model, response time, token counts, cost, and rule votes — providing full routing observability.
- **Semantic classification**: When `sentence-transformers` is installed, Rex embeds every query and uses nearest-centroid classification with pre-seeded exemplar queries to improve routing accuracy from the first request.
- **Learning pipeline**: Background re-training runs K-means clustering, weak supervision, and logistic regression on accumulated data. When the ML classifier reaches quality thresholds (silhouette > 0.5, label model converged), it automatically replaces heuristics as the primary classifier.
- **Message and tool format sanitization**: Rex normalizes mixed-format payloads before forwarding. Some clients (e.g., Cursor) send Anthropic-style content blocks (`tool_result`, `tool_use`) and tool definitions through the OpenAI endpoint — Rex converts these to valid OpenAI format automatically.
- **Transparent passthrough**: Unknown endpoints are forwarded to the primary model's backend — Rex never blocks an endpoint it doesn't handle.
- **Auto-discovery**: When no config exists, Rex falls back to scanning environment variables and local runtimes to find models automatically.

## How It Works

1. On startup, Rex loads models and providers from `~/.rex/config.yaml`, probes configured remote providers for available models, then supplements with auto-discovered providers (environment variables, Ollama) — enriches each with metadata (cost, context window, capabilities) from LiteLLM. Config providers override auto-discovered providers with the same prefix.
2. Rex sorts models by cost (local first, then cheapest cloud) and selects the **primary model**.
3. For each request, the **classifier** identifies the task type (debugging, refactoring, code review, etc.) and the **router** picks the cheapest model that meets the task's requirements. If the primary already qualifies, it stays on primary. When heuristic confidence is low, the **centroid classifier** uses semantic similarity, then the **LLM judge** reclassifies using a small local model.
4. For complex tasks (generation, refactoring, migration, code review, test generation), the **enrichment pipeline** injects task decomposition instructions into the request before the model call. Each enricher is opt-in via config.
5. If the selected model fails, the **fallback chain** tries remaining models in cost order.
6. Every routing decision is **logged** to SQLite with response metrics and embeddings for future learning.

## API

| Endpoint | Description |
|---|---|
| `POST /v1/chat/completions` | OpenAI chat completions (streaming and non-streaming) |
| `POST /v1/messages` | Anthropic Messages API (streaming and non-streaming) |
| `POST /v1/completions` | Legacy completions |
| `GET /v1/models` | Lists all discovered models |
| `GET /health` | Proxy status |
| `POST /v1/reset` | Clears all learning data and trained models, restores fresh state |
| `/{path}` | Transparent passthrough to the primary model's backend |

## Tech Stack

| Component | Technology |
|---|---|
| Language | Python 3.12+ |
| API framework | FastAPI |
| Model backends | LiteLLM (100+ providers) |
| HTTP client | httpx (passthrough requests) |
| Configuration | YAML (`~/.rex/config.yaml`) |

## Project Structure

```
app/
  main.py                # FastAPI app entry point
  cli.py                 # CLI entry point (rex start/stop/reset)
  config.py              # Pydantic settings model + optional YAML loader
  utils.py               # Shared utility functions
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
  learning/
    embeddings.py        # Sentence transformer embedding service
    centroids.py         # Centroid classifier with synthetic exemplars
    clustering.py        # K-means clustering + silhouette score
    labeling.py          # Weak supervision label model
    trainer.py           # ML classifier training pipeline
    scheduler.py         # Re-training scheduler
  logging/
    models.py            # DecisionRecord dataclass
    repository.py        # DecisionRepository protocol
    sqlite.py            # SQLite implementation of decision repository
  router/
    categories.py        # Task categories and routing requirements
    classifier.py        # Heuristic task classifier (keyword + structural)
    detector.py          # Feature detection (completion vs. chat)
    engine.py            # Routing engine (task-aware selection + fallback)
    llm_judge.py         # LLM-as-Judge fallback classifier
    ml_classifier.py     # Trained ML classifier (logistic regression)
    registry.py          # Model registry (lookups, cost sorting, filtering)
  proxy/
    anthropic.py         # Anthropic Messages API translator (request/response/streaming)
    handler.py           # Request handlers (OpenAI + Anthropic)
    message_sanitizer.py # Converts mixed-format messages (Anthropic content blocks in OpenAI requests) to valid OpenAI format
    streaming.py         # OpenAI SSE streaming response logic
config.yaml.example     # Example configuration (optional)
pyproject.toml           # Project dependencies (uv)
tests/                   # pytest test suite
```

## Getting Started

### Quick Setup

```bash
./setup.sh
```

The script installs dependencies, creates TLS material under `~/.rex/tls/` (uses [mkcert](https://github.com/FiloSottile/mkcert) when installed, otherwise OpenSSL self-signed), and starts Rex with **`rex start`** over **HTTPS** so tools like Claude Code can negotiate **HTTP/2**. Pass **`./setup.sh --no-start`** to only sync and prepare certs without launching Rex.

After setup, you can start Rex anytime with **`./start-rex.sh`** (HTTPS using `~/.rex/tls/`, same as above). Use **`./start-rex.sh --http`** for cleartext HTTP. **`./start-rex.sh --help`** lists options.

From the repo root, **`make`** or **`make all`** runs **`./setup.sh --no-start`** then **`./start-rex.sh`**. **`make setup`** and **`make start`** run each step alone; **`make start ARGS=--http`** passes flags through. **`make stop`** runs **`uv run rex stop`**. **`make serve`** runs Hypercorn in the foreground on **HTTPS** at **`https://127.0.0.1:8000`** using **`~/.rex/tls/`** (same as **`./start-rex.sh`**, so **HTTP/2** via TLS); run **`make setup`** first if certs are missing. **`make serve-http`** runs cleartext HTTP on **`0.0.0.0:8000`** when you do not need TLS.

### Manual Setup

1. Install [uv](https://docs.astral.sh/uv/):
   ```bash
   curl -LsSf https://astral.sh/uv/install.sh | sh
   ```
2. Install dependencies:
   ```bash
   uv sync
   ```
3. Create your config (copy the example as a starting point):
   ```bash
   mkdir -p ~/.rex
   cp config.yaml.example ~/.rex/config.yaml
   ```
4. Edit `~/.rex/config.yaml` to add your models and API keys.
5. Start Rex:
   ```bash
   rex start
   ```
   `rex start` runs Hypercorn (HTTP/2 capable: cleartext uses h2c upgrade; add `--certfile` and `--keyfile` for HTTPS and HTTP/2 via TLS). For `rex reset` after a TLS start, pass `--tls`.
6. Point your AI coding tool's base URL to `http://localhost:8000/v1` (or `https://...` if you use TLS).

Without a config file, Rex falls back to auto-discovery from environment variables and local runtimes.

### CLI

Install Rex globally with `uv tool install rex`, then use:

| Command | Description |
|---|---|
| `rex` or `rex start` | Start Rex as a background process |
| `rex stop` | Shut down a running Rex instance |
| `rex reset` | Clear all learning data (asks for confirmation) |
| `rex reset --yes` | Clear all learning data without confirmation |
| `rex reset --tls` | Same as reset, but use `https://` (after `rex start` with TLS) |
| `rex start --certfile … --keyfile …` | Serve HTTPS (HTTP/2 via ALPN with compatible clients) |

Rex runs as a background process that any AI tool can connect to via `http://localhost:8000/v1` unless you configure TLS.

## Documentation

| Document | Description |
|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | System design, routing strategy, learning pipeline, design decisions |
| [ROADMAP.md](ROADMAP.md) | Phased delivery plan and implementation status |
| [docs/specs/](docs/specs/) | Detailed design specs for each phase |
