# ADR 0029: Ask-mode research broker

- **Date:** 2026-06-11
- **Status:** Accepted

## Context

`ask` mode was a single LLM hop with no tools, while daemon access policy already allowed `fs.read` / `fs.list`. Operators could not research the workspace or the web without switching to `plan` / `agent`.

## Decision

1. **`ask` tool set:** `fs.read`, `fs.list`, `web.search` only — no writes or shell.
2. **`BrokerWebSearch`:** New unary `rex.v1` RPC; provider configured in JSON under `$REX_ROOT` (`search.enabled`, `search.provider`, `search.api_key_path`).
3. **Access policy:** `web.search` allowed in `ask` when search is enabled; denied in `plan` / `agent` for v1.
4. **Cache:** L1 exact-match skipped when any broker tool ran in the turn.
5. **Step budget:** `agent.max_tool_steps_ask` (default 5).

## Consequences

- **Positive:** Research mode matches product copy; daemon enforces network boundary.
- **Negative:** Search provider setup required for live web results; mock/stub for CI.

## Related

- [AGENT_ACCESS_POLICY.md](../../AGENT_ACCESS_POLICY.md)
- [OPERATION_FEEDBACK.md](../../OPERATION_FEEDBACK.md)
- [ADR 0013](0013-access-policy-broker-completion.md)
