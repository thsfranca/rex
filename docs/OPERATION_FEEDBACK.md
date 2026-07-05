# Operation feedback during daemon execution


> Role: explanation | Status: active | Audience: contributors | Read when: live operation feedback stream
> Prefer: ## Purpose

**Status:** `implemented` (program tracked on [ROADMAP.md](ROADMAP.md) **R044**).

## Purpose

Operators using the Rex desktop need **visible progress** while the daemon and sidecar work: LLM inference, broker tools, and approvals. This hub defines the **streaming feedback contract**, client presentation intent, approval parity, and ask-mode research tools.

**Transport:** Desktop → UDS gRPC `StreamInference` → daemon → sidecar `RunTurn` ([WEB_UI_ARCHITECTURE.md](WEB_UI_ARCHITECTURE.md), [ADR 0042](architecture/decisions/0042-web-desktop-presentation-pivot.md)).

## Boundaries

| Layer | Owns |
|-------|------|
| Sidecar | Tool/step/text/plan/activity events during `RunTurn`; live flush via stream sink |
| Daemon | Policy, approvals, broker execution, stream passthrough, pre-stream step events |
| Desktop (`rex-stream-ui`) | Event projection, approval modal, idle timeout from JSON, operator messaging ([OPERATOR_UX.md](OPERATOR_UX.md)) |

Configuration uses **`$REX_ROOT/config.json`** and `.rex/config.json` only ([CONFIGURATION.md](CONFIGURATION.md)). Bootstrap env is **`REX_ROOT`** only.

## Stream event catalog

Non-terminal events may appear mid-stream. Exactly one terminal event per request: `done` or `error` ([ERROR_HANDLING.md](ERROR_HANDLING.md), [NDJSON_STREAM.md](NDJSON_STREAM.md)).

| `event` | Fields | Semantics |
|---------|--------|-----------|
| `chunk` | `index`, `text` | Markdown delta |
| `tool` | `index`, `name`, `phase`, `detail?`, `tool_call_id?`, `sequence?`, `elapsed_ms?`, `turn_id?` | Broker tool lifecycle (`running`, `completed`, `failed`) |
| `step` | `index`, `phase`, `summary`, `sequence?`, `turn_id?` | Orchestrator / subagent step |
| `plan` | `index`, `phase`, `title`, `detail?` | Plan artifact |
| `activity` | `index`, `phase`, `summary`, `detail?`, `sequence?` | Heartbeat / coarse phase (`thinking`, `tool_running`, `broker_wait`, `compacting`, `preparing`) |
| `done` | `index` | Stream complete |
| `error` | `message`, `code` | Terminal failure |

Unknown JSON keys are ignored by clients (additive contract).

## Client mapping

| Concern | Desktop web UI |
|---------|----------------|
| Tool running | Timeline + tool cards from `tool` events |
| Activity heartbeat | Activity strip / status chrome |
| Approval (agent) | In-app approval modal; passes approval id to daemon |
| Idle timeout | Stream cancel + idle message; config `cli.stream_idle_timeout_secs` |
| Ask research tools | Tool cards, no approval |
| Daemon lifecycle | Header/status ready / starting / unavailable ([OPERATOR_UX.md](OPERATOR_UX.md)) |

## Approval parity (ADR 0009)

When `agent.approvals_enabled` is true in merged JSON:

- **Desktop:** approval modal before agent execution; passes approval id on the gRPC stream.
- **Daemon:** unchanged — non-empty `approval_id` allows agent mode; missing id denies.

`ask` and `plan` do not require execution approval.

## Ask-mode research

`ask` mode uses a **read-only tool loop**: `fs.read`, `fs.list`, and optionally `web.search` ([ADR 0031](architecture/decisions/0031-ask-mode-research-broker.md)). Native broker path may batch multiple read/list calls per LLM round (**R057**); `web.search` is never mixed with read/list in one batch. No `fs.write`, `exec.shell`, or `plan.save`. L1 cache skips turns that invoked any broker tool ([ADR 0003](architecture/decisions/0003-layered-cache-agent-mode-policy.md)).

**Workspace-first:** The sidecar advertises `web.search` only when `search.enabled` is true in merged config **and** the model has successfully read or listed workspace files in the turn, or the user prompt signals explicit web intent. Early `web.search` attempts are rejected with stable guidance so the model retries with local reads.

## Tool step billing (all modes)

`tool_steps` counts **productive** broker rounds for turn metrics (step caps removed — [ADR 0034](architecture/decisions/0034-remove-tool-step-caps.md)). A round bills when any tool returns `ok=True` or the broker executed and returned exploratory feedback (for example not-found, non-zero exit). Rounds where **every** failure is policy or configuration class (`mode_denied`, `access policy denied`, sidecar validation) do **not** increment ([ADR 0013](architecture/decisions/0013-access-policy-broker-completion.md)). Sidecar pre-gate rejections (invalid batch, workspace-first) never reach billing.

## Live streaming (ADR 0030)

Sidecar `RunTurn` flushes events during graph execution via a **stream sink** and LangGraph `stream` — not only after `invoke` completes. Daemon live path passthrough-maps chunks unchanged.

## Related

- [NDJSON_STREAM.md](NDJSON_STREAM.md) — internal stream event reference
- [WEB_UI_DESIGN.md](WEB_UI_DESIGN.md) — presentation (timeline, activity strip)
- [OPERATOR_UX.md](OPERATOR_UX.md) — desktop operator path
- [AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md) — `web.search` mode matrix
- [POLICY_ENGINE.md](POLICY_ENGINE.md) — approval gate
- [fixtures/ndjson_contract/](../fixtures/ndjson_contract/) — golden stream event lines
