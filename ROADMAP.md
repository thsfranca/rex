# Rex — Roadmap

For system architecture, design decisions, and routing strategy, see [ARCHITECTURE.md](ARCHITECTURE.md).

---

## Phase 0 — Skeleton Proxy

**Goal**: A working OpenAI-compatible proxy that Cursor can connect to, forwarding all requests to a single default model.

**Deliverables**:
- FastAPI app with `/v1/chat/completions` endpoint
- Server-Sent Events (SSE) streaming support
- Basic YAML config loading
- Single model backend via LiteLLM
- Verified Cursor connectivity (configure base URL, send a prompt, get a response)

**Why this first**: Validates the integration point with Cursor before adding any intelligence. If streaming doesn't work or Cursor sends unexpected request formats, we find out here — not after building a complex router.

**Estimated effort**: 1 day

---

## Phase 1 — Model Registry + Feature-Based Routing

**Goal**: Support multiple model backends and route based on Cursor feature type (completion vs. chat/agent).

**Deliverables**:
- YAML-based model registry with metadata per model:
  - `name`, `provider`, `context_window`
  - `cost_per_1k_input`, `cost_per_1k_output`
  - `strengths` (list of task categories)
  - `max_latency_ms`, `is_local`
- LiteLLM integration for multiple backends (Ollama + cloud APIs)
- Feature detection from request signals:
  - Tab completion: short, single-turn, specific system prompts
  - Chat/Agent: multi-turn conversations, longer context
- Route tab completion to fastest model, everything else to default strongest model
- Fallback chain: if primary model fails or times out, try next best

**Estimated effort**: 2-3 days

---

## Phase 2 — Heuristic Task Classifier

**Goal**: Classify chat/agent prompts by coding task type using fast pattern-based heuristics.

**Deliverables**:
- Signal extraction from the last user message:
  - Keyword matching (error/fix/bug → debugging, refactor/clean → refactoring, etc.)
  - Structural analysis (code block ratio, prompt length, presence of stack traces)
  - Language detection (to prefer models that excel in specific languages)
- Confidence scoring: each signal contributes a weighted score, highest-scoring category wins
- Routing table: map task category → best model from registry
- SQLite decision logging:
  - Timestamp, prompt hash, detected category, confidence score
  - Selected model, response time, token count, estimated cost

**Estimated effort**: 3-4 days

---

## Phase 3 — LLM-as-Judge Fallback

**Goal**: For uncertain heuristic classifications, use a small local LLM to classify the task before routing.

**Deliverables**:
- Confidence threshold: below this, trigger the judge instead of routing on low-confidence heuristics
- Small local model integration (Llama 3.2 1B or Phi-3 Mini via Ollama)
- Classification meta-prompt that asks the judge to return structured JSON:
  - `category`: one of the defined task types
  - `complexity`: 1-5 rating
  - `min_context_window`: estimated tokens needed
- JSON mode parsing of judge response
- Latency guard: only triggered for chat/agent paths, never for completions
- Separate logging of judge decisions for accuracy analysis

**Estimated effort**: 2-3 days

---

## Phase 4 — Feedback Loop + Data Collection

**Goal**: Collect structured data that enables training an ML classifier later, and provide visibility into routing behavior.

**Deliverables**:
- Enriched logging per request:
  - Truncated prompt content (configurable privacy level)
  - Heuristic classification + confidence
  - LLM judge classification (if triggered)
  - Final model selected
  - Response latency, token count (input/output), estimated cost
- CLI tool (`rex stats`):
  - Routing statistics: requests per category, per model
  - Cost tracking: cumulative spend, savings vs. single-model baseline
  - Review mode: show uncertain classifications for manual labeling
  - Export: labeled dataset as CSV/JSON for future ML training

**Estimated effort**: 2-3 days

---

## Phase 5 — ML Classifier (Future)

**Goal**: Replace heuristics with a trained classifier once enough labeled data exists.

**Not part of the prototype.** This phase begins when:
- At least 500+ labeled routing decisions have been collected
- Manual review of uncertain cases has produced ground-truth labels

Likely approach:
- Lightweight text classifier (TF-IDF + logistic regression, or fine-tuned DistilBERT)
- Trained on collected data from Phases 2-4
- Runs locally, <50ms inference
- Replaces heuristics as the primary classifier, with heuristics kept as a fast pre-filter

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Heuristic accuracy is low initially | Wrong model selected, poor output quality | LLM judge fallback catches worst cases; logs enable rapid rule iteration |
| LLM judge adds too much latency | Slow responses for uncertain tasks | Only triggered for chat/agent (not completions); raise confidence threshold to reduce fallback rate |
| Cursor sends unexpected request formats | Proxy breaks | Phase 0 validates integration before adding complexity |
| Local models need significant RAM/GPU | Can't run on all hardware | Config supports cloud-only mode; model pool is user-configurable |
| Model availability (Ollama not running, API down) | Request fails | Fallback chain in routing engine; health checks on startup |

## Success Metrics

- **Routing accuracy**: % of requests where the selected model was appropriate (measured via manual review)
- **Cost savings**: Estimated cost vs. using a single frontier model for everything
- **Latency overhead**: Time added by classification (target: <1ms for heuristics, <500ms for judge)
- **Fallback rate**: % of requests that hit the LLM judge (target: <20%)
