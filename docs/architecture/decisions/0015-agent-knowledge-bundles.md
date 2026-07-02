# ADR 0015: Agent knowledge bundles

- **Date:** 2026-05-24
- **Status:** Accepted

## Context

Repositories accumulate agent-oriented markdown (rules, ADRs, hubs). Copilot injects instruction files per turn; MCP can fetch remote docs but adds token overhead. REX needs a governed, budgeted knowledge surface distinct from session-derived LTM ([AGENT_KNOWLEDGE.md](../../AGENT_KNOWLEDGE.md)).

## Decision

1. **Default model:** **Hybrid pointer** — committed `AGENTS.md` (or equivalent) references a `bundle_id` and minimum revision; payload may live under operator data dir (`$REX_HOME` concept) or synced from `docs/` hubs.
2. **Retrieval:** Single daemon stage `KnowledgeRetrieval` (15% default context budget); **MCP is a transport profile**, not a second inject path (**C11**).
3. **Rex-native read path:** Daemon `KnowledgeBroker` (or pipeline hook) lists/reads bundles under policy — not MCP-only.
4. **Drift rule:**
 - **`agent` mode:** `fail-closed` — if bundle revision ≠ git pointer, log `knowledge=drift` and omit stale chunks (or block turn per operator config).
 - **`ask` mode:** `prefer-git` — prefer committed pointer content when drift detected; log warning.
5. **Dedup with layered prompts:** One ingestion path per source file; content-hash dedupe (**C6**).
6. **Open questions resolved:**

 | Question | Decision |
 |----------|----------|
 | Rex-native broker vs MCP-only? | Rex-native broker; MCP optional transport |
 | Multi-workspace isolation? | **Deferred** — separate bundle namespace per workspace root when implemented |
 | Who edits knowledge? | **Deferred** — CLI/file drop first; UI later |
 | Single store vs federated bundles? | **Deferred** — start with one bundle per workspace |
 | vs layered prompts? | Prompts = rules; knowledge = reference corpora; separate stages |

## Consequences

- **Positive:** Bounded injection; revision-aware cache keys; clear split from LTM.
- **Negative:** Operators maintain pointers and bundles; drift handling may surprise in agent mode.
- **Risks / follow-up:** Implement sync job from `docs/` optional; document bundle format.

## Related

- [ADR 0012](0012-layered-prompt-assemblies.md)
- [ADR 0014](0014-long-term-memory-boundary.md)
- [AGENT_KNOWLEDGE.md](../../AGENT_KNOWLEDGE.md)

## Market benchmark

- **Copilot** repository instructions — REX adds versioning, budget, and drift policy.
- **Continue** MCP for docs — REX folds remote fetch into same `KnowledgeRetrieval` budget.
