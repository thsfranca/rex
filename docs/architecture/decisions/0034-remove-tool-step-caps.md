# ADR 0034: Remove tool step caps

- **Date:** 2026-06-18
- **Status:** Accepted
- **Supersedes:** [ADR 0032](0032-soft-cap-continue-turn.md) (soft cap + hard cap policy)

## Context

Rex agent tool loops terminate at mode-specific hard caps (`agent.max_tool_steps*`) and optional soft-cap pauses ([ADR 0032](0032-soft-cap-continue-turn.md)). Short advisory prompts (for example “What should we do next?”) often exhaust the ask cap before the model synthesizes an answer. OpenCode and similar agents run until the model emits a final response.

R063 shipped Continue UX to extend caps; operators still hit cap-terminal errors on legitimate research turns.

## Decision

1. **Remove hard step cap enforcement** in the sidecar graph — no terminal message when `tool_steps >= max_steps`.
2. **Remove soft-cap pause** — no `awaiting_continue` activity, no `ContinueTurn` step extension path. Deprecate `ContinueTurn` RPC (no-op or removed in a follow-up cleanup PR).
3. **Loop termination:** model emits `final` / `clarify` / `plan`, or **`agent_loop_stuck`** fires after three consecutive policy/config tool failures (R060 circuit breaker unchanged).
4. **Retain:** `max_tools_per_step` batch limit (R057), exact-match duplicate cache (R061), broker access policy, agent approvals.
5. **Config:** `agent.max_tool_steps`, `agent.max_tool_steps_ask`, `agent.max_tool_steps_plan`, `agent.soft_cap_enabled`, `agent.soft_cap_fraction`, `agent.soft_cap_step_extension` → **removed** from schema (**R082**); ignored if present in older JSON.
6. **Economics guardrail:** turn metrics (R064) and LangFuse export remain the path to reintroduce optional caps if runaway cost appears — not a default product limit.

## Consequences

- **Positive:** Ask/plan research prompts can complete without artificial cap errors; aligns with OpenCode-style agent loops.
- **Negative:** Unbounded loops possible on misbehaving models; mitigated by circuit breaker and operator cancel.
- **Follow-up:** Schema fields removed (**R082**).

## Related

- [AGENT_GRAPH_ARCHITECTURE.md](../../AGENT_GRAPH_ARCHITECTURE.md) — **R069**
- [ADR 0032](0032-soft-cap-continue-turn.md) — superseded cap policy
- [CONFIGURATION.md](../../CONFIGURATION.md) — deprecated keys
- [ECONOMICS_VALIDATION.md](../../ECONOMICS_VALIDATION.md) — turn loop metrics
