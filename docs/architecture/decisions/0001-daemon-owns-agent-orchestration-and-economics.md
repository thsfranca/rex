# ADR 0001: Daemon owns agent orchestration and economics

- **Date:** 2026-05-02
- **Status:** Accepted

## Context

REX aims to minimize **dependence on opaque external agent loops** for **cost, routing, and policy**, while still supporting **multiple inference backends** (local mock, subprocess CLIs, future APIs).

Alternative: delegate the full agent (planning, tools, multi-step edits) to a third-party CLI and treat REX as a thin transport shim.

## Decision

The **rex-daemon** owns the **development agent boundary** over time:

- Stream contract, modes, caches, capability-aware pipelines, adapter selection/configuration, and (as implemented) telemetry fields.
- **Inference adapters** produce **model output** under daemon policy but **do not define** REX’s product identity as “the agent.” An external subprocess may loop internally; REX remains the **integration and economics envelope** visible to IDE and CLI clients.

## Consequences

- **Positive:** Single place to add routing, metering, compaction, durable project memory without forking clients.
- **Negative:** More logic eventually lives in the daemon; subprocess adapters must respect timeouts and terminals.
- **Risks / follow-up:** Full tool orchestration MCP loop is **not** necessarily implemented yet; roadmap must separate **intent** vs **shipping state** in [ARCHITECTURE.md](../../ARCHITECTURE.md) and [MVP_SPEC.md](../../MVP_SPEC.md).
