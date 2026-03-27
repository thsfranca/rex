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

## Phase 1 — Heuristic Task Classifier + Embedding Pipeline

**Goal**: Classify chat/agent prompts by coding task type using fast pattern-based heuristics. Begin storing query embeddings for the learning pipeline.

**Deliverables**:
- [x] Keyword matching on last user message (error/fix/bug → debugging, refactor/clean → refactoring, etc.)
- [x] Structural analysis (stack trace detection)
- [x] Confidence scoring: each signal contributes a weighted score, highest-scoring category wins
- [x] Task-aware routing: categories route to the cheapest model that meets their requirements (context window, cloud, capabilities)
- [ ] Structural analysis (code block ratio, prompt length)
- [ ] Language detection (to prefer models that excel in specific languages)
- [ ] Outcome logging per category for future upward migration
- [ ] Sentence transformer integration ([all-MiniLM-L6-v2](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2); [Reimers & Gurevych, 2019](https://arxiv.org/abs/1908.10084)): embed every query (~10ms, local CPU), store alongside heuristic rule votes in SQLite
- [ ] Pre-seeded cluster centroids: synthetic exemplar queries per category, embedded at startup as initial centroids, nearest-centroid classification from the first query
- [ ] SQLite decision logging: timestamp, prompt hash, category, confidence, selected model, response time, token count, cost, embedding vector, rule votes

---

## Phase 2 — LLM-as-Judge Fallback

**Goal**: Use a small local LLM to classify the task when heuristic confidence is low ([Zheng et al., 2023](https://arxiv.org/abs/2306.05685)).

**Deliverables**:
- [ ] Confidence threshold: below this, trigger the judge instead of routing on low-confidence heuristics
- [ ] Small local model integration via LiteLLM
- [ ] Classification meta-prompt that returns structured JSON (`category`, `complexity`, `min_context_window`)
- [ ] JSON mode parsing of judge response
- [ ] Latency guard: only triggered for chat/agent paths, never for completions
- [ ] Separate logging of judge decisions for accuracy analysis

---

## Phase 3 — Learning Pipeline + Visibility

**Goal**: Train the ML classifier automatically from accumulated data using unsupervised clustering and weak supervision. Provide visibility into routing behavior.

**Deliverables**:
- [ ] Unsupervised K-means clustering on stored query embeddings with [silhouette score](https://doi.org/10.1016/0377-0427(87)90125-7) for optimal cluster count (Rousseeuw, 1987)
- [ ] Periodic re-clustering as query volume grows (~every 100 new queries)
- [ ] Weak supervision label model: heuristic rules as noisy labeling functions, probabilistic aggregation of votes ([Ratner et al., 2017](https://arxiv.org/abs/1711.10160))
- [ ] ML classifier training: logistic regression on cluster-derived + weakly-supervised labels, automatic retraining
- [ ] Per-category outcome tracking (fallback triggers, error rate, latency, re-ask rate)
- [ ] Upward migration: promote categories with persistent poor outcomes to more capable models
- [ ] CLI tool (`rex stats`): routing statistics, cost tracking, cluster visualization, dataset export

---

## Phase 4 — Personalized ML Classifier

**Goal**: The ML classifier, trained automatically by the learning pipeline, becomes the primary router — personalized to each user's coding patterns.

This phase activates automatically when:
- Clustering silhouette score crosses the quality threshold (>0.5)
- Weak supervision label model converges on rule reliability scores

**Deliverables**:
- [ ] The ML classifier replaces heuristics as primary in the classifier chain
- [ ] Heuristics demote to labeling functions only (feed the learning pipeline, no longer route directly)
- [ ] The router promotes task categories to more capable models based on accumulated outcome data
- [ ] The learning pipeline retrains continuously as usage patterns evolve
