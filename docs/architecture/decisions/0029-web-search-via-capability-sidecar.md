# ADR 0029: Web search via capability sidecar

- **Date:** 2026-06-11
- **Status:** Accepted

## Context

Agents need live web grounding. [`net.fetch`](../../AGENT_ACCESS_POLICY.md) remains default deny. Prior research evaluated daemon-inline Brave LLM Context API; product direction selects **open-source SearXNG** as a **capability sidecar** so operators avoid search API keys while keeping host sidecars network-free ([CAPABILITY_SIDECARS.md](../../CAPABILITY_SIDECARS.md), [ADR 0028](0028-host-and-capability-sidecar-fleet.md)).

The default **12-step** tool loop and **8192-byte** broker truncation require a **single unified `web.search` tool** returning pre-truncated markdown snippets.

## Decision

1. **Tool:** `web.search` — parameters `query`, optional `recency`, optional `site_filter`.
2. **Broker RPC:** `BrokerWebSearch` on `rex.v1`; daemon invokes **`rex.capability.v1` `Invoke`** on the registered SearXNG capability sidecar.
3. **Primary backend (v1):** **SearXNG** capability sidecar — no Brave API key on default path.
4. **Formatting:** Daemon converts SERP JSON to compressed markdown; enforces `broker.max_tool_result_bytes`.
5. **Enablement:** Default **`enabled: false`**; explicit operator opt-in.
6. **Mode matrix:** `ask` **deny**; `plan` **allow**; `agent` **allow** with **ApprovalGate** ([ADR 0009](0009-centralized-agent-approvals-and-checkpoints.md)).
7. **Economics:** Per-turn rate cap (`max_queries_per_turn`, default 3); L2 query cache (12h TTL); per-turn dedup.
8. **UX:** v1 NDJSON tool-progress event while invoke is in flight (**E-WS01**).
9. **Future alternates:** Brave LLM Context or provider-native search — separate capability or provider; not v1 default.

## Consequences

- **Positive:** OSS path; daemon policy preserved; one tool step for typical research.
- **Negative:** Operator must run/maintain SearXNG; SERP snippets only until optional extract path; AGPL considerations for self-hosted SearXNG.
- **Risks / follow-up:** **R056** capability fleet before **R055** implementation; CI uses mock capability + fixtures.

## Related

- [WEB_SEARCH.md](../../WEB_SEARCH.md)
- [CAPABILITY_SIDECARS.md](../../CAPABILITY_SIDECARS.md)
- [ADR 0013](0013-access-policy-broker-completion.md)
- [ERROR_HANDLING.md](../../ERROR_HANDLING.md)
