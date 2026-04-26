# Prioritization (Rex)

Rex is a **small study project** (see [README.md](../README.md)): a hands-on look at local AI infrastructure (daemon, gRPC over UDS, thin clients, streaming contracts). This document gives a **lightweight** way to **order** work and to **park** deferred items, using clear buckets and a short score.

Use [ROADMAP.md](ROADMAP.md) for a **Now / Next / Later** view. Deeper lists live in [MVP_SPEC.md](MVP_SPEC.md), [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md), and [EXTENSION_ROADMAP.md](EXTENSION_ROADMAP.md). The **`REX_*` catalog** is in [CONFIGURATION.md](CONFIGURATION.md).

## When to use this

- **Bucketing** (Must/Should/Could/Won’t) and a **quick** R-ICE pass to compare two **Should**-level items → this file, with [ROADMAP.md](ROADMAP.md) for the one-page list.
- **Choosing what to learn or build next** in the near term → this file and the roadmap, aligned with the linked design docs.
- **Environment variable names, defaults, and configuration semantics** → [CONFIGURATION.md](CONFIGURATION.md).
- **A small, focused change** (for example one crate, one test) → ship the fix; a full pass through this file is **optional** unless the change shifts priorities.

## MoSCoW (horizon filter)

| Bucket | Definition for this repo |
|--------|--------------------------|
| **Must** | Breaks the contract, safety, or CI, or blocks the main learning loop (daemon + UDS + streaming + NDJSON). |
| **Should** | Clear **learning** or “local demo” value; risk is manageable; fits [ARCHITECTURE.md](ARCHITECTURE.md) and [ADAPTERS.md](ADAPTERS.md). |
| **Could** | A solid next step after current items; more work or groundwork first. |
| **Won’t (now)** | **Parked** for this period. Add a **pointer** in a design doc so the idea can be picked up later with context (see [DOCUMENTATION.md](DOCUMENTATION.md) for how we write scope). |

## R-ICE (quick rank inside a bucket)

Optional. Three scores **1–5** (5 is best). Use this to break ties between two **Should** items.

| Dimension | 5 | 1 |
|----------|---|-----|
| **Impact** | Teaches something important *or* clearly improves the local demo in [README.md](../README.md) | Niche, little learning value |
| **Confidence** | Matches current docs and code seams; unknowns are small | Speculative or in tension with the current design |
| **Ease** | **Small, reviewable** change you can finish and learn from | Unbounded or touches every layer |

**Optional:** multiply the three numbers for a tie-breaker. Treat the result as a **rough hint** for ordering.

## Tie-breakers (Rex-specific)

| Signal | How to use it |
|--------|----------------|
| **Contract risk** (low / medium / high) | Touches NDJSON, gRPC, or proto. When risk is high and payoff unclear, lean toward **Should** or **Could** (or **Won’t**) instead of **Must**. |
| **CI cost** | Favors approaches that keep automation simple: see [CI.md](CI.md) and [DEPENDENCIES.md](DEPENDENCIES.md) for what runs on every change. |
| **Local-first leverage** | Prefer work that increases the share of tasks solved by local/open runtimes before escalation, while preserving terminal correctness and debuggability. |
| **Blast radius** | Often: daemon only → +CLI → +extension. A short note is enough. |

**Example:** medium learning value, high contract risk, migration still undefined → **Should** or **Could**.

## How often to touch this

- Update the [ROADMAP](ROADMAP.md) when you **change focus**, finish a piece of work that shifts priorities, or reach a **learning milestone** (for example: streaming behavior feels solid, next interest is adapters).
- When **contract, security, or CI** policy changes, update the relevant doc; adjust the roadmap in the same change if priorities move.

## Scope of this file

- Covers **bucketing** and a **light** R-ICE pass. The consolidated [ROADMAP.md](ROADMAP.md) stays short; long design stays in the linked files.
- For configuration keys and server behavior details, use [CONFIGURATION.md](CONFIGURATION.md) and the feature-specific design docs.

## Evidence cues for optimization work

Use these as short confidence cues when ranking optimization proposals:

- Adaptive retrieval and retrieve-on-demand behavior can reduce unnecessary context loading on simple queries ([Self-RAG](https://arxiv.org/abs/2310.11511)).
- Query-aware compression can reduce prompt size while preserving useful task signal when tuned and evaluated ([LLMLingua](https://arxiv.org/abs/2310.05736)).
- Long prompts remain position-sensitive; high-value context packing and ordering matter for quality ([Lost in the Middle](https://aclanthology.org/2024.tacl-1.9/)).
- Routing/cascades provide a practical quality-cost tradeoff by escalating only harder requests ([A Unified Approach to Routing and Cascading for LLMs](https://proceedings.mlr.press/v267/dekoninck25a.html)).

## Related

- [ROADMAP.md](ROADMAP.md) — one-page study roadmap
- [DOCUMENTATION.md](DOCUMENTATION.md) — how we write project docs
- [CI.md](CI.md) and [DEPENDENCIES.md](DEPENDENCIES.md) — what stays green; optional local tools
