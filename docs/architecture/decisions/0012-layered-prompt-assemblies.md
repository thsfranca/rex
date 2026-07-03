# ADR 0012: Layered prompt assemblies

- **Date:** 2026-05-24
- **Status:** Accepted

## Context

GitHub Copilot and similar tools inject repository instructions on every chat turn, often without a central token budget. REX aims to reduce redundant tokens by assembling context in the daemon ([CONTEXT_EFFICIENCY.md](../../CONTEXT_EFFICIENCY.md)). Layered prompts must coexist with lexical retrieval, knowledge bundles, and project memory without duplicating prose ([DEVELOPMENT_ASSISTANCE_CAPABILITIES.md](../../DEVELOPMENT_ASSISTANCE_CAPABILITIES.md) **C5**, **C6**).

## Decision

1. **Daemon owns** merge order: `system → project → mode` before lexical retrieval stages in the budget pipeline.
2. **Glob filtering** applies to project slices (Copilot-style `applyTo` semantics): only sections matching the active file path or task glob are merged.
3. **Config (R015):** JSON section `prompts` with paths or inline refs; env overrides remain for CI. Keys are documented in [CONFIGURATION.md](../../CONFIGURATION.md).
4. **`prompt_assembly_revision`:** Bump when any layer file changes; include in L1 cache key together with normalized effective user prompt and `context_revision` when retrieval ran ([CACHING.md](../../CACHING.md), tension **T1**).
5. **Caps:** Layered prompts consume up to **25%** of `context.max_context_tokens` by default (overridable); truncation trims retrieval before dropping core system slice.
6. **Dedup:** When importing git instruction files into a Rex bundle, dedupe by **content hash**; log `prompts=dedupe` on collision.
7. **Status:** Design accepted; implementation scheduled after R015.

## Consequences

- **Positive:** One assembly point; measurable `prompts=` bytes; fewer duplicated rules in sidecar.
- **Negative:** Operators must maintain prompt files; glob misconfiguration may omit needed rules.
- **Risks / follow-up:** Implement assembly in daemon; align extension to stop duplicating long rule blocks in user prompts.

## Related

- [ADR 0015](0015-agent-knowledge-bundles.md) — Curated knowledge vs prompts
- [ADR 0003](0003-layered-cache-agent-mode-policy.md) — L1 ask-only

## Market benchmark

- **Copilot** `copilot-instructions.md` + glob `*.instructions.md` — REX centralizes merge and caps tokens.
- **Cursor always-apply rules** — REX logs assembly size per turn.
