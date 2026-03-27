# Rex — Architecture

An OpenAI-compatible proxy that sits between Cursor IDE and multiple AI model backends (local + cloud), automatically selecting the best model for each coding task.

## System Overview

```mermaid
flowchart LR
    Cursor[Cursor IDE] -->|OpenAI API| Proxy[Rex Proxy]
    Proxy --> Classifier{Task Classifier}
    Classifier -->|fast path| Heuristics[Heuristic Rules]
    Classifier -->|uncertain| LLMJudge[LLM-as-Judge]
    Heuristics --> Router[Routing Engine]
    LLMJudge --> Router
    Router --> Registry[Model Registry]
    Registry --> Local[Ollama Local Models]
    Registry --> Cloud[Cloud APIs]
    Router --> Logger[Decision Logger]
    Logger --> DataStore[SQLite]
```

## Design Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Language | Python + FastAPI | Fastest to prototype, async-native, rich AI ecosystem |
| Model backends | LiteLLM as library | Handles 100+ providers with unified interface |
| Local models | Ollama | Easiest way to run open-source models locally |
| Classification | Hybrid (heuristics + LLM judge) | Heuristics are fast and free; LLM judge catches edge cases |
| Config format | YAML | Human-readable, easy to edit |
| Logging | SQLite | Zero-dependency, single-file, good enough for single user |

## Task Categories

The classifier maps each incoming prompt into one of these categories. Each category has different model requirements:

| Category | Signals | Model Needs |
|---|---|---|
| **completion** | Short prompt, code context, single-turn | Fastest model, latency < 100ms |
| **debugging** | Stack traces, "error", "fix", "bug" | Strong reasoning model |
| **refactoring** | "refactor", "clean up", "simplify" | Large context window |
| **test_generation** | "write tests", "add test", "spec" | Good instruction-following model |
| **explanation** | "explain", "what does", "how does" | Any decent model, optimize cost |
| **generation** | Writing new code from description | Strong coding model |
| **general** | Fallback when nothing else matches | Default model |

## Routing Strategy

```mermaid
flowchart TD
    Request[Incoming Request from Cursor] --> FeatureDetect{Cursor Feature Detection}
    FeatureDetect -->|Tab Completion| FastPath[Always use fastest model]
    FeatureDetect -->|Chat / Agent| HeuristicAnalysis[Heuristic Analysis]
    HeuristicAnalysis --> ConfidenceCheck{Confidence above threshold?}
    ConfidenceCheck -->|Yes| RouteDirectly[Route to best model for category]
    ConfidenceCheck -->|No| LLMJudge[LLM-as-Judge classification]
    LLMJudge --> RouteClassified[Route based on judge output]
    FastPath --> Log[Log decision to SQLite]
    RouteDirectly --> Log
    RouteClassified --> Log
    Log --> FeedbackData[Accumulate labeled data for future ML classifier]
```

**Fast path**: Tab completion requests skip classification entirely — always use the fastest available model.

**Heuristic path**: Analyze the prompt with keyword matching, pattern detection, and structural analysis. If the confidence score is high enough, route immediately (<1ms overhead).

**LLM judge fallback**: When heuristics are uncertain, use a small local model to classify the task. Only triggered for chat/agent requests where 200-500ms extra latency is acceptable.

## Project Structure

```
rex/
  main.py                # FastAPI app entry point
  config.yaml            # Model registry + routing config
  router/
    classifier.py        # Heuristic task classifier
    llm_judge.py         # LLM-as-judge fallback
    engine.py            # Routing engine (classifier -> model selection)
    registry.py          # Model registry loader
  proxy/
    handler.py           # OpenAI-compatible request handler
    streaming.py         # SSE streaming response logic
  logging/
    store.py             # SQLite decision logging
    cli.py               # CLI for stats, review, and labeling
  requirements.txt
  README.md
  ROADMAP.md
```
