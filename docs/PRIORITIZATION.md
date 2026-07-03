# Prioritization (Rex)

Rex is a **small study project** (see [README.md](../README.md)): a hands-on look at local AI infrastructure (daemon, gRPC over UDS, thin clients, streaming contracts). This document gives a **lightweight** way to **order** work and to **park** deferred items, using clear buckets and a short score.

Use [ROADMAP.md](ROADMAP.md) for a **Now / Next / Later** view toward **[V1_0.md](V1_0.md)**. Deeper lists live in [MVP_SPEC.md](MVP_SPEC.md), [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md), [ROADMAP.md](ROADMAP.md), and [LANGFUSE_DISCOVERY_ROADMAP.md](LANGFUSE_DISCOVERY_ROADMAP.md). **JSON configuration** (and the sole product env var **`REX_ROOT`**) is in [CONFIGURATION.md](CONFIGURATION.md).

## When to use this

- **Bucketing** (Must/Should/Could/Won’t) and a **quick** R-ICE pass to compare two **Should**-level items → this file, with [ROADMAP.md](ROADMAP.md) for the one-page list.
- **Choosing what to learn or build next** in the near term → this file and the roadmap, aligned with the linked design docs.
- **Configuration keys, defaults, and semantics** → [CONFIGURATION.md](CONFIGURATION.md) (JSON only; product env is **`REX_ROOT`** alone).
- **A small, focused change** (for example one crate, one test) → ship the fix; a full pass through this file is **optional** unless the change shifts priorities.
- **Architecture policy or doc precedence** (which doc wins, ownership of a boundary) → [ARCHITECTURE_GUIDELINES.md](ARCHITECTURE_GUIDELINES.md) before re-scoring.

## MoSCoW (horizon filter)

| Bucket | Definition for this repo |
|--------|--------------------------|
| **Must** | Unmet **Must** release criteria (**RC-***) in [V1_0.md](V1_0.md)—or breaks contract, safety, or CI. Economics matrix must stay coherent ([CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md)). |
| **Should** | Clear **learning** or “local demo” value; risk is manageable; fits [ARCHITECTURE.md](ARCHITECTURE.md) and [ADAPTERS.md](ADAPTERS.md). |
| **Could** | A solid next step after current items; more work or groundwork first. |
| **Won’t (now)** | **Parked** for this period. New **Could** / **Won’t (now)** capabilities need a feature hub per [DOCUMENTATION.md — Roadmap and new features](DOCUMENTATION.md#roadmap-and-new-features) before a roadmap row. |

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

**Example (Should tie):** **R013** (daemon proto, medium contract risk, high learning value) vs **RC-S2** (extension-only, lower blast radius) — run **RC-S2** in parallel with **R013** when extension capacity is available; otherwise sequence **R013** first per blast-radius tie-breaker.

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

## Won't (now) — sidecar envelope (reference)

| Item | Pointer |
|------|---------|
| VM/container as default Mac dev-agent envelope | [AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md), [ROADMAP.md](ROADMAP.md) parked table |
| Always-on Colima/Docker/Firecracker for local agents | [AGENT_RUNTIME_ENVIRONMENT.md](AGENT_RUNTIME_ENVIRONMENT.md) deferred catalog |

## Current focus queue (audit 2026-07-01)

**Context:** Streaming/agent Must **RC-01–RC-10** are **Met** ([V1_0.md](V1_0.md)). **v1.0 not Met** — observability Must **RC-LF1** (**Not met**) blocks the **`1.0.0` git tag**. Product agent program (**R013–R038**) is **Done**. In-repo extension removed ([ADR 0038](architecture/decisions/0038-cli-ndjson-stream-transport.md)); terminal harness is the **primary operator surface**.

**Focus pivot:** Terminal harness program **R072** → **R073** supersedes LangFuse discovery sequencing until harness MVP lands. Hubs: [CLI_OPERATOR_UX.md](CLI_OPERATOR_UX.md), [TERMINAL_HARNESS_ARCHITECTURE.md](TERMINAL_HARNESS_ARCHITECTURE.md), [ADR 0039](architecture/decisions/0039-terminal-harness-presentation-and-daemon-intelligence.md).

**Ordering method:** MoSCoW bucket first; **R-ICE** (Impact × Confidence × Ease, 1–5) for **Should** ties; [tie-breakers](#tie-breakers-rex-specific) for blast radius and CI cost.

| Rank | ID / theme | MoSCoW | R-ICE | RC-* | Blast | Rationale |
|------|------------|--------|-------|------|-------|-----------|
| 1 | Terminal harness design docs + **ADR 0039** | **Must** | **75** (5×5×3) | — | docs | Locks architecture before **R072** code |
| 2 | **R072** — NDJSON core + messaging + **mdstream** | **Must** (program) | **70** | — | cli | Primary surface legibility — [TERMINAL_HARNESS_ARCHITECTURE.md](TERMINAL_HARNESS_ARCHITECTURE.md) |
| 3 | **R073** — full TUI + approval modals | **Should** | **65** | — | cli | Terminal-first MVP after **R072** |
| 4 | **R074** — optional LLM narrator | **Could** | **35** | — | cli | After TUI stable |
| 5 | **R067** — advisory intent retrieval | **Should** | **50** | — | daemon | Short-prompt context — parallel when capacity allows |
| 6 | **R069** — remove tool step caps | **Should** | **55** | — | sidecar | [ADR 0034](architecture/decisions/0034-remove-tool-step-caps.md) |
| 7 | **R040** — nightly live-smoke workflow | **Should** | **60** | RC-S6 | CI only | Informational tier |

**Deferred (until R072–R073 land):** **LF-D01** → **LF-D02** → **LF-R01** → **LF-F01** — LangFuse program; **RC-LF1** remains **Not met**. See [LANGFUSE_DISCOVERY_ROADMAP.md](LANGFUSE_DISCOVERY_ROADMAP.md).

**Later (harness v2):** **R076** LSP, **R077** git broker, **R078** MCP dynamic UI — [TERMINAL_HARNESS_ARCHITECTURE.md](TERMINAL_HARNESS_ARCHITECTURE.md).

**Deferred (Later / Won't now):** L2 semantic cache, in-daemon MLX, self-hosted LangFuse on Mac — see [ROADMAP.md](ROADMAP.md).

## Prior focus queue (audit 2026-06-09) — superseded

<details>
<summary>LangFuse-primary queue (2026-06-09)</summary>

**Advisory ask efficiency (2026-06-18):** **R069** → **R067** → **R068** → **R070** → **R059**. LangFuse **LF-D01** was rank 1. Superseded by terminal harness pivot 2026-07-01.

</details>

**Domain roadmaps:** [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md), [LANGFUSE_DISCOVERY_ROADMAP.md](LANGFUSE_DISCOVERY_ROADMAP.md), [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md), [ROADMAP.md](ROADMAP.md), [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md).

## Related

- [ROADMAP.md](ROADMAP.md) — one-page study roadmap
- [DOCUMENTATION.md](DOCUMENTATION.md) — how we write project docs
- [CI.md](CI.md) and [DEPENDENCIES.md](DEPENDENCIES.md) — what stays green; optional local tools
