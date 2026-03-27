# Rex — Architecture

For a project overview and getting started guide, see [README.md](README.md). For the delivery plan, see [ROADMAP.md](ROADMAP.md).

An OpenAI-compatible proxy that sits between AI-powered coding tools and multiple model backends (local + cloud), automatically selecting the best model for each coding task.

- Compatible with any tool that supports a custom OpenAI API base URL (Cursor, Claude Code, Continue, Aider, etc.).
- Each user runs their own Rex instance locally — all data, embeddings, and trained classifiers stay on the user's machine.
- The ML classifier personalizes to each user's coding patterns over time.

## System Overview

```mermaid
flowchart LR
    Client[AI Coding Tool] -->|OpenAI API| Proxy[Rex Proxy]
    Proxy --> Adapter[Client Adapter]
    Adapter --> Classifier{Task Classifier}
    Classifier -->|fast path| Heuristics[Heuristic Rules]
    Classifier -->|trained| MLClassifier[ML Classifier]
    Classifier -->|uncertain| LLMJudge[LLM-as-Judge]
    Heuristics --> Router[Routing Engine]
    MLClassifier --> Router
    LLMJudge --> Router
    Router --> Registry[Model Registry]
    Registry --> Local[Local Models]
    Registry --> Cloud[Cloud APIs]
    Router --> Logger[Decision Logger]
    Logger --> Store[Storage Interface]
    Store --> DataStore[SQLite]
    Adapter --> Embedder[Sentence Transformer]
    Embedder --> EmbeddingStore[Embedding Store]
    EmbeddingStore --> LearningPipeline[Learning Pipeline]
    Heuristics --> LearningPipeline
    LearningPipeline --> MLClassifier
```

- The **Client Adapter** normalizes tool-specific request patterns into a common format for the classifier.
- Each supported tool (Cursor, Claude Code, etc.) has its own adapter that detects features like tab completion vs. chat.
- The **Learning Pipeline** runs in the background, consuming query embeddings and heuristic votes to train the ML classifier automatically.

## Design Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Language | Python + FastAPI | Fastest to prototype, async-native, rich AI ecosystem |
| Model backends | LiteLLM as library | Handles 100+ providers (local and cloud) with unified interface |
| Classification | Hybrid (heuristics → ML classifier → LLM judge) | Heuristics are fast and free; ML classifier replaces heuristics once trained; LLM judge catches edge cases |
| Query embeddings | Sentence Transformer ([all-MiniLM-L6-v2](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2)) | ~80MB local model, ~10ms per query on CPU, zero API cost; produces vectors for clustering and classification ([Reimers & Gurevych, 2019](https://arxiv.org/abs/1908.10084); [Wang et al., 2020](https://arxiv.org/abs/2002.10957)) |
| Category discovery | Unsupervised K-means clustering | Automatically discovers task categories from query embeddings without labels; [silhouette score](https://doi.org/10.1016/0377-0427(87)90125-7) selects optimal cluster count (Rousseeuw, 1987) |
| Automated labeling | Weak supervision | Heuristic rules act as noisy labeling functions; a probabilistic label model aggregates their votes into clean training labels without manual annotation ([Ratner et al., 2016](https://arxiv.org/abs/1605.07723); [Ratner et al., 2017](https://arxiv.org/abs/1711.10160)) |
| Config format | YAML | Human-readable, easy to edit |
| Client detection | User-Agent header → adapter | Rex selects the adapter based on the client's User-Agent header; new tools supported by adding an adapter |
| API compatibility | Full OpenAI, transparent proxy | Rex routes known endpoints and passes through everything else to the default backend; never blocks unknown endpoints |
| Error handling | Graceful degradation | Every failure falls back to a simpler path; classification failure → default model; all models fail → error to client |
| Logging storage | Repository pattern | Core logic decoupled from storage; SQLite as default implementation, swappable without touching routing code |
| Deployment model | Per-user local instance | All data stays on the user's machine; each instance learns independently from its own usage |
| Default storage | SQLite | Zero-dependency, single-file, good enough for single user |
| Cost tracking | LiteLLM runtime cost calculation | LiteLLM's `completion_cost()` returns actual cost per request from its built-in pricing database; no manual cost config needed for known models; optional YAML override for custom endpoints or local models |

## Task Categories

The heuristic classifier uses these predefined categories as a starting point:

| Category | Signals | Model Needs |
|---|---|---|
| **completion** | Short prompt, code context, single-turn | Fastest model, latency < 100ms |
| **debugging** | Stack traces, "error", "fix", "bug", "crash" | Strong reasoning model |
| **refactoring** | "refactor", "clean up", "simplify", "restructure" | Large context window |
| **optimization** | "faster", "performance", "optimize", "memory", "efficient" | Strong reasoning + code analysis |
| **test_generation** | "write tests", "add test", "spec", "coverage" | Good instruction-following model |
| **explanation** | "explain", "what does", "how does", "why" | Any decent model, optimize cost |
| **documentation** | "document", "docstring", "README", "API docs" | Any decent model, optimize cost |
| **code_review** | "review", "is this correct", "what's wrong", "security" | Strong analysis model |
| **generation** | Writing new code from description | Strong coding model |
| **migration** | "upgrade", "migrate", "convert to", "update from" | Cloud model (needs current knowledge) |
| **general** | Fallback when nothing else matches | Default model |

- Once clustering produces a silhouette score above the quality threshold (>0.5), unsupervised clustering takes over.
- The learning pipeline discovers the user's actual task categories from their real usage patterns.
- Discovered categories may differ from the predefined ones — they reflect how the individual user actually works.

## API Surface

Rex exposes a fully OpenAI-compatible API as a transparent proxy:

- **Routed endpoints** — Rex applies classification and routing logic:
  - `POST /v1/chat/completions` (streaming and non-streaming)
  - `POST /v1/completions` (legacy)
- **Handled directly**:
  - `GET /v1/models` — returns models from Rex's registry
  - `GET /health` — returns proxy status
- **Transparent passthrough** — Rex forwards to the default backend without routing:
  - `/v1/embeddings`, `/v1/audio/*`, `/v1/images/*`, `/v1/files`, `/v1/moderations`, and any other endpoint
  - Rex never blocks an unknown endpoint — it passes it through to the default model backend

## Routing Strategy

```mermaid
flowchart TD
    Request[Incoming Request] --> ClientAdapter[Client Adapter]
    ClientAdapter --> EndpointCheck{Routed Endpoint?}
    EndpointCheck -->|No| Passthrough[Forward to default backend]
    EndpointCheck -->|Yes| FeatureDetect{Feature Detection}
    FeatureDetect -->|Completion| FastPath[Always use fastest model]
    FeatureDetect -->|Chat / Agent| ClassifierChain{Classifier Chain}
    ClassifierChain -->|heuristics confident| RouteDirectly[Route to best model for category]
    ClassifierChain -->|ML model available| MLClassify[ML Classifier]
    ClassifierChain -->|still uncertain| LLMJudge[LLM-as-Judge classification]
    MLClassify --> RouteClassified[Route based on classification]
    LLMJudge --> RouteClassified
    FastPath --> Log[Log decision]
    RouteDirectly --> Log
    RouteClassified --> Log
    Log --> FeedbackData[Accumulate labeled data for ML training]
```

- **Client adapter**: Normalizes the incoming request from a specific tool into a common format. Detects features (completion vs. chat/agent) based on tool-specific request patterns.
- **Fast path**: Completion requests skip classification — the router always selects the fastest available model.

**Classifier chain** (the router evaluates in order, stops at the first confident result):

1. **Heuristics**: Keyword matching, pattern detection, structural analysis.
   - The router routes immediately if confidence is high (<1ms overhead).
   - Heuristic rules also serve as labeling functions for the learning pipeline.
2. **ML classifier**: Classifies by nearest pre-seeded centroid in embedding space from day 1.
   - Evolves to logistic regression trained on cluster-derived and weakly-supervised labels once clusters stabilize.
   - Runs locally, <50ms inference.
3. **LLM judge**: A small local LLM classifies the task when the above are uncertain ([Zheng et al., 2023](https://arxiv.org/abs/2306.05685)).
   - Only triggered for chat/agent requests where 200-500ms extra latency is acceptable.

## Learning Pipeline

The learning pipeline trains the ML classifier automatically in the background, without manual labeling.

```mermaid
flowchart TD
    subgraph seed [Initialization]
        Exemplars[Synthetic Exemplars] --> SeedEmbed["Embed seed queries"]
        SeedEmbed --> InitCentroids[Initial Cluster Centroids]
    end

    subgraph perRequest [Per Request]
        Query[User Query] --> Embed["Sentence Transformer (~10ms)"]
        Embed --> NearestCentroid[Nearest Centroid Classification]
        InitCentroids --> NearestCentroid
        Query --> Rules[Heuristic Rules]
    end

    subgraph background [Background - Periodic]
        StoredEmb[Stored Embeddings] --> Cluster["K-means Clustering\n(discovers task categories)"]
        Cluster --> Categories[Discovered Categories]
        RuleVotes[Heuristic Rule Votes] --> WeakSup["Weak Supervision Label Model\n(aggregates noisy votes)"]
        WeakSup --> Labels[Probabilistic Labels]
        Categories --> Train[Train Classifier]
        Labels --> Train
        Train --> Updated[Updated ML Classifier]
    end

    Embed --> StoredEmb
    Rules --> RuleVotes
    Categories -->|update| InitCentroids
    Updated -->|replaces heuristics| Rules
```

**Initialization**:
- The system embeds synthetic exemplar queries for each predefined category (e.g., "fix this null pointer exception" → debugging).
- These embeddings serve as initial cluster centroids, enabling semantic classification from the first query ([Kushnareva et al., 2025](https://arxiv.org/abs/2601.09692)).

**Per request**:
- The sentence transformer embeds each query (~10ms, local CPU).
- The router classifies by nearest centroid in embedding space.
- This catches semantic similarity that keyword heuristics miss (e.g., "fix this crash" matches the debugging cluster without the word "error").
- The system stores the embedding and heuristic rule votes for the background pipeline.

**Periodically (~every 100 queries)**:
1. **Clustering**: K-means re-groups stored embeddings.
   - Centroids shift to reflect actual usage patterns.
   - New categories can emerge beyond the predefined set.
   - [Silhouette score](https://doi.org/10.1016/0377-0427(87)90125-7) determines optimal cluster count and whether clusters are stable enough to promote (threshold: >0.5) (Rousseeuw, 1987).
   - Unsupervised clustering on query embeddings can match oracle-level routing accuracy ([Neurometric, 2026](https://neurometric.substack.com/p/unsupervised-llm-routing-matching)).
2. **Weak supervision**: Heuristic rules act as noisy labeling functions.
   - A probabilistic label model learns each rule's reliability from agreement/disagreement patterns.
   - The model produces clean probabilistic labels without any ground-truth labels ([Ratner et al., 2017](https://arxiv.org/abs/1711.10160)).
3. **Training**: A lightweight classifier (logistic regression) trains on the cluster-derived and weakly-supervised labels.
   - Once trained, it replaces heuristics as the primary classifier in the chain.

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
