# Rex

An OpenAI-compatible proxy that sits between AI-powered coding tools and multiple model backends (local + cloud), automatically selecting the best model for each coding task.

- Works with any tool that supports a custom OpenAI API base URL (Cursor, Claude Code, Continue, Aider, etc.).
- Each user runs their own Rex instance locally — all data, embeddings, and trained classifiers stay on the user's machine.
- The ML classifier personalizes to each user's coding patterns over time.

## Key Features

- **Smart routing**: Classifies each prompt by task type (debugging, refactoring, code generation, etc.) and routes to the best-suited model.
- **Hybrid classification**: Combines fast keyword heuristics, a local ML classifier, and an LLM-as-Judge fallback for accurate task detection.
- **Self-improving**: A learning pipeline trains the classifier automatically from usage data — no manual labeling required.
- **Local-first**: All data, embeddings, and models run on the user's machine. Nothing leaves without explicit cloud API configuration.
- **Cost-aware**: Tracks actual cost per request via LiteLLM's built-in pricing database — no manual cost configuration needed. Routes cheaper tasks to cheaper models.
- **Fallback chains**: If the primary model fails or times out, the router tries the next best option.

## How It Works

1. The **client adapter** normalizes the incoming request from the AI coding tool into a common format.
2. The **task classifier** identifies what kind of coding task the prompt represents (completion, debugging, refactoring, etc.).
3. The **routing engine** selects the best model from the registry based on the task category and model capabilities.
4. The **proxy** forwards the request to the selected backend and streams the response back.
5. The **learning pipeline** stores query embeddings and classifier decisions, retraining the ML model in the background.

For full system diagrams and design decisions, see [ARCHITECTURE.md](ARCHITECTURE.md).

## Current Status

Rex is in the **design and planning phase**. The architecture and roadmap are defined, but implementation has not started yet.

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
| Configuration | YAML |
| Storage | SQLite (default, swappable via repository pattern) |

## Project Structure

```
rex/
  main.py                # FastAPI app entry point
  config.yaml            # Model registry + routing config
  adapters/
    base.py              # Client adapter interface
    cursor.py            # Cursor-specific request normalization
    generic.py           # Fallback adapter for unknown clients
  router/
    detector.py          # Feature detection (completion vs. chat)
    classifier.py        # Heuristic task classifier
    ml_classifier.py     # Trained ML classifier
    llm_judge.py         # LLM-as-Judge fallback
    engine.py            # Routing engine (classifier -> model selection)
    registry.py          # Model registry loader
  learning/
    embedder.py          # Sentence transformer query embedding
    seeds.py             # Synthetic exemplar queries per category
    clustering.py        # K-means unsupervised category discovery
    weak_supervision.py  # Noisy label aggregation from heuristic rules
    trainer.py           # ML classifier training orchestration
  proxy/
    handler.py           # OpenAI-compatible request handler
    streaming.py         # SSE streaming response logic
  logging/
    base.py              # Storage interface
    sqlite_store.py      # SQLite implementation
    cli.py               # CLI for stats and export
  requirements.txt
  README.md
  ROADMAP.md
```

## Getting Started

> [!NOTE]
> Setup instructions will be available once Phase 0 (Proxy + Basic Routing) is complete. See [ROADMAP.md](ROADMAP.md) for progress.

The general flow will be:

1. Install Python dependencies.
2. Configure `config.yaml` with model backends and API keys.
3. Start the Rex proxy server.
4. Point the AI coding tool's base URL to the Rex instance.

## Documentation

| Document | Description |
|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | System design, routing strategy, learning pipeline, design decisions |
| [ROADMAP.md](ROADMAP.md) | Phased delivery plan, risks, success metrics |
| [docs/specs/](docs/specs/) | Detailed design specs for each phase |
