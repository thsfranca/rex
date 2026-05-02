# ADR 0002: Inference adapter contract and capabilities

- **Date:** 2026-05-02
- **Status:** Accepted

## Context

Different backends tolerate different **prompt shaping** (context injection, truncation, caches). Applying every pipeline stage to every backend risks **double retrieval**, **lost user text**, or **unsafe caching**.

Alternative: adapters are undocumented black boxes invoked from the daemon with no negotiation.

## Decision

Expose an **`InferenceRuntime` trait seam** (`crates/rex-daemon/src/adapters.rs`) with documented **`AdapterCapabilities`** (concept in [ADAPTERS.md](../../ADAPTERS.md)):

- **Per-backend** switches for context injection, token budgets, timeouts, cacheable modes.

The daemon **skips or applies** `ContextPipeline` stages based on capability flags before invocation.

## Consequences

- **Positive:** Keeps UX consistent while backends differ sharply (e.g. Cursor CLI subprocess vs pure local mock).
- **Negative:** Every new capability flag must remain documented alongside runtimes [ADAPTERS.md](../../ADAPTERS.md).
- **Risks / follow-up:** Capability matrix must evolve when new pipeline stages arrive (routing, compaction services).
