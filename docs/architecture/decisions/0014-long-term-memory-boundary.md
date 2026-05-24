# ADR 0014: Long-term memory boundary

- **Date:** 2026-05-24
- **Status:** Accepted

## Context

Long-running agent sessions pressure context windows. Products like Mem0 extract and retrieve salient facts instead of replaying full transcripts. REX commits to daemon-owned economics ([ADR 0001](0001-daemon-owns-agent-orchestration-and-economics.md)) but must not store chat UI state in the daemon ([DEVELOPMENT_ASSISTANCE_CAPABILITIES.md](../../DEVELOPMENT_ASSISTANCE_CAPABILITIES.md) **C4**).

## Decision

1. **Scope:** **Workspace-bound** project memory — facts and fingerprints useful across sessions, not full chat logs.
2. **Split:** **Episodic** (session/tool traces, time-bounded) vs **semantic** (durable preferences, decisions, repo facts).
3. **Ingestion sources:** Exported events from extension or sidecar (tool summaries, operator-approved files), optional structured imports — **not** a daemon-owned transcript database.
4. **Pipeline hook:** `ProjectMemoryRetrieval` runs after layered prompts and knowledge, before lexical retrieval; budget **10%** of context tokens by default.
5. **Storage (implementation bet):** `sqlite` or files under operator data dir; vector index optional behind economics gate.
6. **Compaction / forgetting:** Explicit policies (TTL, max entries per workspace, summarization) — separate from per-turn extractive compression (**T6**).
7. **No new `rex.v1` RPC** until a versioned migration and benchmark proof of **net token reduction** ([OBSERVABILITY_AND_ECONOMICS.md](../../OBSERVABILITY_AND_ECONOMICS.md)).
8. **Hub:** [LONG_TERM_MEMORY.md](../../LONG_TERM_MEMORY.md) lists remaining implementation bets; accepted boundaries are in this ADR.

## Consequences

- **Positive:** Unified policy with retrieval; path to Mem0-like savings without a separate SDK in the sidecar.
- **Negative:** Ingestion plumbing across clients; risk of stale facts without compaction.
- **Risks / follow-up:** Benchmark harness before shipping; define export format from extension.

## Related

- [ADR 0015](0015-agent-knowledge-bundles.md) — Stable curated knowledge vs session-derived memory
- [LONG_TERM_MEMORY.md](../../LONG_TERM_MEMORY.md)

## Market benchmark

- **Mem0** ~90% token reduction vs full-context in published benchmarks — REX targets similar economics under daemon policy, not mandatory third-party service.
