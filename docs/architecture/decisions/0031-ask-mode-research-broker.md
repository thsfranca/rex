# ADR 0031: Ask-mode research broker

- **Date:** 2026-06-11
- **Status:** Accepted

## Context

`ask` mode was a single LLM hop with no tools, while daemon access policy already allowed `fs.read` / `fs.list`. Operators could not research the workspace or the web without switching to `plan` / `agent`.

## Decision

1. **`ask` tool set:** `fs.read`, `fs.list`, `web.search` only — no writes or shell.
2. **`BrokerWebSearch`:** New unary `rex.v1` RPC; provider configured in JSON under `$REX_ROOT` (`search.enabled`, `search.provider`, `search.api_key_path`).
3. **Access policy:** `web.search` allowed in `ask` when search is enabled; denied in `plan` / `agent` for v1.
4. **Cache:** L1 exact-match skipped when any broker tool ran in the turn.
5. **Step budget:** removed (**R069** / [ADR 0034](0034-remove-tool-step-caps.md)). Sidecar bills productive broker rounds for metrics only; policy/config denials do not increment ([ADR 0013](0013-access-policy-broker-completion.md)).
6. **Workspace-first:** `web.search` is advertised only when `search.enabled` and the turn has explored workspace files (`fs.read` / `fs.list` success) or the prompt signals explicit web intent; read/list and search are not batched together.

## Consequences

- **Positive:** Research mode matches product copy; daemon enforces network boundary.
- **Negative:** Search provider setup required for live web results; mock/stub for CI.

## Related

- [AGENT_ACCESS_POLICY.md](../../AGENT_ACCESS_POLICY.md)
- [OPERATION_FEEDBACK.md](../../OPERATION_FEEDBACK.md)
- [ADR 0013](0013-access-policy-broker-completion.md)
