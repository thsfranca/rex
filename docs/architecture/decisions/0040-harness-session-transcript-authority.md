# ADR 0040: Harness session transcript authority

- **Date:** 2026-07-04
- **Status:** Accepted

## Context

Multiple bare **`rex`** terminals in one workspace share a per-workspace daemon ([ADR 0036](0036-per-workspace-daemon-routing.md)). Each terminal must run an **independent harness session** — separate agent context, streams, and operator transcript — without duplicate daemons or merged UI state.

[ADR 0014](0014-long-term-memory-boundary.md) and [DEVELOPMENT_ASSISTANCE_CAPABILITIES.md](../../DEVELOPMENT_ASSISTANCE_CAPABILITIES.md) **C4** state that the daemon must not store a UI transcript database for **LTM ingestion**. That boundary targets **long-term memory** (extracted facts), not **operational harness session state** required for multi-turn agent turns and TUI reattach.

[ADR 0039](0039-terminal-harness-presentation-and-daemon-intelligence.md) places intelligence in the daemon; the TUI is a presentation client. The TUI owns **all UI logic** (viewport cache, pagination, motion); the daemon owns **durable session context** for agent seeding.

## Decision

1. **Harness session id:** Each bare `rex` TUI generates a `harness_session_id` (gRPC metadata `x-rex-harness-session-id`). Prefix and L1 caches are scoped per session.
2. **Context source of truth:** Daemon holds a **durable append-only event log** per `harness_session_id` under `.rex/sessions/` (implementation: follow-up PR). Events follow [NDJSON_STREAM.md](../../NDJSON_STREAM.md) semantics.
3. **TUI role:** In-memory **viewport only** — bounded cache, scroll position, merge rules (hot tail never evicted on retroactive fetch). Not authoritative for agent context.
4. **Fetch APIs (additive gRPC):** `FetchSessionEvents` with cursor modes:
   - **Incremental:** `after_sequence` + `limit` (catch-up / live gap)
   - **Retroactive:** `before_sequence` + `limit` (scroll-up backfill)
   Daemon returns events + cursors; **no pagination policy** in the daemon.
5. **Agent seed:** On each turn, daemon builds `prior_messages` from the session log — not from TUI resend.
6. **LTM boundary unchanged:** Operational session transcript ≠ LTM ingestion source ([ADR 0014](0014-long-term-memory-boundary.md)). LTM continues to ingest exported events / extracted facts only.

## Consequences

- **Positive:** Parallel harness terminals with isolated agent threads; TUI reattach via fetch APIs; extension/CLI parity on the same RPCs.
- **Negative:** Disk growth under `.rex/sessions/`; new gRPC surface; TUI viewport + motion work for history fetch.
- **Risks / follow-up:** Session compaction policy (distinct from LTM); privacy documentation for on-disk transcripts.

## Related

- [ADR 0014](0014-long-term-memory-boundary.md) — LTM boundary (amended nuance for operational transcript)
- [ADR 0036](0036-per-workspace-daemon-routing.md) — shared daemon per workspace
- [ADR 0039](0039-terminal-harness-presentation-and-daemon-intelligence.md) — presentation vs intelligence split
- [TERMINAL_HARNESS_ARCHITECTURE.md](../../TERMINAL_HARNESS_ARCHITECTURE.md) — responsibility split
- [TUI_DESIGN.md](../../TUI_DESIGN.md) — in-flight motion invariant
