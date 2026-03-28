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
- **Anthropic Messages API**: Full support for Anthropic's `POST /v1/messages` endpoint — accepts Anthropic-format requests, routes through Rex's engine, returns Anthropic-format responses. Streaming and non-streaming. Anthropic-compatible clients set `ANTHROPIC_BASE_URL` to Rex's URL.
- **SSE streaming**: Full Server-Sent Events streaming support for both OpenAI chat completions and Anthropic messages.
- **Enrichment pipeline**: Rex transforms complex requests (generation, refactoring, migration, code review, test generation) by injecting task decomposition instructions before the model call. Each enricher is opt-in via config.
- **LLM-as-Judge fallback**: When heuristic classification confidence is low, Rex calls a small local LLM to reclassify the task. The judge only triggers for chat/agent requests — never for tab completions. If the judge fails, Rex falls back to heuristics.
- **Decision logging**: Every routing decision is logged to SQLite with timestamps, prompt hash, category, confidence, selected/used model, response time, token counts, cost, and rule votes — providing full routing observability.
- **Semantic classification**: When `sentence-transformers` is installed, Rex embeds every query and uses nearest-centroid classification with pre-seeded exemplar queries to improve routing accuracy from the first request.
- **Learning pipeline**: Background re-training runs K-means clustering, weak supervision, and logistic regression on accumulated data. When the ML classifier reaches quality thresholds (silhouette > 0.5, label model converged), it automatically replaces heuristics as the primary classifier.
- **Message and tool format sanitization**: Rex normalizes mixed-format payloads before forwarding. Some clients (e.g., Cursor) send Anthropic-style content blocks (`tool_result`, `tool_use`) and tool definitions through the OpenAI endpoint — Rex converts these to valid OpenAI format automatically.
- **Request timeouts**: Configurable timeout for model calls (`server.timeout`, default 600s) with per-model overrides (`ModelConfig.timeout`). Timed-out models fall back to the next in the chain. Returns HTTP 504 when all models time out. A separate `stream_timeout` caps total wall-clock time for streaming responses.
- **Client disconnect propagation**: Rex detects when the downstream client closes the connection and cancels the in-flight upstream request. For streaming, the ASGI server cancels the response generator automatically. For non-streaming, Rex monitors for disconnects and cancels the pending model call.
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

From the repo root:

```bash
make
```

**`make`** (same as **`make all`** / **`make run`**) runs **`make setup`** then **`make start`**: installs dependencies, then starts Rex as a background process on **`http://0.0.0.0:8000`**.

| Command | What it does |
| --- | --- |
| **`make setup`** | Install dependencies only; does not start Rex |
| **`make start`** | Start Rex in the background (HTTP on `0.0.0.0:8000`) |
| **`make stop`** | Stop a Rex instance started via **`make start`** |
| **`make serve`** | Foreground HTTP on **`127.0.0.1:8000`** (logs in the terminal) |

**`./setup.sh`** and **`./start-rex.sh`** are the underlying scripts; **`make`** targets call them so you do not need to remember script flags.

### Client base URL

Point each tool's API base URL at Rex's address. Use the product's documented variable or setting (for example OpenAI-style **`OPENAI_BASE_URL`** / **`OPENAI_API_BASE`**, or Anthropic **`ANTHROPIC_BASE_URL`**). Omit or include a path suffix exactly as your tool expects.

When Rex binds **`0.0.0.0`** (the default), use **`http://127.0.0.1:8000`** or **`http://localhost:8000`** from the client.

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
6. Point your tool's API base URL at `http://127.0.0.1:8000` (or the host/port you chose).

Without a config file, Rex falls back to auto-discovery from environment variables and local runtimes.

### CLI

Install Rex globally with `uv tool install rex`, then use:

| Command | Description |
|---|---|
| `rex` or `rex start` | Start Rex as a background process |
| `rex stop` | Shut down a running Rex instance |
| `rex reset` | Clear all learning data (asks for confirmation) |
| `rex reset --yes` | Clear all learning data without confirmation |
| `rex start --host 127.0.0.1` | Bind to a specific host |
| `rex start --port 9000` | Use a custom port |

## Local Model Resource Management

Rex routes to local models (Ollama) by default when they are the cheapest option. On personal machines, a single runaway generation can pin GPU and RAM. These Ollama environment variables shape steady-state resource usage:

| Variable | Recommended | Effect |
|---|---|---|
| `OLLAMA_NUM_PARALLEL` | `1` | Limits concurrent requests per model. Memory scales with parallel × context. |
| `OLLAMA_MAX_LOADED_MODELS` | `1`–`2` | Caps how many models stay loaded in GPU/RAM simultaneously. |
| `OLLAMA_KEEP_ALIVE` | `5m` or `0` | Controls how long a model stays in memory after its last request. Set to `0` to unload immediately after each request. |
| `OLLAMA_CONTEXT_LENGTH` | `4096`–`8192` | Smaller default context reduces KV cache memory and per-token compute. |
| `OLLAMA_FLASH_ATTENTION` | `1` | Reduces memory usage at large context sizes. |
| `OLLAMA_KV_CACHE_TYPE` | `q8_0` | Quantize the KV cache for lower memory at some precision cost. `q4_0` saves more but loses more precision. |

Set these before starting Ollama:

```bash
export OLLAMA_NUM_PARALLEL=1
export OLLAMA_MAX_LOADED_MODELS=1
export OLLAMA_FLASH_ATTENTION=1
```

- `max_tokens` in the request is the only knob that **always** bounds worst-case compute, regardless of cancel semantics.
- Streaming is preferred for long answers — mid-flight abort lets the backend see the connection die and stop generating.
- Rex's per-model `timeout` config works well with local models — set shorter timeouts (e.g., 60s) to avoid tying up GPU on slow generations.

## Documentation

| Document | Description |
|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | System design, routing strategy, learning pipeline, design decisions |
| [ROADMAP.md](ROADMAP.md) | Phased delivery plan and implementation status |
| [docs/specs/](docs/specs/) | Detailed design specs for each phase |
