# Rex

An OpenAI-compatible proxy that sits between AI-powered coding tools and multiple model backends (local + cloud), automatically selecting the best model for each coding task.

- Works with any tool that supports a custom OpenAI API base URL (Cursor, Claude Code, Continue, Aider, etc.).
- Each user runs their own Rex instance locally — all data, embeddings, and trained classifiers stay on the user's machine.
- The ML classifier personalizes to each user's coding patterns over time.

## Key Features

- **Zero-config**: Rex discovers available models automatically from environment variables and local runtimes — no config file needed.
- **Smart routing**: Classifies each prompt by task type (debugging, refactoring, code generation, etc.) and routes to the best-suited model.
- **Hybrid classification**: Combines fast keyword heuristics, a local ML classifier, and an LLM-as-Judge fallback for accurate task detection.
- **Self-improving**: A learning pipeline trains the classifier automatically from usage data — no manual labeling required.
- **Local-first**: All data, embeddings, and models run on the user's machine. Nothing leaves without explicit cloud API configuration.
- **Cost-first**: All tasks start on the cheapest model. Rex tracks actual cost per request via LiteLLM's built-in pricing database — no manual cost configuration needed.
- **Fallback chains**: If the primary model fails, Rex escalates to the next model up the cost ladder. The learning pipeline permanently promotes task categories that consistently need more capable models.

## How It Works

1. The **client adapter** normalizes the incoming request from the AI coding tool into a common format.
2. The **task classifier** identifies what kind of coding task the prompt represents (completion, debugging, refactoring, etc.).
3. The **routing engine** selects the best model from the registry based on the task category and model capabilities.
4. The **proxy** forwards the request to the selected backend and streams the response back.
5. The **learning pipeline** stores query embeddings and classifier decisions, retraining the ML model in the background.

For full system diagrams and design decisions, see [ARCHITECTURE.md](ARCHITECTURE.md).

## Current Status

Rex has completed **Phase 0 — Proxy + Basic Routing**. The proxy accepts requests, detects feature type (completion vs. chat), routes to the appropriate model, and falls back on failure.

- See [ROADMAP.md](ROADMAP.md) for the phased delivery plan (Phase 0 through Phase 4).
- See [ARCHITECTURE.md](ARCHITECTURE.md) for system design, routing strategy, and the learning pipeline.

## Planned Tech Stack

| Component | Technology |
|---|---|
| Language | Python |
| API framework | FastAPI |
| Model backends | LiteLLM (supports 100+ providers) |
| Query embeddings | Sentence Transformers ([all-MiniLM-L6-v2](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2)) |
| Clustering | K-means with silhouette score for optimal cluster count |
| Label generation | Weak supervision ([Ratner et al., 2017](https://arxiv.org/abs/1711.10160)) |
| ML classifier | Logistic regression |
| Configuration | Optional YAML overrides |
| Storage | SQLite (default, swappable via repository pattern) |

## Project Structure

```
app/
  main.py                # FastAPI app entry point
  config.py              # Pydantic settings model + optional YAML loader
  discovery/
    providers.py         # Detects available providers from env vars
    models.py            # Queries provider APIs for available models
    metadata.py          # Enriches models with LiteLLM metadata
  router/
    detector.py          # Feature detection (completion vs. chat)
    engine.py            # Routing engine (detector -> model selection + fallback)
    registry.py          # Model registry
  proxy/
    handler.py           # OpenAI-compatible request handler
    streaming.py         # SSE streaming response logic
config.yaml.example     # Example configuration (optional overrides)
pyproject.toml           # Project dependencies (uv)
tests/                   # pytest test suite
```

Future phases will add:
- `app/adapters/` — client adapter interface (Cursor, Claude Code, etc.)
- `app/router/classifier.py` — heuristic task classifier
- `app/router/ml_classifier.py` — trained ML classifier
- `app/router/llm_judge.py` — LLM-as-Judge fallback
- `app/learning/` — embedding pipeline, clustering, weak supervision
- `app/logging/` — decision logging with storage interface

## Getting Started

### Quick Setup

```bash
./setup.sh
```

The script installs dependencies and shows how to start Rex.

### Manual Setup

1. Install [uv](https://docs.astral.sh/uv/):
   ```bash
   curl -LsSf https://astral.sh/uv/install.sh | sh
   ```
2. Install dependencies:
   ```bash
   uv sync
   ```
3. Set at least one provider API key:
   ```bash
   export OPENAI_API_KEY="sk-..."
   ```
4. Start the Rex proxy:
   ```bash
   uv run uvicorn app.main:app --host 0.0.0.0 --port 8000
   ```
5. Point your AI coding tool's base URL to `http://localhost:8000/v1`.

Rex discovers available models automatically from environment variables and local runtimes. No config file needed. See `config.yaml.example` for optional overrides.

## Documentation

| Document | Description |
|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | System design, routing strategy, learning pipeline, design decisions |
| [ROADMAP.md](ROADMAP.md) | Phased delivery plan, risks, success metrics |
| [docs/specs/](docs/specs/) | Detailed design specs for each phase |
