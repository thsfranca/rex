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
- [ ] `POST /v1/reset` endpoint that clears all learning state in a single call
- [ ] Clear all decision records from the SQLite database (`decisions` table)
- [ ] Delete the trained ML classifier model file (`~/.rex/ml_classifier.joblib`) and clear in-memory model state
- [ ] Reset the retraining scheduler (training counter, promotion status, label model)
- [ ] Restore the cold-start centroid classifier from synthetic exemplars
- [ ] Demote the ML classifier in the routing engine so heuristics resume as the primary classifier

**Design**: After reset, Rex behaves exactly as it does on a fresh start — heuristics classify requests, cold-start centroids provide semantic fallback, and the learning pipeline begins accumulating data from scratch.

---

## Phase 6 — Observability Dashboard (optional)

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
