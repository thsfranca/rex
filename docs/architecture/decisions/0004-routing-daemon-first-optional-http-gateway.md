# ADR 0004: Daemon-first routing; optional HTTP gateway

- **Date:** 2026-05-02
- **Status:** Accepted

## Context

Teams sometimes centralize LM calls through **HTTP proxies** (OpenAI-compat, multi-provider gateways) for metering and spend caps. REX prioritizes **local UDS**, **minimal hops**, and **repo-controllable routing**.

Alternative A: mandate an external LiteLLM-style proxy for production.

Alternative B: implement only random HTTP clients with no cohesive policy slot.

## Decision

1. **Primary:** Routing and budget policy reside **inside `rex-daemon`** (eventually as an explicit router module layered before `InferenceRuntime`).
2. **Optional later:** Thin opt-in **`REX_HTTP_PROXY_URL`**-style outbound path treated as **one adapter/backend**, not mandatory for correctness.
3. **Rejected:** Hard dependency on a separate Python/other gateway process before the local agent story proves out.

See optimization matrix rows in [CONTEXT_EFFICIENCY.md](../../CONTEXT_EFFICIENCY.md).

## Consequences

- **Positive:** Lower baseline latency on macOS workstations; aligns with privacy/local-first stance.
- **Negative:** Daemon must evolve to match gateway-class features teams expect (routing metadata, quotas) rather than outsource them blindly.
- **Risks / follow-up:** Revisit proxy-first posture if regulated environments **require** a central gateway tenant model.
