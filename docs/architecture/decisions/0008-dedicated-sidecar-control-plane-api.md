# ADR 0008: Dedicated sidecar control-plane API (brokered integration)

- **Date:** 2026-05-05
- **Status:** Accepted

## Context

[ADR 0005](0005-rex-owns-sidecar-environment-not-agent-implementations.md) states that **`rex-daemon`** owns the **isolated sidecar / agent-runtime environment** (lifecycle, resource envelope, authorized channel, policy alignment). It does **not** fully specify the **integration surface**: how a guest runtime requests inference and tools **without** widening the stable **`rex.v1`** contract used by CLI and editor-class clients.

Requirements gathered from product architecture discussion:

1. **Trust model — request / authorize, not command / obey**  
   The sidecar expresses **intent** (for example inference plan tier, structured tool invocation). **`rex-daemon`** **authorizes, executes, meters, logs**, and retains **economics and stream authority** — consistent with [ADR 0001](0001-daemon-owns-agent-orchestration-and-economics.md) and ADR 0005.

2. **Separation from `rex.v1`**  
   A **narrow, purpose-built** sidecar ↔ daemon integration surface lets guest protocols evolve **without** turning **`rex.v1`** into a tunnel.

3. **Isolation envelope**  
   The sidecar runs in an **isolated environment** without ambient host filesystem or network **by default**. Host reach is **only** via **daemon-brokered** API. Cross-boundary transport follows **explicit bridging** assumptions (see [AGENT_RUNTIME_ENVIRONMENT.md](../../AGENT_RUNTIME_ENVIRONMENT.md)).

4. **Streaming semantics**  
   **`StreamInference` / terminal correctness for `rex.v1` clients** remain **daemon-authoritative**. Internal routing or sidecar-planned backends must converge to **one** client-visible outcome.

5. **Caching and policy hooks**  
   Routing identifiers and cache keys attach to **daemon-resolved execution** (“what actually ran”), not only uncleared guest intent — so economics and cache keys stay consistent.

## Decision

1. **Dedicated sidecar control plane**  
   Sidecar workloads integrate through a **purpose-built sidecar ↔ daemon API**, **distinct** from **`rex.v1`** consumed by `rex-cli` and thin editors.

2. **Full isolation by default**  
   No ambient host FS/network for the guest unless explicitly brokered and policy-gated through the daemon.

3. **`rex.v1` unchanged for client scope**  
   CLI and extension-class clients keep the **thin shared contract**; sidecar integration **does not** substitute for or widen **`rex.v1`** into a generic proxy.

4. **Explicit versioning**  
   The sidecar-facing API is **versioned** and evolves with **small composable verbs**, schemas, quotas, timeouts, and structured errors — not ad-hoc escape hatches.

5. **Anti-pattern: generic proxy**  
   Reject exposing arbitrary URLs, raw TCP “tunnel the host,” or ambient **`exec`** without policy as part of this surface.

| Do | Do not |
|----|--------|
| Add **small, composable verbs** with schemas, quotas, timeouts, structured errors | Expose **arbitrary URLs**, raw TCP sockets, or “tunnel the host network” primitives |
| **Default-deny** new capabilities until policy + observability hooks exist | Add **ambient filesystem** or open-ended **`exec`** escapes without policy |
| Version the sidecar API explicitly; deprecate deliberately | Grow ad-hoc “escape hatches” for velocity |

## Consequences

- **Positive:** Clear trust boundary; **`rex.v1`** stays stable; sidecar stacks can evolve behind one brokered surface.
- **Negative:** Extra design and implementation work for the control-plane API and policy matrix; indirection for guest runtimes vs direct host access.
- **Risks / follow-up:** Multi-backend routing inside the daemon must still converge streaming and economics to a **single** observable outcome; document concrete API sketches when implementation starts.

## Related

- [ADR 0001](0001-daemon-owns-agent-orchestration-and-economics.md) · [ADR 0005](0005-rex-owns-sidecar-environment-not-agent-implementations.md) · [ADR 0007](0007-editor-extension-hybrid-transport-cli-and-grpc.md)
- [ARCHITECTURE.md](../../ARCHITECTURE.md) · [AGENT_RUNTIME_ENVIRONMENT.md](../../AGENT_RUNTIME_ENVIRONMENT.md) · [PLUGIN_ROADMAP.md](../../PLUGIN_ROADMAP.md)
- [README.md](README.md) (index)
