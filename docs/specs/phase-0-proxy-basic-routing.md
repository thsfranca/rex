# Phase 0 — Proxy + Basic Routing

For the full delivery plan, see [ROADMAP.md](../../ROADMAP.md). For system design and routing strategy, see [ARCHITECTURE.md](../../ARCHITECTURE.md).

---

## Goal

- Build an OpenAI-compatible proxy that routes requests across multiple model backends.
- Detect whether a request is a tab completion or a chat/agent interaction.
- Route completions to the fastest model and chat/agent requests to the strongest model.
- Fall back to the next available model if the primary fails.
- Validate end-to-end integration with AI coding tools (streaming, request formats, connectivity).

---

## Config Schema

Rex loads a YAML config file at startup.

```yaml
server:
  host: "0.0.0.0"
  port: 8000

models:
  - name: "openai/gpt-4o"
    provider: "openai"
    context_window: 128000
    cost_per_1k_input: 0.005
    cost_per_1k_output: 0.015
    strengths:
      - debugging
      - refactoring
      - code_review
      - generation
    max_latency_ms: 2000
    is_local: false
    api_key: "sk-..."

  - name: "ollama/llama3"
    provider: "ollama"
    context_window: 8192
    cost_per_1k_input: 0
    cost_per_1k_output: 0
    strengths:
      - completion
      - explanation
    max_latency_ms: 100
    is_local: true
    api_base: "http://localhost:11434"

routing:
  completion_model: "ollama/llama3"
  default_model: "openai/gpt-4o"
```

### Server

| Field | Type | Required | Description |
|---|---|---|---|
| `server.host` | string | yes | Address Rex listens on |
| `server.port` | integer | yes | Port Rex listens on |

### Models

Each entry in the `models` list describes a model backend:

| Field | Type | Required | Description |
|---|---|---|---|
| `name` | string | yes | LiteLLM model identifier (e.g., `openai/gpt-4o`, `ollama/llama3`) |
| `provider` | string | yes | Provider name (e.g., `openai`, `ollama`, `anthropic`) |
| `context_window` | integer | yes | Maximum context length in tokens |
| `cost_per_1k_input` | float | yes | Cost per 1,000 input tokens (0 for local models) |
| `cost_per_1k_output` | float | yes | Cost per 1,000 output tokens (0 for local models) |
| `strengths` | list[string] | yes | Task categories this model excels at |
| `max_latency_ms` | integer | yes | Expected maximum response latency |
| `is_local` | boolean | yes | Whether the model runs locally |
| `api_key` | string | no | API key for this model's provider |
| `api_base` | string | no | Override the provider's default API URL |

### Routing

| Field | Type | Required | Description |
|---|---|---|---|
| `routing.completion_model` | string | yes | Model identifier used for tab completion requests |
| `routing.default_model` | string | yes | Model identifier used for chat/agent requests and as the passthrough target |

- Rex validates the config at startup using a Pydantic `Settings` model.
- If the config file is missing or invalid, Rex fails immediately with a clear error message.
- `routing.completion_model` and `routing.default_model` must reference a `name` in the `models` list.
- LiteLLM infers the provider from the model identifier prefix (e.g., `openai/` → OpenAI, `ollama/` → Ollama).

---

## Model Registry

- Rex loads the `models` list from config at startup into an in-memory registry.
- Each model entry becomes a `ModelConfig` Pydantic model.
- The registry provides lookups by name and filtering by capability (strengths, is_local, latency).

---

## Feature Detection

Rex detects whether a request is a **tab completion** or a **chat/agent** interaction based on request signals:

| Signal | Completion | Chat/Agent |
|---|---|---|
| Conversation length | Single turn (1 user message) | Multi-turn (2+ messages) |
| Prompt length | Short (< 200 tokens) | Longer |
| System prompt | Contains code context / cursor position | General instructions |
| `max_tokens` | Low (< 500) | Higher or unset |
| `temperature` | Low (0-0.2) | Higher or unset |

- The detector examines the request and returns a feature type: `completion` or `chat`.
- Phase 0 uses a simple scoring approach: each signal contributes a weighted score, and the highest-scoring feature type wins.
- If the score is too close to call, the detector defaults to `chat` (the safer option — uses the stronger model).

### Routing Logic

```
if feature_type == "completion":
    model = routing.completion_model
else:
    model = routing.default_model
```

---

## Fallback Chain

If the selected model fails (connection error, timeout, rate limit), Rex tries the next available model:

1. Try the primary model for the detected feature type.
2. On failure, try the `default_model` (if different from the primary).
3. On failure, try any remaining model in the registry.
4. If all models fail, return the error from the last attempt to the client.

- Rex logs each fallback attempt (which model failed, why, which model it fell back to).
- The fallback chain adds no overhead on the happy path — it only activates on failure.

---

## Endpoints

### POST /v1/chat/completions

- Accepts the full OpenAI chat completions request body.
- Runs feature detection on the request to determine `completion` vs. `chat`.
- Selects the model based on the routing logic and ignores the `model` field from the request.
- Passes all other parameters through to LiteLLM (`messages`, `temperature`, `max_tokens`, `top_p`, `stop`, `stream`, etc.).

**Non-streaming** (`stream: false` or omitted):

- Calls `await litellm.acompletion(model=selected_model, **params)`.
- Returns the response as JSON with `Content-Type: application/json`.

**Streaming** (`stream: true`):

- Calls `await litellm.acompletion(model=selected_model, stream=True, **params)`.
- Returns a `StreamingResponse` with `Content-Type: text/event-stream`.
- Each chunk: `data: {json}\n\n`
- Final signal: `data: [DONE]\n\n`

### POST /v1/completions

- Same pattern as chat completions but calls `await litellm.atext_completion()`.
- Supports both streaming and non-streaming modes.
- Always routes to `routing.completion_model` (no feature detection needed — this endpoint is inherently a completion).

### GET /v1/models

- Returns all models from the registry in OpenAI's models list format:

```json
{
  "object": "list",
  "data": [
    {
      "id": "openai/gpt-4o",
      "object": "model",
      "created": 1700000000,
      "owned_by": "rex"
    },
    {
      "id": "ollama/llama3",
      "object": "model",
      "created": 1700000000,
      "owned_by": "rex"
    }
  ]
}
```

### GET /health

- Returns proxy status and model availability:

```json
{
  "status": "ok",
  "models": {
    "openai/gpt-4o": "available",
    "ollama/llama3": "unavailable"
  }
}
```

### Transparent Passthrough (catch-all)

- Catches any request to a path not handled above.
- Forwards the raw request (method, path, headers, query params, body) to the `default_model`'s `api_base` via `httpx.AsyncClient`.
- Returns the upstream response as-is (status code, headers, body).
- If no `api_base` is configured for the default model, returns `501 Not Implemented`.

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

- Missing config file → exit with `"Config file not found: {path}"`.
- Invalid config values → exit with Pydantic validation errors.
- `routing.completion_model` or `routing.default_model` not found in `models` list → exit with `"Model '{name}' referenced in routing but not defined in models"`.
- Rex does not start if the config is invalid.

---

## Project Files

Phase 0 creates only the files needed for a working proxy with basic routing:

```
app/
  main.py              # FastAPI app, lifespan, endpoint definitions
  config.py            # Pydantic Settings model, YAML loader
  proxy/
    handler.py         # Completion request handling via LiteLLM
    streaming.py       # SSE async generator
  router/
    registry.py        # Model registry loader + lookups
    detector.py        # Feature detection (completion vs. chat)
    engine.py          # Routing engine (detector → model selection + fallback)
config.yaml.example   # Example configuration
pyproject.toml         # Project dependencies (uv)
tests/                 # pytest test suite
```

### main.py

- Defines the FastAPI app with a lifespan context manager.
- Loads the config and initializes the model registry at startup.
- Registers all endpoint routes.
- The catch-all route handles transparent passthrough.

### config.py

- `ModelConfig` Pydantic model for each model entry.
- `RoutingConfig` Pydantic model for routing settings.
- `Settings` Pydantic model for the full config schema.
- `load_config(path: str) -> Settings` function that reads YAML and validates.

### proxy/handler.py

- `handle_chat_completion(request, engine) -> Response` — runs feature detection, selects model, calls LiteLLM, returns JSON or streaming.
- `handle_text_completion(request, engine) -> Response` — routes to completion model, calls LiteLLM.
- `handle_passthrough(request, settings) -> Response` — forwards raw requests via httpx.

### proxy/streaming.py

- `stream_completion(response) -> AsyncIterator[str]` — the SSE async generator.

### router/registry.py

- `ModelRegistry` class that loads models from config.
- Lookups: `get_by_name(name)`, `get_all()`, `get_by_strength(category)`.

### router/detector.py

- `detect_feature(request) -> FeatureType` — analyzes request signals, returns `completion` or `chat`.
- `FeatureType` enum: `COMPLETION`, `CHAT`.

### router/engine.py

- `RoutingEngine` class that combines detection + model selection + fallback.
- `route(request) -> ModelConfig` — detects feature type, selects model, handles fallback chain.

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
   uvicorn rex.main:app --host 0.0.0.0 --port 8000
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

4. Send a short single-turn request (should route to `completion_model`):
   ```bash
   curl -X POST http://localhost:8000/v1/chat/completions \
     -H "Content-Type: application/json" \
     -d '{"messages": [{"role": "user", "content": "complete: def hello"}], "max_tokens": 50, "temperature": 0}'
   ```
5. Send a multi-turn chat request (should route to `default_model`):
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
   - Send a tab completion and verify the fast model responds.
   - Send a chat message and verify the strong model responds.
