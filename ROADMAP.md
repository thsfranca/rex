# Rex — Roadmap

For system architecture, design decisions, and routing strategy, see [ARCHITECTURE.md](ARCHITECTURE.md).

---

## Phase 0 — Proxy + Basic Routing

> Detailed spec: [docs/specs/phase-0-proxy-basic-routing.md](docs/specs/phase-0-proxy-basic-routing.md)

**Goal**: An OpenAI-compatible proxy that routes requests across multiple model backends based on feature detection.

**Deliverables**:
- FastAPI app with full OpenAI-compatible API:
  - `POST /v1/chat/completions` (streaming and non-streaming)
  - `POST /v1/completions` (legacy)
  - `GET /v1/models` (returns discovered models)
  - `GET /health` (proxy status)
  - Transparent passthrough for all other endpoints
- Server-Sent Events (SSE) streaming support
- Zero-config model discovery:
  - Detect available providers from environment variables (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, etc.)
  - Query each provider's API for available models
  - Probe local runtimes (Ollama at `localhost:11434`)
  - Enrich models with metadata from LiteLLM's built-in database (context window, pricing, capabilities)
  - Auto-select the cheapest model as the primary for all tasks
- Optional YAML config for overrides (add custom models, override auto-selected routes)
- LiteLLM integration for multiple backends (local + cloud APIs)
- Feature detection from request signals:
  - Tab completion: short, single-turn, low temperature, low max_tokens
  - Chat/Agent: multi-turn conversations, longer context
- Route all tasks to the cheapest model; fallback chain escalates to more expensive models on failure
- Fallback chain: if primary model fails or times out, try next best
- Verified client connectivity (configure base URL, send a prompt, get a response)

---

## Phase 1 — Heuristic Task Classifier + Embedding Pipeline

**Goal**: Classify chat/agent prompts by coding task type using fast pattern-based heuristics. Begin storing query embeddings for the learning pipeline.

**Deliverables**:
- Signal extraction from the last user message:
  - Keyword matching (error/fix/bug → debugging, refactor/clean → refactoring, etc.)
  - Structural analysis (code block ratio, prompt length, presence of stack traces)
  - Language detection (to prefer models that excel in specific languages)
- Confidence scoring: each signal contributes a weighted score, highest-scoring category wins
- All categories start on the cheapest model; the learning pipeline logs outcomes per category for future upward migration
- Sentence transformer integration ([all-MiniLM-L6-v2](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2); [Reimers & Gurevych, 2019](https://arxiv.org/abs/1908.10084)):
  - Embed every query on each request (~10ms, local CPU)
  - Store embeddings alongside heuristic rule votes in SQLite
- Pre-seeded cluster centroids:
  - Synthetic exemplar queries for each predefined category
  - The system embeds them at startup to serve as initial centroids
  - Nearest-centroid classification available from the first query
- SQLite decision logging:
  - Timestamp, prompt hash, detected category, confidence score
  - Selected model, response time, token count, cost (from LiteLLM runtime calculation)
  - Query embedding vector and individual heuristic rule votes

---

## Phase 2 — LLM-as-Judge Fallback

**Goal**: Use a small local LLM to classify the task when heuristic confidence is low ([Zheng et al., 2023](https://arxiv.org/abs/2306.05685)).

**Deliverables**:
- Confidence threshold: below this, trigger the judge instead of routing on low-confidence heuristics
- Small local model integration via LiteLLM
- Classification meta-prompt that asks the judge to return structured JSON:
  - `category`: one of the defined task types
  - `complexity`: 1-5 rating
  - `min_context_window`: estimated tokens needed
- JSON mode parsing of judge response
- Latency guard: only triggered for chat/agent paths, never for completions
- Separate logging of judge decisions for accuracy analysis

---

## Phase 3 — Learning Pipeline + Visibility

**Goal**: Train the ML classifier automatically from accumulated data using unsupervised clustering and weak supervision. Provide visibility into routing behavior.

**Deliverables**:
- Unsupervised clustering (K-means):
  - Group stored query embeddings into natural task categories
  - [Silhouette score](https://doi.org/10.1016/0377-0427(87)90125-7) to automatically determine optimal cluster count (Rousseeuw, 1987)
  - Periodic re-clustering as query volume grows (~every 100 new queries)
- Weak supervision label model ([Ratner et al., 2017](https://arxiv.org/abs/1711.10160)):
  - Treat heuristic rules as noisy labeling functions
  - The probabilistic model aggregates rule votes by learning each rule's reliability from agreement/disagreement patterns
  - Outputs clean probabilistic labels per query, no manual annotation needed
- ML classifier training:
  - Logistic regression on cluster-derived + weakly-supervised labels
  - Automatic retraining when new clusters stabilize
  - Per-category outcome tracking (fallback triggers, error rate, latency, re-ask rate)
  - Upward migration: the pipeline identifies categories with persistent poor outcomes on the cheap model and promotes them to more capable models
- CLI tool (`rex stats`):
  - Routing statistics: requests per category, per model
  - Cost tracking: cumulative spend, savings vs. single-model baseline
  - Cluster visualization: discovered categories and their characteristics
  - Export: labeled dataset as CSV/JSON

---

## Phase 4 — Personalized ML Classifier

**Goal**: The ML classifier, trained automatically by the learning pipeline, becomes the primary router — personalized to each user's coding patterns.

This phase activates automatically when:
- Clustering silhouette score crosses the quality threshold (>0.5)
- Weak supervision label model converges on rule reliability scores

No manual labeling required:
- The system bootstraps from heuristics.
- The learning pipeline discovers patterns from the query stream.
- The classifier improves over time as usage accumulates.

**Deliverables**:
- The ML classifier replaces heuristics as primary in the classifier chain
- Heuristics demote to labeling functions only (feed the learning pipeline, no longer route directly)
- The router promotes task categories to more capable models based on accumulated outcome data (upward migration)
- Categories that work well on cheap models stay cheap; only categories with persistent poor outcomes migrate up
- The learning pipeline retrains continuously as usage patterns evolve

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Heuristic accuracy is low initially | Wrong model selected, poor output quality | LLM judge fallback catches worst cases; learning pipeline automatically improves over time |
| LLM judge adds too much latency | Slow responses for uncertain tasks | Only triggered for chat/agent (not completions); raise confidence threshold to reduce fallback rate |
| Client sends unexpected request formats | Proxy breaks | Phase 0 validates integration before adding complexity |
| Local models need significant RAM/GPU | Can't run on all hardware | Rex works with cloud-only providers; local runtimes are auto-detected, not required |
| Model availability (local runtime not running, API down) | Request fails | Fallback chain in routing engine; health checks on startup |

## Success Metrics

- **Routing accuracy**: % of requests where the router selected an appropriate model (measured via cluster coherence and weak supervision confidence)
- **Cost savings**: Estimated cost vs. using a single frontier model for everything
- **Latency overhead**: Time the classification adds (target: <1ms for heuristics, <50ms for ML classifier, <500ms for judge)
- **Fallback rate**: % of requests that hit the LLM judge (target: <20%, decreasing as ML classifier improves)
- **Cluster stability**: Silhouette score of discovered categories (target: >0.5)
- **Classifier takeover**: Time until the ML classifier replaces heuristics as primary router
