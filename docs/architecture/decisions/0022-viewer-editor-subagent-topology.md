# ADR 0022: Viewer/Editor subagent topology for product agent

- **Date:** 2026-06-04
- **Status:** Accepted

## Context

Monolithic ReAct (**R018**) sends one growing scratch transcript on every `BrokerInference` call. Exploratory reads pollute Editor context and inflate tokens. Research and [AGENT_GRAPH_ARCHITECTURE.md](../../AGENT_GRAPH_ARCHITECTURE.md) recommend Orchestrator-led **Viewer** (read/list) and **Editor** (write/exec) subgraphs with isolated message channels.

Rex retains broker-only host effects ([ADR 0008](0008-dedicated-sidecar-control-plane-api.md)) and JSON `tool`/`args` protocol compatibility.

## Decision

1. **Target topology:** LangGraph **Orchestrator** routes turns to **Viewer**, **Editor**, or direct final answer; each subgraph uses `RexBrokerChatModel` and shared broker `ToolNode`.
2. **Context isolation:** Editor prompts exclude raw Viewer read dumps; Viewer findings compact into `viewer_summary` for Editor (**R028**, **R029**).
3. **No rigid planner:** Routing stays LLM-driven per turn, not a fixed plan-and-execute graph.
4. **One tool per step** retained to match daemon policy and current JSON stream contract.
5. **Implementation** lands in phased sidecar PRs **R027–R032**; narrative and playbook live in [AGENT_GRAPH_ARCHITECTURE.md](../../AGENT_GRAPH_ARCHITECTURE.md).

## Consequences

- **Positive:** Lower intra-turn token growth; clearer separation of exploration vs modification; aligns with product agent roadmap.
- **Negative:** More graph complexity than monolithic ReAct; LangChain message migration in sidecar.
- **Risks / follow-up:** Prove economics on live turns; optional **R031** read pruning if compaction insufficient; **R033** may deprecate JSON regex when native tools ship.

## Related

- [ADR 0016](0016-mcp-in-sidecar-envelope.md) — MCP Phase 2
- [AGENT_DELIVERY_ROADMAP.md](../../AGENT_DELIVERY_ROADMAP.md)
