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

**Goal**: Classify chat/agent prompts by coding task type using fast pattern-based heuristics.

**Deliverables**:
- [x] Keyword matching on last user message (error/fix/bug → debugging, refactor/clean → refactoring, etc.)
- [x] Structural analysis (stack trace detection)
- [x] Confidence scoring: each signal contributes a weighted score, highest-scoring category wins
- [x] Task-aware routing: categories route to the cheapest model that meets their requirements (context window, cloud, capabilities)
- [x] `supports_reasoning` routing criterion: add to `ModelConfig`, `TaskRequirements`, and enrich from LiteLLM metadata; require for debugging, optimization, and code review categories
- [x] Structural analysis (code block ratio, prompt length)
- [x] Client adapter interface: normalize tool-specific request patterns (Cursor, Claude Code, etc.) into a common format for the classifier

---

## Phase 2 — Enrichment Pipeline

> Detailed spec: [docs/specs/enrichment-pipeline.md](docs/specs/enrichment-pipeline.md)

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
- [ ] Confidence threshold: below this, trigger the judge instead of routing on low-confidence heuristics
- [ ] Small local model integration via LiteLLM
- [ ] Classification meta-prompt that returns structured JSON (`category`, `min_context_window`)
- [ ] JSON mode parsing of judge response
- [ ] Latency guard: only triggered for chat/agent paths, never for completions

---

## Phase 4 — Decision Logging + Embedding Pipeline

**Goal**: Build the data collection infrastructure that feeds the learning pipeline. Decision logging provides routing observability, and sentence transformer embeddings enable semantic classification from the first query.

**Deliverables**:
- [ ] Decision logging with storage interface: repository pattern, SQLite implementation (timestamp, prompt hash, category, confidence, selected model, response time, token count, cost, rule votes)
- [ ] Sentence transformer integration ([all-MiniLM-L6-v2](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2); [Reimers & Gurevych, 2019](https://arxiv.org/abs/1908.10084)): embed every query (~10ms, local CPU), store embeddings alongside decision logs
- [ ] Pre-seeded cluster centroids: synthetic exemplar queries per category, embedded at startup as initial centroids, nearest-centroid classification from the first query

---

## Phase 5 — Learning Pipeline + ML Classifier

**Goal**: Train a personalized ML classifier from accumulated data using unsupervised clustering and weak supervision. The classifier replaces heuristics as the primary router once it reaches quality thresholds.

**Deliverables**:
- [ ] Unsupervised K-means clustering on stored query embeddings with [silhouette score](https://doi.org/10.1016/0377-0427(87)90125-7) for optimal cluster count (Rousseeuw, 1987)
- [ ] Periodic re-clustering as query volume grows (~every 100 new queries)
- [ ] Weak supervision label model: heuristic rules as noisy labeling functions, probabilistic aggregation of votes ([Ratner et al., 2017](https://arxiv.org/abs/1711.10160))
- [ ] ML classifier module: logistic regression inference, integration into the classifier chain, model persistence
- [ ] ML classifier training: train on cluster-derived + weakly-supervised labels, automatic retraining as usage patterns evolve
- [ ] Automatic promotion: ML classifier replaces heuristics as primary when silhouette score > 0.5 and label model converges; heuristics demote to labeling functions only
- [ ] Per-category outcome tracking (fallback triggers, error rate, latency, re-ask rate)
- [ ] Upward migration: promote categories with persistent poor outcomes to more capable models
