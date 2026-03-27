# Phase 0 — Proxy + Basic Routing

For the full delivery plan, see [ROADMAP.md](../../ROADMAP.md). For system design and routing strategy, see [ARCHITECTURE.md](../../ARCHITECTURE.md).

---

## Goal

- Build an OpenAI-compatible proxy that routes requests across multiple model backends.
- Detect whether a request is a tab completion or a chat/agent interaction.
- Auto-select the cheapest model as the primary (prefer local over cloud at equal cost).
- Allow the user to override the primary model via optional config.
- Fall back to the next available model if the primary fails.
- Validate end-to-end integration with AI coding tools (streaming, request formats, connectivity).

---

## Routing Criteria

Rex selects models based on measurable properties available at startup — no manual tagging required.

Phase 0 uses two properties:

| Property | Source | How Rex uses it |
|---|---|---|
| `cost_per_1k_input` | Config or LiteLLM `get_model_info()` | Cheaper models rank higher |
| `is_local` | Config or auto-detected (Ollama probe) | Local models rank above cloud at equal cost |

Later phases will add:

| Property | Source | How Rex uses it |
|---|---|---|
| Context window | LiteLLM `get_model_info()` | Route large-context tasks (refactoring, code review) to models with adequate context |
| `supports_reasoning` | LiteLLM capability flags | Route debugging/optimization to reasoning-capable models |
| `supports_function_calling` | LiteLLM capability flags | Route agent/tool-use tasks to models that support it |

Rex never uses a manually curated "strengths" list. All routing signals come from cost, context window, and capability flags that LiteLLM provides automatically for known models.

---

## Config Schema

Rex works without a config file. When present, `config.yaml` acts as an override mechanism.

```yaml
server:
  host: "0.0.0.0"
  port: 8000

models:
  - name: "openai/gpt-4o-mini"
    api_key: "sk-..."

  - name: "ollama/llama3"
    api_base: "http://localhost:11434"
    is_local: true

routing:
  primary_model: "ollama/llama3"
```

### Server

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `server.host` | string | no | `"0.0.0.0"` | Address Rex listens on |
| `server.port` | integer | no | `8000` | Port Rex listens on |

### Models

Each entry in the `models` list describes a model backend:

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `name` | string | yes | — | LiteLLM model identifier (e.g., `openai/gpt-4o-mini`, `ollama/llama3`) |
| `api_key` | string | no | `null` | API key for this model's provider |
| `api_base` | string | no | `null` | Override the provider's default API URL |
| `cost_per_1k_input` | float | no | `0.0` | Cost per 1,000 input tokens (0 for local models) |
| `is_local` | boolean | no | `false` | Whether the model runs locally |

### Routing

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `routing.primary_model` | string | no | `null` | Override which model Rex uses as primary. When omitted, Rex auto-selects the cheapest model (local first, then lowest cloud cost). |

- Rex validates the config at startup using a Pydantic `Settings` model.
- If the config file is missing, Rex discovers models from environment variables and local runtimes.
- If the config file exists but is invalid, Rex fails with Pydantic validation errors.
- If `routing.primary_model` is set but not found in the `models` list, Rex fails with `"Primary model '{name}' not found in registry"`.
- LiteLLM infers the provider from the model identifier prefix (e.g., `openai/` → OpenAI, `ollama/` → Ollama).

---

## Model Registry

- At startup, Rex runs model discovery (env vars, Ollama probe, provider APIs) and merges the results with any models from `config.yaml`.
- Manual config entries override discovered models with the same name.
- Each model becomes a `ModelConfig` Pydantic model enriched with cost from `litellm.get_model_info()`.
- The registry provides lookups by name (`get_by_name`), full list (`get_all`), cost-sorted list (`sorted_by_cost`), and name list (`names`).
- `sorted_by_cost` ranks local models first, then by ascending `cost_per_1k_input`.

---

## Feature Detection

Rex detects whether a request is a **tab completion** or a **chat/agent** interaction based on request signals:

| Signal | Completion | Chat/Agent |
|---|---|---|
| Conversation length | Single turn (1 user message) | Multi-turn (2+ messages) |
| Prompt length | Short (< 200 tokens) | Longer |
| `max_tokens` | Low (< 500) | Higher or unset |
| `temperature` | Low (0-0.2) | Higher or unset |

- The detector examines the request and returns a feature type: `completion` or `chat`.
- Phase 0 uses a simple scoring approach: each signal contributes a weighted score, and the highest-scoring feature type wins.
- If the score is too close to call, the detector defaults to `chat` (the safer option).
- Phase 0 detects the feature type but routes all requests to the primary model. Per-feature routing (separate completion vs. chat models) is a future enhancement.

### Routing Logic

```
primary = cheapest model (local-first), or config override

selected = primary  # Phase 0: all requests go to primary
```

---

## Fallback Chain

If the selected model fails (connection error, timeout, rate limit), Rex tries the next available model:

1. Try the primary model.
2. On failure, try remaining models in cost order (local first, then cheapest cloud).
3. If all models fail, return the error from the last attempt to the client.

- Rex logs each fallback attempt (which model failed, why, which model it fell back to).
- The fallback chain adds no overhead on the happy path — it only activates on failure.

---

## Endpoints

### POST /v1/chat/completions

- Accepts the full OpenAI chat completions request body.
- Runs feature detection on the request (logged, but routing uses primary model in Phase 0).
- Passes all other parameters through to LiteLLM (`messages`, `temperature`, `max_tokens`, `top_p`, `stop`, `stream`, etc.).
- Uses the client's `Authorization: Bearer <key>` as the API key if the model config has no `api_key` set.

**Non-streaming** (`stream: false` or omitted):

- Calls `await litellm.acompletion(model=selected_model, **params)`.
- Returns the response as JSON with `Content-Type: application/json`.

**Streaming** (`stream: true`):

- Calls `await litellm.acompletion(model=selected_model, stream=True, **params)`.
- Returns a `StreamingResponse` with `Content-Type: text/event-stream`.
- Each chunk: `data: {json}\n\n`
- Final signal: `data: [DONE]\n\n`

### POST /v1/completions

- Same pattern as chat completions but calls `await litellm.acompletion()`.
- Supports both streaming and non-streaming modes.

### GET /v1/models

- Returns all models from the registry in OpenAI's models list format:

```json
{
  "object": "list",
  "data": [
    {
      "id": "openai/gpt-4o-mini",
      "object": "model",
      "created": 0,
      "owned_by": "rex"
    },
    {
      "id": "ollama/llama3",
      "object": "model",
      "created": 0,
      "owned_by": "rex"
    }
  ]
}
```

### GET /health

- Returns proxy status:

```json
{
  "status": "ok"
}
```

### Transparent Passthrough (catch-all)

- Catches any request to a path not handled above.
- Forwards the raw request (method, path, headers, query params, body) to the primary model's `api_base` via `httpx.AsyncClient`.
- Returns the upstream response as-is (status code, headers, body).
- If no `api_base` is configured for the primary model, returns `501 Not Implemented`.

---

## SSE Streaming

The streaming response uses an async generator that yields OpenAI-compatible SSE events:

```python
async def stream_completion(response) -> AsyncIterator[str]:
    async for chunk in response:
        yield f"data: {chunk.model_dump_json()}\n\n"
    yield "data: [DONE]\n\n"
```

- FastAPI's `StreamingResponse` wraps this generator with `media_type="text/event-stream"`.
- LiteLLM's `acompletion(stream=True)` returns an async iterable of chunk objects.
- Each chunk follows the OpenAI `ChatCompletionChunk` schema (contains `id`, `choices` with `delta`, `finish_reason`).

---

## Error Handling

Rex follows the graceful degradation strategy from [ARCHITECTURE.md](../../ARCHITECTURE.md).

### LiteLLM Errors

| LiteLLM Exception | HTTP Status | Error Type |
|---|---|---|
| `AuthenticationError` | 401 | `authentication_error` |
| `RateLimitError` | 429 | `rate_limit_error` |
| `ServiceUnavailableError` | 503 | `service_unavailable` |
| `Timeout` | 504 | `timeout_error` |
| `BadRequestError` | 400 | `invalid_request_error` |
| Any other exception | 502 | `proxy_error` |

- Before returning an error to the client, Rex attempts the fallback chain.
- Rex only returns an error if all models in the fallback chain fail.
- All error responses use the OpenAI error format:

```json
{
  "error": {
    "message": "All model backends failed. Last error: ...",
    "type": "proxy_error",
    "code": 502
  }
}
```

### Request Validation

- FastAPI and Pydantic handle request validation automatically.
- Invalid request bodies return `422 Unprocessable Entity` with field-level error details.

### Startup Failures

- Missing config file → Rex discovers models from environment variables and local runtimes.
- Invalid config values → Rex fails with Pydantic validation errors.
- `routing.primary_model` not found in the registry → Rex fails with `"Primary model '{name}' not found in registry"`.
- Empty model registry (no providers detected and no models in config) → Rex exits with a message listing the supported environment variables.

---

## Project Files

Phase 0 creates only the files needed for a working proxy with basic routing:

```
app/
  main.py              # FastAPI app, lifespan, endpoint definitions
  cli.py               # CLI entry point (`rex` command)
  config.py            # Pydantic Settings model, YAML loader
  discovery/
    providers.py       # Detects available providers from env vars
    models.py          # Queries provider APIs for available models
    metadata.py        # Enriches models with LiteLLM metadata
    registry_builder.py # Orchestrates discovery and builds the model registry
  proxy/
    handler.py         # Completion request handling via LiteLLM
    streaming.py       # SSE async generator
  router/
    registry.py        # Model registry (lookups, cost sorting)
    detector.py        # Feature detection (completion vs. chat)
    engine.py          # Routing engine (primary selection + fallback)
config.yaml.example   # Example configuration (optional)
pyproject.toml         # Project dependencies (uv)
tests/                 # pytest test suite
```

### main.py

- Defines the FastAPI app with a lifespan context manager.
- Loads the config (optional), runs model discovery, and builds the model registry at startup via `build_registry()`.
- Registers all endpoint routes.
- The catch-all route handles transparent passthrough.

### config.py

- `ModelConfig` Pydantic model for each model entry (name required, everything else optional with defaults).
- `RoutingConfig` Pydantic model for routing settings (optional `primary_model`).
- `Settings` Pydantic model for the full config schema (all sections optional with defaults).
- `load_config(path) -> Settings | None` — reads YAML, returns `None` if file missing.

### proxy/handler.py

- `handle_chat_completion(body, engine, authorization) -> Response` — runs feature detection, selects model, calls LiteLLM, returns JSON or streaming.
- `handle_text_completion(body, engine, authorization) -> Response` — routes to primary model, calls LiteLLM.
- `handle_passthrough(request, api_base) -> Response` — forwards raw requests via httpx.

### proxy/streaming.py

- `stream_completion(response) -> AsyncIterator[str]` — the SSE async generator.

### router/registry.py

- `ModelRegistry` class that stores and queries discovered and config-provided models.
- Lookups: `get_by_name(name)`, `get_all()`, `sorted_by_cost()`, `names()`.

### router/detector.py

- `detect_feature(messages, max_tokens, temperature) -> FeatureType` — analyzes request signals, returns `completion` or `chat`.
- `FeatureType` enum: `COMPLETION`, `CHAT`.

### router/engine.py

- `RoutingEngine` class that combines detection + model selection + fallback.
- `select_model(messages, max_tokens, temperature) -> ModelConfig` — detects feature type, returns primary model.
- `fallback_order(primary) -> list[ModelConfig]` — returns remaining models sorted by cost.
- Auto-selects cheapest model (local first) as primary if no override is set.

---

## Dependencies

```
fastapi>=0.115.0
uvicorn>=0.30.0
litellm>=1.40.0
pyyaml>=6.0
httpx>=0.27.0
```

| Dependency | Purpose |
|---|---|
| `fastapi` | HTTP server, request validation, streaming responses |
| `uvicorn` | ASGI server to run FastAPI |
| `litellm` | Unified interface to model backends (100+ providers) |
| `pyyaml` | Config file parsing |
| `httpx` | Async HTTP client for transparent passthrough |

---

## Verification

### Basic Connectivity

1. Start Rex:
   ```bash
   uv run uvicorn app.main:app --host 0.0.0.0 --port 8000
   ```
2. Test health:
   ```bash
   curl http://localhost:8000/health
   ```
3. Test models list:
   ```bash
   curl http://localhost:8000/v1/models
   ```

### Completion Routing

4. Send a short single-turn request (detected as completion):
   ```bash
   curl -X POST http://localhost:8000/v1/chat/completions \
     -H "Content-Type: application/json" \
     -d '{"messages": [{"role": "user", "content": "complete: def hello"}], "max_tokens": 50, "temperature": 0}'
   ```
5. Send a multi-turn chat request (detected as chat):
   ```bash
   curl -X POST http://localhost:8000/v1/chat/completions \
     -H "Content-Type: application/json" \
     -d '{"messages": [{"role": "system", "content": "You are a helpful assistant"}, {"role": "user", "content": "Explain how async works in Python"}, {"role": "assistant", "content": "..."}, {"role": "user", "content": "Show me an example with aiohttp"}]}'
   ```

### Streaming

6. Test streaming with a chat request:
   ```bash
   curl -X POST http://localhost:8000/v1/chat/completions \
     -H "Content-Type: application/json" \
     -d '{"messages": [{"role": "user", "content": "Hello"}], "stream": true}'
   ```
   - Verify SSE chunks arrive incrementally, each prefixed with `data: `.
   - Verify the stream ends with `data: [DONE]\n\n`.

### Fallback

7. Configure a model with an invalid API key or unreachable `api_base`.
   - Send a request that routes to the broken model.
   - Verify Rex falls back to the next model and returns a valid response.

### Client Integration

8. Configure a coding tool to use `http://localhost:8000` as the API base URL.
   - Send a tab completion and verify the primary model responds.
   - Send a chat message and verify the primary model responds.
