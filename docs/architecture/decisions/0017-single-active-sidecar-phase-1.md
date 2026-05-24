# ADR 0017: Single-active sidecar for Phase 1

- **Date:** 2026-05-24
- **Status:** Accepted

## Context

[ROADMAP.md](../../ROADMAP.md) **R016** proposes multi-active sidecars and broadcast `RunTurn`. Shared workspace LTM and daemon locks conflict with multiple concurrent agent graphs unless carefully specified ([DEVELOPMENT_ASSISTANCE_CAPABILITIES.md](../../DEVELOPMENT_ASSISTANCE_CAPABILITIES.md) **P2**).

## Decision

1. **Phase 1 product path:** **Single active sidecar** — config key `sidecars.active` names one entry (default `agent` → `rex-agent` when shipped).
2. **R016 broadcast API:** **Deferred** (Could) — no `sidecars.active[]` broadcast in Phase 1.
3. **When multi-active is revisited:** Shared **daemon** LTM and knowledge stores with workspace-scoped locks; **per-sidecar** ephemeral scratch only; ADR amendment required.
4. **`rex-sidecar-stub`** remains the CI harness; switched via `REX_SIDECAR_*` or R015 config.

## Consequences

- **Positive:** Simpler supervisor, stream routing, and economics attribution for R017–R019.
- **Negative:** No parallel multi-agent broadcast until R016 is designed and accepted.
- **Risks / follow-up:** Re-open when operator demand and LTM semantics are clear.

## Related

- [AGENT_DELIVERY_ROADMAP.md](../../AGENT_DELIVERY_ROADMAP.md)
- [SIDECAR_RUNTIME.md](../../SIDECAR_RUNTIME.md)
