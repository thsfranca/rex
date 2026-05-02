# Purpose and principles

This document is the **single source of truth** for **why REX exists** and **how contributors scope decisions**. Other docs link here instead of duplicating full intent lists.

## Purpose

REX is a **local AI runtime** for macOS (Apple Silicon): a Rust **`rex-daemon`** owns inference and stream lifecycle; **thin clients** (CLI, editor extension, scripts) use **gRPC on a Unix domain socket** (`rex.v1`). The work favors a **daemon-owned economics surface** (routing hooks, caches, pipelines, observability) described in [ARCHITECTURE.md](ARCHITECTURE.md), [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md), and [ADR 0001](architecture/decisions/0001-daemon-owns-agent-orchestration-and-economics.md).

Concrete goals:

- **Single runtime boundary.** One long-lived process holds model/runtime policy, queueing, and shutdown semantics so every client sees the same behavior.
- **Stable tool contract.** Clients integrate through **`rex-cli`** and shared protobuf types in `rex-proto` instead of embedding vendor inference SDKs.
- **Streaming-first correctness.** Server-streaming RPCs, explicit terminal outcomes (`done` / `error`), and tests around UDS races and interruption.
- **Local-first default.** **Mock** inference without network by default; optional adapters (for example Cursor CLI) remain **bounded subprocesses** under daemon policy—see [ADAPTERS.md](ADAPTERS.md).
- **Room to grow.** Additional adapters and optional isolated runtimes evolve **without breaking** `rex.v1` consumers—see [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md).

**Configuration:** precedence and `REX_*` catalog live in [CONFIGURATION.md](CONFIGURATION.md).

**Audience:** engineers studying daemon-hosted inference, gRPC streaming over UDS, and editor integration; anyone building toward a **local-first** assistant on Mac.

## Principles

1. **Study and experimentation.** This repository is a **learning lab** and **experimental** codebase: APIs, docs, and behavior may change. It is **not** offered as a finished product, supported distribution, or production SLA. Use it for learning and prototypes.

2. **Daemon-first economics and contracts.** Stream semantics, modes, caches, capability-aware pipelines, and adapter selection/configuration converge in **`rex-daemon`** ([ADR 0001](architecture/decisions/0001-daemon-owns-agent-orchestration-and-economics.md)). Inference backends produce output **under** that envelope.

3. **Incremental implementation.** Ship narrow, testable slices; separate **intent** from **shipping state** using [MVP_SPEC.md](MVP_SPEC.md), [ROADMAP.md](ROADMAP.md), and MoSCoW-style deferrals in [PRIORITIZATION.md](PRIORITIZATION.md). Do not describe future work as shipped.

4. **Stable external contract.** Preserve **`rex.v1`** and deterministic **`rex-cli --format ndjson`** behavior for integrations unless a versioned migration is intentional.

5. **Documentation hubs.** Keep **one** canonical explanation document per major feature area under `docs/`; use **links** plus one-line status elsewhere ([DOCUMENTATION.md](DOCUMENTATION.md)). Record accepted boundaries in **ADRs** when formalizing decisions.

6. **Isolation and sidecars when justified.** Prefer logic in the daemon until **process isolation**, another runtime stack, or **fault containment** clearly outweighs integration cost ([PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md)). Optional isolated agent environments are **additive**, not a rewrite of core economics.

7. **Neutral, operational naming.** Prefer function-based headings and scope descriptions in docs and plans; avoid framing whose main job is to label maturity ([DOCUMENTATION.md](DOCUMENTATION.md) norms).

## How to use this document

- **Authors:** When adding or rewriting **purpose-level** prose, **update this file** and replace long repeats elsewhere with **links** here.
- **Readers:** Start here for intent; follow links for specs, roadmaps, and ADRs.
