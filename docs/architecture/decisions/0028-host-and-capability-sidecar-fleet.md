# ADR 0028: Host and capability sidecar fleet

- **Date:** 2026-06-11
- **Status:** Accepted
- **Amends:** [ADR 0017](0017-single-active-sidecar-phase-1.md)

## Context

Phase 1 supervised **one** sidecar process with `rex.sidecar.v1` oriented around **`RunTurn`** ([ADR 0017](0017-single-active-sidecar-phase-1.md)). [ADR 0005](0005-rex-owns-sidecar-environment-not-agent-implementations.md) states Rex owns the **environment**, not a single agent implementation — sidecars are pluggable host processes.

General-purpose features (web search via SearXNG, future plugins) need **callable services** with scoped network egress. They are not agent loops and must not displace the host sidecar slot or receive user prompts directly.

## Decision

1. **Role split:** Supervised sidecar processes are either **host** (prompt + tool loop) or **capability** (feature provider).
2. **Host:** Exactly **one** active host sidecar when the sidecar agent path is used; continues **`rex.sidecar.v1`** / `RunTurn`.
3. **Capability:** **0..N** concurrent capability sidecars; new package **`rex.capability.v1`** with `Health`, `GetCapabilities`, **`Invoke`**.
4. **Routing:** Host sidecars call **daemon broker RPCs only**. Daemon resolves `capability_id` → capability sidecar and forwards `Invoke` after policy.
5. **Network:** Host sidecar remains **no ambient egress**. Capability sidecars may receive **scoped egress** for their feature, still daemon-supervised.
6. **Config intent:** `sidecars.host` (or compatibility fallback from `sidecars.active`) plus `sidecars.capabilities[]` entries with `provides`, `socket`, `enabled`.
7. **Not a sidecar:** LiteLLM inference gateway stays under `inference.gateway.*` ([INFERENCE_GATEWAY.md](../../INFERENCE_GATEWAY.md)).

## Consequences

- **Positive:** General-purpose plugins without overloading `RunTurn`; preserves daemon policy envelope.
- **Negative:** New proto, supervisor, and registry; more moving parts than daemon-inline HTTP.
- **Risks / follow-up:** Implementation in **R056**; first consumer **R055** `web.search` — [WEB_SEARCH.md](../../WEB_SEARCH.md).

## Related

- [CAPABILITY_SIDECARS.md](../../CAPABILITY_SIDECARS.md)
- [SIDECAR_RUNTIME.md](../../SIDECAR_RUNTIME.md)
- [ADR 0008](0008-dedicated-sidecar-control-plane-api.md)
- [ROADMAP.md](../../ROADMAP.md) **R056**, **R016**
