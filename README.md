# Rex

> This project only exists because I'm too lazy to pick the best model myself.

An OpenAI-compatible proxy that sits between AI-powered coding tools and multiple model backends (local + cloud). Rex identifies what each coding task needs and routes it to the cheapest model that fits.

- Works with any tool that supports a custom OpenAI API base URL (Cursor, Claude Code, Continue, Aider, etc.).
- Each user runs their own Rex instance locally — all data, embeddings, and trained classifiers stay on the user's machine.
- The ML classifier personalizes to each user's coding patterns over time.

## Key Features

- **Zero-config model discovery**: Rex detects available providers from environment variables and local runtimes (Ollama) — no config file needed.
- **Task-aware routing** (planned): Classifies each prompt by task type (debugging, refactoring, code generation, etc.) and routes to the cheapest model with the right capabilities (context window, reasoning, function calling).
- **Cost-first default**: Until the task classifier is trained, all tasks route to the cheapest available model (local first, then cheapest cloud).
- **Measurable routing criteria**: Routes based on cost, context window, and LiteLLM capability flags (`supports_reasoning`, `supports_function_calling`, etc.) — no manually curated "strengths" list.
- **Hybrid classification** (planned): Combines fast keyword heuristics, a local ML classifier, and an LLM-as-Judge fallback for accurate task detection.
- **Self-improving** (planned): A learning pipeline trains the classifier automatically from usage data — no manual labeling required.
- **Local-first**: All data, embeddings, and models run on the user's machine. Nothing leaves without explicit cloud API configuration.
- **Fallback chains**: If the primary model fails, Rex tries the next model in cost order.

## How It Works

1. Rex **discovers** available models from environment variables, local runtimes (Ollama), and provider APIs — then merges with any `config.yaml` overrides.
2. The **feature detector** identifies whether the request is a tab completion or a chat/agent interaction.
3. The **routing engine** selects the cheapest model that meets the task's requirements, or a config override.
4. The **proxy** forwards the request to the selected backend and streams the response back.
5. If the primary fails, the **fallback chain** tries remaining models in cost order.

For full system diagrams and design decisions, see [ARCHITECTURE.md](ARCHITECTURE.md).

## Current Status

Rex has completed **Phase 0 — Proxy + Basic Routing**. The proxy accepts requests, detects feature type (completion vs. chat), routes to the primary model, and falls back on failure.

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
    registry_builder.py  # Orchestrates discovery and builds the model registry
  router/
    detector.py          # Feature detection (completion vs. chat)
    engine.py            # Routing engine (primary selection + fallback)
    registry.py          # Model registry (lookups, cost sorting)
  proxy/
    handler.py           # OpenAI-compatible request handler
    streaming.py         # SSE streaming response logic
config.yaml.example     # Example configuration (optional)
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
