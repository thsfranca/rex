# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Rex** is an OpenAI- and Anthropic-compatible proxy that routes requests to the cheapest model that fits the task. It runs locally as a CLI tool (`rex start`/`rex stop`) and intelligently classifies coding requests using heuristics, semantic embeddings, and a trained ML classifier.

**Core idea**: Instead of users picking a model, Rex picks the model based on the task (debugging, refactoring, code review, etc.) and routes to the cheapest option that meets the task's requirements.

## Common Development Commands

### Setup
```bash
make setup   # Install dependencies (uv)
```

### Running the Server
```bash
make start              # Background HTTP on 0.0.0.0:8000
make stop               # Stop that instance
make serve              # Foreground HTTP on 127.0.0.1:8000
uv run uvicorn app.main:app --host 0.0.0.0 --port 8000  # optional dev server
```

### Testing
```bash
uv run pytest                    # Run all tests
uv run pytest tests/test_*.py   # Run tests matching a pattern
uv run pytest tests/test_engine.py -v  # Run a single test file with verbose output
uv run pytest tests/test_engine.py::test_function_name -v  # Run a specific test
```

### Code Quality
```bash
uv run black --check .  # Check formatting without changes
uv run black app/ tests/  # Auto-format code
uv run flake8 app/ tests/  # Lint code
```

All code must pass both `black` (line length: 100) and `flake8` (ignore E203, W503) before merging.

### Development Workflow
```bash
uv sync          # Install/update dependencies
uv run pytest -v  # Run tests after making changes
uv run black app/ tests/  # Auto-format before committing
```

## Architecture at a Glance

**Request flow**: Client → Adapter → Classifier → Router → Enrichment → Model Backend

```
app/
├── main.py              # FastAPI entry point
├── cli.py               # CLI (rex start/stop/reset)
├── config.py            # Pydantic config + YAML loader
├── utils.py             # Shared utilities
├── adapters/            # Client-specific request normalization
│   ├── base.py          # Adapter interface
│   ├── default.py       # Generic feature detection
│   └── registry.py      # Adapter selection by User-Agent
├── discovery/           # Model & provider detection
│   ├── providers.py     # Detect providers from env vars
│   ├── models.py        # Query provider APIs for models
│   ├── metadata.py      # Enrich models with LiteLLM metadata
│   └── registry_builder.py  # Orchestrate discovery
├── router/              # Classification and routing logic
│   ├── categories.py    # Task categories + requirements
│   ├── classifier.py    # Heuristic task classifier
│   ├── detector.py      # Feature detection (completion vs. chat)
│   ├── engine.py        # Routing engine + fallback chain
│   ├── llm_judge.py     # LLM-as-Judge classifier
│   ├── ml_classifier.py # Trained ML classifier
│   └── registry.py      # Model registry (lookups, filtering)
├── learning/            # ML training pipeline
│   ├── embeddings.py    # Sentence Transformer service
│   ├── centroids.py     # Centroid classifier + exemplars
│   ├── clustering.py    # K-means + silhouette scoring
│   ├── labeling.py      # Weak supervision label model
│   ├── trainer.py       # Training pipeline
│   └── scheduler.py     # Background retraining scheduler
├── enrichment/          # Request transformation
│   ├── context.py       # EnrichmentContext dataclass
│   ├── pipeline.py      # Enricher protocol + runner
│   └── task_decomposition.py  # Task decomposition enricher
├── logging/             # Decision logging
│   ├── models.py        # DecisionRecord dataclass
│   ├── repository.py    # Repository protocol
│   └── sqlite.py        # SQLite implementation
└── proxy/               # API endpoint handlers
    ├── handler.py       # OpenAI + Anthropic request handlers
    ├── anthropic.py     # Anthropic Messages API translator
    ├── message_sanitizer.py  # Mixed-format message conversion
    └── streaming.py     # SSE streaming logic
```

## Key Architectural Concepts

### Classification Chain (in order of execution)
1. **Heuristics** → Fast keyword/pattern matching on the last user message
2. **Centroid classifier** → Semantic similarity from sentence embeddings (from day 1, zero training data)
3. **ML classifier** → Trained logistic regression (takes over after ~100 queries at >0.5 silhouette)
4. **LLM judge** → Small local LLM (only for chat/agent, when confidence is low)

### Routing Strategy
- All tasks start on the **cheapest model** that meets the task's requirements
- **Task requirements** = set of criteria (cost, context window, `supports_reasoning` flag)
- On failure, escalate to the next more expensive model (fallback chain)
- **Confidence-based model escalation**: when classification confidence is low after the full chain, jump to a more capable model even if cheaper one technically qualifies

### Learning Pipeline (runs every ~100 queries)
1. **K-means clustering** on stored embeddings → discovers actual user task categories
2. **Weak supervision** → heuristic rules act as noisy labeling functions, label model aggregates them
3. **Training** → logistic regression on cluster-derived + weakly-supervised labels
4. **Promotion** → ML classifier replaces heuristics when silhouette > 0.5 and converged

### Data Flow
- Every request is embedded (sentence-transformers, ~10ms)
- Embeddings + heuristic votes are logged to SQLite
- Background scheduler periodically retrains the ML classifier
- All learning data is local to the user's machine

## Important Design Decisions

| Area | Approach | Why |
|------|----------|-----|
| Classification | Hybrid (heuristics → ML classifier → LLM judge) | Fast + free + evolves over time |
| Embeddings | sentence-transformers all-MiniLM-L6-v2 | ~80MB, ~10ms local, zero API cost |
| Clustering | Unsupervised K-means | Discovers categories from usage without labels |
| Labeling | Weak supervision | Heuristics as noisy functions, label model aggregates |
| Model metadata | LiteLLM built-in database | No manual config needed for known models |
| Routing criteria | Cost + context window + capability flags | All signals are measurable, no curated "strengths" |
| Config | Optional YAML overrides | Falls back to auto-discovery from env vars |
| Request timeouts | `asyncio.wait_for` + per-model override | `ServerConfig.timeout` (600s default), `ModelConfig.timeout` overrides per model; timeout → fallback → 504 |
| Storage | SQLite + repository pattern | Zero dependencies, single-file, easily testable |
| Deployment | Per-user local instance | CLI (`rex start`), all data stays on machine |

## Task Categories & Routing

The heuristic classifier recognizes these categories and routes to the cheapest model meeting requirements:

| Category | Signals | Requirements |
|---|---|---|
| **completion** | Short, tab completion | Cheapest, lowest latency |
| **debugging** | "error", "fix", "bug", "stack trace" | `supports_reasoning` |
| **refactoring** | "refactor", "clean", "simplify" | Context window ≥ 32K |
| **code_review** | "review", "security", "what's wrong" | Context ≥ 32K, `supports_reasoning` |
| **test_generation** | "test", "spec", "coverage" | Context window ≥ 16K |
| **documentation** | "document", "docstring", "README" | Context window ≥ 16K |
| **generation** | Writing new code from description | Context window ≥ 16K |
| **migration** | "upgrade", "migrate", "convert" | Context ≥ 32K, `supports_reasoning` |
| **optimization** | "faster", "performance", "memory" | `supports_reasoning` |
| **explanation** | "explain", "how does", "what is" | Cheapest |
| **general** | Fallback | Primary model |

## Testing Patterns

### Unit Testing Strategy
- Test individual components in isolation (classifier, embedder, router, etc.)
- Mock external dependencies (LiteLLM calls, model backends)
- Focus on decision logic, not infrastructure

### Integration Testing
- Test request → classification → routing → response flow
- Verify decision logging stores correct metadata
- Test fallback chains when primary model fails

### Common Test Fixtures
- `models_registry`: Mock model registry with test models
- `sample_request`: Basic OpenAI-format request
- `anthropic_request`: Anthropic-format request

Run tests frequently during development:
```bash
uv run pytest -v --tb=short  # Verbose with short tracebacks
```

## Config-First Model Registry

**Key principle**: `~/.rex/config.yaml` is the **primary** source of models. Auto-discovery supplements.

**Priority order**:
1. Config-defined models (if `models:` exists in config)
2. Config-defined providers (if `providers:` exists; remote LiteLLM proxies, custom endpoints)
3. Auto-discovered cloud providers (env vars: `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, etc.)
4. Auto-discovered local providers (Ollama)

**Config overrides**: A config provider with the same prefix replaces auto-discovery for that provider.

See `config.yaml.example` for syntax.

## Incremental PR Philosophy

Per `.cursor/rules/incremental-prs.mdc`:
- Break large features into small, focused PRs
- Each PR does one thing and reaches a working state
- Changes should build on each other incrementally
- Include tests in the same PR as the code change

Example breakdown for a new enricher:
1. PR 1: Add enricher protocol to pipeline
2. PR 2: Implement the new enricher
3. PR 3: Add config option + tests
4. PR 4: Add documentation

## API Endpoints

### Routed (classification + routing applied)
- `POST /v1/chat/completions` — OpenAI format (streaming, non-streaming)
- `POST /v1/messages` — Anthropic Messages API format (streaming, non-streaming)
- `POST /v1/completions` — Legacy completions

### Direct
- `GET /v1/models` — Returns models from Rex's registry
- `GET /health` — Proxy health status
- `POST /v1/reset` — Clear all learning data, restore to fresh state

### Passthrough (no routing)
- `/v1/embeddings`, `/v1/audio/*`, `/v1/images/*`, `/v1/files`, `/v1/moderations`, etc.

## Debugging Tips

### Viewing routing decisions
```bash
# Decision logs go to ~/.rex/decisions.db
# Query with:
sqlite3 ~/.rex/decisions.db "SELECT * FROM decisions ORDER BY created_at DESC LIMIT 10;"

# Or install Datasette for a dashboard:
pip install datasette datasette-dashboards
datasette ~/.rex/decisions.db --metadata rex-dashboards.yml
```

### Checking which model was selected
Every decision record includes:
- `selected_model`: Heuristically selected model
- `used_model`: Actually used model (after fallbacks)
- `category`: Detected task category
- `confidence`: Classification confidence (0.0 to 1.0)
- `escalated`: Whether confidence-based model escalation triggered

### Testing a specific request
```python
# From app/main.py, manually call:
from app.router.engine import RoutingEngine
engine = RoutingEngine(...)
result = await engine.route(normalized_request)
print(f"Category: {result.category}")
print(f"Model: {result.selected_model}")
print(f"Confidence: {result.confidence}")
```

## Development Checklist

Before committing:
1. ✅ Run `uv run pytest -v` — all tests pass
2. ✅ Run `uv run black app/ tests/` — auto-format code
3. ✅ Run `uv run flake8 app/ tests/` — no lint errors
4. ✅ Add tests for new code (unit + integration)
5. ✅ Update docs if architecture changes

## Documentation Structure

- **README.md** — Quick start, features, tech stack, CLI usage
- **ARCHITECTURE.md** — System design, routing strategy, learning pipeline, design decisions
- **ROADMAP.md** — Phased delivery plan, implementation status
- **docs/specs/** — Detailed design docs for each phase

When adding a feature, consider updating both ROADMAP.md (if it affects delivery plan) and ARCHITECTURE.md (if it affects system design).

## External Dependencies

**Core**:
- `fastapi` — Web framework
- `uvicorn` — ASGI server
- `litellm` — Unified interface to 100+ model providers
- `httpx` — HTTP client for transparent passthrough
- `pyyaml` — Config parsing

**ML**:
- `numpy` — Numerical computing
- `scikit-learn` — K-means clustering, logistic regression, silhouette scoring
- `sentence-transformers` — Query embeddings (optional, lazy-loaded)

**Dev**:
- `pytest`, `pytest-asyncio` — Testing
- `black` — Code formatting
- `flake8` — Linting

All dependencies are declared in `pyproject.toml`. Add new deps there, then run `uv sync`.

## Where to Look

**Integrating a new model provider?** → `discovery/metadata.py`, `router/registry.py`

**Adding a new task category?** → `router/categories.py`, `router/classifier.py`

**Changing enrichment logic?** → `enrichment/pipeline.py`, `enrichment/task_decomposition.py`

**Improving classification?** → `router/classifier.py` (heuristics), `learning/centroids.py` (semantic), `learning/trainer.py` (ML)

**Debugging routing decisions?** → `logging/sqlite.py`, `router/engine.py`

**Changing timeout behavior?** → `config.py` (`ServerConfig.timeout`, `ModelConfig.timeout`), `proxy/handler.py` (`_resolve_timeout`, `_call_with_fallback`)

**Adding a new client adapter?** → `adapters/default.py`, `adapters/registry.py`
