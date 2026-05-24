# ADR 0016: MCP in sidecar envelope

- **Date:** 2026-05-24
- **Status:** Accepted

## Context

MCP exposes many tools whose JSON schemas can consume a large fraction of the context window every turn. REX already provides a small, stable broker surface (`fs.read`, `fs.list`, `fs.write`, `exec.shell`) on `rex.v1` ([ADR 0008](0008-dedicated-sidecar-control-plane-api.md)). Host filesystem and network must not become ambient in the sidecar guest.

## Decision

1. **Placement:** MCP stacks run **primarily in the isolated sidecar** process; **host-affecting** operations map to **daemon broker** RPCs, not ambient guest access (**C10**).
2. **Default product path:** `rex-agent` uses **broker verbs** for workspace operations; MCP is an **optional profile** for ecosystem tools (package registries, issue trackers, etc.).
3. **Tool definition overhead:** Prefer **lazy tool discovery** (search / load-on-demand) when MCP is enabled; never preload full multi-server schema into the LLM context by default. Target constant small proxy surface (industry pattern: ~600 tokens vs 10K+).
4. **`net.fetch`:** Remains **denied** at broker until a scoped policy exists ([AGENT_ACCESS_POLICY.md](../../AGENT_ACCESS_POLICY.md)).
5. **Mapping:** MCP tools that read/write workspace files must call broker equivalents; daemon enforces `AccessPolicy` ([ADR 0013](0013-access-policy-broker-completion.md)).
6. **Implementation:** Deferred; this ADR records direction for [CONTEXT_EFFICIENCY.md](../../CONTEXT_EFFICIENCY.md) matrix and [ROADMAP.md](../../ROADMAP.md).

## Consequences

- **Positive:** Avoids MCP token tax on every turn for core dev workflows; aligns with Mac-first process sidecar.
- **Negative:** MCP servers may need adapters to broker; lazy discovery adds latency on first tool use.
- **Risks / follow-up:** Formal MCP ADR addendum when scheduling implementation; evaluate MCP Tool Search–style deferral.

## Related

- [ADR 0005](0005-rex-owns-sidecar-environment-not-agent-implementations.md)
- [ADR 0015](0015-agent-knowledge-bundles.md) — Knowledge via daemon stage, not duplicate MCP resource path

## Market benchmark

- Measured **55K tokens** for large GitHub MCP tool lists on a single turn — REX broker-first path avoids schema dump for host ops.
- **Anthropic code execution with MCP** — lazy file load; REX maps to broker-by-name + optional MCP proxy.
