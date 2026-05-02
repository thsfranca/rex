# ADR 0003: Layered cache and agent mode exclusions

- **Date:** 2026-05-02
- **Status:** Accepted

## Context

Response caching reduces **latency and cost**, but **`agent`** and often **`plan`** modes can imply **mutation** or **stale-plan** semantics if completions are reused after the workspace diverged.

Alternative: cache all modes identically until problems appear.

## Decision

Maintain **layered caches** documented in [CACHING.md](../../CACHING.md):

1. **L1 exact-match** applies to **`ask`** (and narrowly scoped safe cases); **`agent`** is **uncached by default**.
2. **L2 semantic** cache, if adopted, stays **high-guardrail** — **ask**-first until evidence supports broader use.

Schema/version and workspace-aware keys enforce invalidation semantics.

## Consequences

- **Positive:** Fewer correctness incidents from reused agent output.
- **Negative:** Repeated agent-like work cannot trivially reuse prior completions via L1 today.
- **Risks / follow-up:** If future agent runs become **explicitly deterministic** reads, revise policy with TTL + workspace fingerprints.
