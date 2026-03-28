# Rex — Roadmap

For system architecture, design decisions, and routing strategy, see [ARCHITECTURE.md](ARCHITECTURE.md).

---

## Phase 0 — Proxy + Basic Routing

> Detailed spec: [docs/specs/phase-0-proxy-basic-routing.md](docs/specs/phase-0-proxy-basic-routing.md)

**Goal**: An OpenAI-compatible proxy that routes requests across multiple model backends based on feature detection.

**Deliverables**:
- [x] FastAPI app with full OpenAI-compatible API (`POST /v1/chat/completions`, `POST /v1/completions`, `GET /v1/models`, `GET /health`, transparent passthrough)
- [x] Server-Sent Events (SSE) streaming support
- [x] Zero-config model discovery (detect providers from env vars, query provider APIs, probe Ollama, enrich with LiteLLM metadata, auto-select cheapest primary)
- [x] Optional YAML config for overrides (add custom models, override auto-selected routes)
- [x] LiteLLM integration for multiple backends (local + cloud APIs)
- [x] Feature detection from request signals (tab completion vs. chat/agent)
- [x] Cost-first routing with fallback chain on failure

---

## Phase 1 — Heuristic Task Classifier

> Detailed spec: [docs/specs/phase-1-heuristic-task-classifier.md](docs/specs/phase-1-heuristic-task-classifier.md)

**Goal**: Classify chat/agent prompts by coding task type using fast pattern-based heuristics.

**Deliverables**:
- [x] Keyword matching on last user message (error/fix/bug → debugging, refactor/clean → refactoring, etc.)
- [x] Structural analysis (stack trace detection)
- [x] Confidence scoring: each signal contributes a weighted score, highest-scoring category wins
- [x] Task-aware routing: categories route to the cheapest model that meets their requirements (context window, reasoning, capabilities)
- [x] `supports_reasoning` routing criterion: add to `ModelConfig`, `TaskRequirements`, and enrich from LiteLLM metadata; require for debugging, optimization, and code review categories
- [x] Structural analysis (code block ratio, prompt length)
- [x] Client adapter interface: normalize tool-specific request patterns (Cursor, Claude Code, etc.) into a common format for the classifier

---

## Phase 2 — Enrichment Pipeline

> Detailed spec: [docs/specs/phase-2-enrichment-pipeline.md](docs/specs/phase-2-enrichment-pipeline.md)

**Goal**: A pluggable pipeline that transforms requests after routing but before the model call. The first enricher injects task decomposition instructions for complex tasks, replicating structured step-by-step execution across all models.

**Deliverables**:
- [x] Enricher interface: receives request (messages, model, task category), returns modified request
- [x] Enrichment pipeline runner: executes enabled enrichers in sequence, no-op when none are enabled
- [x] Task decomposition enricher: detects complex tasks via classifier output and prompt signals, injects system-level decomposition instruction
- [x] Config extension: `enrichments.task_decomposition` opt-in toggle (default: off)
- [x] Skip logic: pipeline only applies to `chat` requests, never to `completion` (tab completions)

---

## Phase 3 — LLM-as-Judge Fallback

> Detailed spec: [docs/specs/phase-3-llm-judge-fallback.md](docs/specs/phase-3-llm-judge-fallback.md)

**Goal**: Use a small local LLM to classify the task when heuristic confidence is low ([Zheng et al., 2023](https://arxiv.org/abs/2306.05685)).

**Deliverables**:
- [x] Confidence threshold: below this, trigger the judge instead of routing on low-confidence heuristics
- [x] Small local model integration via LiteLLM (auto-selects cheapest local model when none configured)
- [x] Classification meta-prompt that returns structured JSON (`category`, `min_context_window`)
- [x] JSON mode parsing of judge response with category validation
- [x] Latency guard: only triggered for chat/agent paths, never for completions

---

## Phase 4 — Decision Logging + Embedding Pipeline

> Detailed spec: [docs/specs/phase-4-decision-logging-embeddings.md](docs/specs/phase-4-decision-logging-embeddings.md)

**Goal**: Build the data collection infrastructure that feeds the learning pipeline. Decision logging provides routing observability, and sentence transformer embeddings enable semantic classification from the first query.

**Deliverables**:
- [x] Decision logging with storage interface: repository pattern, SQLite implementation (timestamp, prompt hash, category, confidence, selected model, response time, token count, cost, rule votes)
- [x] Sentence transformer integration ([all-MiniLM-L6-v2](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2); [Reimers & Gurevych, 2019](https://arxiv.org/abs/1908.10084)): embed every query (~10ms, local CPU), store embeddings alongside decision logs
- [x] Pre-seeded cluster centroids: synthetic exemplar queries per category, embedded at startup as initial centroids, nearest-centroid classification from the first query

---

## Phase 5 — Learning Pipeline + ML Classifier

> Detailed spec: [docs/specs/phase-5-learning-pipeline-ml-classifier.md](docs/specs/phase-5-learning-pipeline-ml-classifier.md)

**Goal**: Train a personalized ML classifier from accumulated data using unsupervised clustering and weak supervision. The classifier replaces heuristics as the primary router once it reaches quality thresholds.

**Deliverables**:
- [x] Unsupervised K-means clustering on stored query embeddings with [silhouette score](https://doi.org/10.1016/0377-0427(87)90125-7) for optimal cluster count (Rousseeuw, 1987)
- [x] Periodic re-clustering as query volume grows (~every 100 new queries)
- [x] Weak supervision label model: heuristic rules as noisy labeling functions, probabilistic aggregation of votes ([Ratner et al., 2017](https://arxiv.org/abs/1711.10160))
- [x] ML classifier module: logistic regression inference, integration into the classifier chain, model persistence
- [x] ML classifier training: train on cluster-derived + weakly-supervised labels, automatic retraining as usage patterns evolve
- [x] Automatic promotion: ML classifier replaces heuristics as primary when silhouette score > 0.5 and label model converges; heuristics demote to labeling functions only

---

## Phase 5.1 — Confidence-Based Model Escalation

**Goal**: Route uncertain requests to more capable models based on classification confidence. Extends the classifier chain so that low confidence triggers not just classifier escalation but also model escalation.

**Deliverables**:
- [x] Confidence-based model escalation: when confidence remains below threshold after the full classifier chain, select the next more capable model in cost order instead of the cheapest
- [x] `escalated` flag on routing decisions for observability in the decision log
- [x] Remove outcome tracker (`outcomes.py`) — per-request escalation replaces retroactive outcome-based category promotion

**Design**: See [ARCHITECTURE.md — Confidence-Based Model Escalation](ARCHITECTURE.md#confidence-based-model-escalation) for the full design rationale and flow diagram.

**Research basis**: [Confidence-Driven LLM Router](https://arxiv.org/abs/2502.11021) (Zhang et al., 2025) — uncertainty-based routing outperforms accuracy-based and preference-based routing.

---

## Phase 5.2 — Training Reset

**Goal**: Allow users to intentionally reset all learning data and trained models, returning Rex to its initial state.

**Deliverables**:
- [x] `POST /v1/reset` endpoint that clears all learning state in a single call
- [x] Clear all decision records from the SQLite database (`decisions` table)
- [x] Delete the trained ML classifier model file (`~/.rex/ml_classifier.joblib`) and clear in-memory model state
- [x] Reset the retraining scheduler (training counter, promotion status, label model)
- [x] Restore the cold-start centroid classifier from synthetic exemplars
- [x] Demote the ML classifier in the routing engine so heuristics resume as the primary classifier

**Design**: After reset, Rex behaves exactly as it does on a fresh start — heuristics classify requests, cold-start centroids provide semantic fallback, and the learning pipeline begins accumulating data from scratch.

---

## Phase 5.3 — CLI + Config-First Model Registry

**Goal**: Make Rex installable as a global CLI command and prioritize config-defined models over auto-discovered ones.

**Deliverables**:
- [x] CLI entry point: install Rex globally (e.g., `uv tool install rex`), run with `rex start` or `rex`
- [x] `rex stop` shuts down a running Rex instance gracefully
- [x] `rex reset` clears all learning data with an interactive confirmation prompt (skippable with `--yes`)
- [x] CLI runs Rex as a background process; clients point their API base URL at Rex's address (see README.md **Client base URL**)
- [x] Config-first model registry: when `models` is defined in `~/.rex/config.yaml`, Rex uses that list as the primary model source
- [x] Auto-discovery supplements config-defined models only — adds models not already listed in config
- [x] Without a config file, auto-discovery works as it does today (no behavior change)

**Design**: Rex currently discovers models automatically and treats config as optional overrides. This inverts the priority — config-defined models come first, auto-discovery fills in the gaps. The CLI makes Rex a single command away from running globally, so any AI tool (Cursor, Claude Code, Continue, Aider, etc.) can connect to the same instance.

---

## Phase 5.4 — Anthropic Messages API

**Goal**: Support Anthropic-native clients (Claude Code, etc.) by accepting and responding in Anthropic Messages API format, while routing through Rex's existing engine.

**Deliverables**:
- [x] Anthropic message format translator: convert between Anthropic Messages API and OpenAI chat completions format (system prompt extraction, content block flattening, stop reason mapping)
- [x] `POST /v1/messages` endpoint: accept Anthropic-format requests, route through Rex's engine, return Anthropic-format responses
- [x] API key extraction from `x-api-key` header (Anthropic convention) with passthrough to model backends
- [x] Anthropic SSE streaming: convert OpenAI streaming chunks to Anthropic event format (`message_start`, `content_block_start`, `content_block_delta`, `content_block_stop`, `message_delta`, `message_stop`)
- [x] Echo the client's requested model name in responses while routing to the best model internally

**Design**: Rex translates Anthropic requests to OpenAI format internally, routes through the same engine, and translates the response back. Anthropic-compatible clients set `ANTHROPIC_BASE_URL` to Rex's URL (see README.md **Client base URL**).

---

## Phase 5.5 — Remote Provider Discovery

**Goal**: Support remote LiteLLM proxies and custom API endpoints as configurable providers. Rex probes their model list endpoints at startup and merges discovered models with local and manually configured models.

**Deliverables**:
- [x] `ProviderConfig` in config schema: `prefix`, `api_base`, `api_key` (direct value), `api_key_env` (environment variable name)
- [x] `providers` list in `Settings` (defaults to empty)
- [x] Resolve config providers into `DetectedProvider` at startup, resolving API key from direct value or environment variable
- [x] Config providers override auto-discovered providers with the same prefix (e.g., configuring an Anthropic proxy replaces direct Anthropic API discovery)
- [x] Model listing accepts custom `api_base` for Anthropic and OpenAI-compatible providers
- [x] Skip config providers when their `api_key_env` references an unset environment variable (with warning log)

**Design**: Config providers get resolved into `DetectedProvider` objects during startup. The registry builder merges three sources: config providers, auto-discovered cloud providers (excluding prefixes already defined in config), and auto-discovered local providers (Ollama). This allows users behind corporate LiteLLM proxies to route through their proxy while keeping local model discovery working.

---

## Phase 5.6 — Context-Aware Routing

**Goal**: Route requests to models that can actually fit the input, and recover intelligently when a model rejects a request for exceeding its context window.

**Deliverables**:
- [ ] Pre-call token estimation: before sending a request, estimate the input token count using `litellm.token_counter()` and exclude models whose `max_context_window` cannot fit it
- [ ] Effective context ratio: apply a configurable safety margin (default: 0.8) to `max_context_window` to account for real-world performance degradation before the hard limit ([Redis, 2026](https://redis.io/blog/context-window-overflow/))
- [ ] Context-aware model selection in `RoutingEngine.select_model`: after classification picks a model, verify the request fits; if not, select the next cheapest model with a large enough context window
- [ ] Context-specific fallback in `_call_with_fallback`: catch `litellm.ContextWindowExceededError` separately from general errors and filter remaining fallback candidates to only models with a larger context window than the one that failed
- [ ] Config extension: `routing.effective_context_ratio` (float, default `0.8`) to control the safety margin applied to advertised context windows

**Design**:
- The pre-call check prevents wasted latency and API costs from requests guaranteed to fail. LiteLLM provides `litellm.token_counter(model, messages=messages)` for estimation and `litellm.get_max_tokens(model)` for retrieving model limits.
- The context-specific fallback acts as a safety net for cases where the token count estimate diverges from the provider's actual counting (different tokenizers, images, tool definitions not counted accurately).
- LiteLLM raises `litellm.ContextWindowExceededError` (a subclass of `BadRequestError`) specifically for context overflow, enabling targeted recovery without treating it as a generic failure.
- Research shows model performance degrades 30-40% before the advertised context limit due to the "lost-in-the-middle" problem ([Zylos Research, 2026](https://zylos.ai/research/2026-01-19-llm-context-management)). The effective context ratio defaults to 0.8 to route conservatively.
- Today, `_call_with_fallback` iterates all models in cost order regardless of context window — a request needing 12K tokens might fail on three 8K models before reaching a 32K one. Context-specific fallback skips models that cannot fit the request.

**Design**: See [ARCHITECTURE.md — Context-Aware Routing](ARCHITECTURE.md#context-aware-routing) for the full design rationale and flow diagram.

---

## Phase 6 — Request Timeout + Local Resource Management

**Goal**: Bound how long Rex waits for model responses, propagate cancellation to upstream backends, and optimize resource usage on personal machines running local models.

**Deliverables**:
- [x] Configurable request timeout: global default timeout for `litellm.acompletion` calls, applied via `asyncio.wait_for` so the proxy stops waiting and tears down the upstream HTTP connection
- [x] Per-model timeout override: allow `timeout` on individual model entries in `~/.rex/config.yaml` (local models may need shorter timeouts than cloud APIs)
- [x] Streaming wall-clock limit: cap total stream duration so a runaway generation cannot hold resources indefinitely
- [ ] Client disconnect propagation: detect when the downstream client closes the connection and cancel the in-flight upstream request (streaming and non-streaming paths)
- [x] Config extension: `timeout` under `server` (global default, seconds), `timeout` per model entry, `stream_timeout` for streaming-specific limit
- [ ] Documentation: recommended Ollama environment variables for personal machines (`OLLAMA_NUM_PARALLEL`, `OLLAMA_MAX_LOADED_MODELS`, `OLLAMA_KEEP_ALIVE`, `OLLAMA_CONTEXT_LENGTH`, `OLLAMA_FLASH_ATTENTION`, `OLLAMA_KV_CACHE_TYPE`)

**Design**:
- Rex cannot force a remote provider to stop GPU work. It can only close the HTTP connection. For cloud APIs this is best-effort. For local backends (Ollama, llama.cpp server), closing the connection typically stops generation — but behavior depends on the runtime version and offload mode.
- `max_tokens` remains the only knob that **always** bounds worst-case compute, regardless of cancel semantics.
- Streaming is preferred for long answers because mid-flight abort is meaningful — the local server sees the connection die and can stop generating.
- Ollama's server-side knobs (`OLLAMA_NUM_PARALLEL=1`, conservative `OLLAMA_MAX_LOADED_MODELS`, short `OLLAMA_KEEP_ALIVE`) shape steady-state load on a personal machine so one bad pattern does not pin GPU/RAM.
- Ollama uses llama.cpp as its inference engine. llama.cpp server (recent versions) cancels both prompt processing and token generation when the client disconnects ([PR #9679](https://github.com/ggml-org/llama.cpp/pull/9679)). This means Ollama inherits that cancellation behavior, though Ollama's own connection handling layer can affect how reliably the disconnect reaches llama.cpp.

---

## Phase 7 — Observability Dashboard (optional)

**Goal**: Give users a visual dashboard to explore routing decisions, cost breakdown, model usage, and reliability — without adding UI maintenance burden to Rex. Users install and run the dashboard only if they want it.

**Approach**: Rex ships a pre-configured [Datasette](https://datasette.io/) dashboard definition file. Datasette is a Python tool built specifically for exploring SQLite databases. The [datasette-dashboards](https://datasette.io/plugins/datasette-dashboards) plugin renders interactive charts from declarative YAML config. Rex owns the YAML file with query definitions; Datasette handles all rendering, filtering, and visualization.

**Why Datasette**:
- Same Python ecosystem as Rex — installs with `pip`
- Built for SQLite — reads `~/.rex/decisions.db` directly, zero integration code
- Rex ships a YAML config file, not a UI — no frontend code to build or maintain
- Users also get raw table browsing and a SQL editor for free

**Deliverables**:
- [ ] Datasette dashboard definition file (`rex-dashboards.yml`) with pre-configured panels:
  - Cost overview: total spend, cost over time, cost by category, cost by model
  - Routing: requests by category, requests by model, local vs. cloud ratio, classifier path distribution
  - Reliability: fallback rate, fallbacks by model, error count by model
  - Latency: average latency by model, average latency by category
- [ ] Documentation: install instructions (`pip install datasette datasette-dashboards`), run command (`datasette ~/.rex/decisions.db --metadata rex-dashboards.yml`)