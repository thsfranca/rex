# Stream event contract (internal)


> Role: reference | Status: active | Audience: contributors | Read when: stream event vocabulary for desktop projection
> Prefer: ## Event catalog

Canonical reference for **internal stream events** projected by the desktop client: modes, event shapes, terminal invariant, and stable error codes. **Not** a public subprocess API — public operator transport is UDS gRPC `StreamInference` via the Tauri desktop ([ADR 0042](architecture/decisions/0042-web-desktop-presentation-pivot.md); [ADR 0038](architecture/decisions/0038-cli-ndjson-stream-transport.md) superseded).

**Consumers:** `rex-stream-ui`, `fixtures/stream_events/` golden lines, guidelines CI. **Operator path:** [OPERATOR_UX.md](OPERATOR_UX.md). System context: [ARCHITECTURE.md](ARCHITECTURE.md).

## Goals

- Stable **event vocabulary** (`chunk`, `done`, `error`, additive `tool` / `step` / `plan` / `activity`) for desktop transcript projection.
- Mode-driven behavior: **`ask`**, **`plan`**, **`agent`** with deterministic guardrails and approvals where required.
- **Thin client:** the desktop does **not** host the agent. Assistant reasoning runs in a **daemon-supervised sidecar**; `rex-daemon` brokers inference and tools ([MVP_SPEC.md](MVP_SPEC.md), [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md)).

## Mode contract

| Mode | Purpose | Mutation behavior |
|---|---|---|
| `ask` | Research and explanation | Blocks file mutations. |
| `plan` | Structured planning and handoff | Mutations only after approval checkpoints. |
| `agent` | Guarded execution flow | Approval for execution and mutations. |

Rules: one active mode per session; visible transitions.

## Event catalog

One JSON object per logical stream line. Exactly **one** terminal event per request path (`done` **or** `error`); `tool`, `step`, `plan`, and `activity` lines may appear mid-stream and are non-terminal.

| Event | Fields | Purpose |
|-------|--------|---------|
| `chunk` | `index`, `text` | Markdown delta |
| `tool` | `index`, `name`, `phase`, `detail?`, `tool_call_id?`, `sequence?`, `elapsed_ms?`, `turn_id?` | Broker tool lifecycle (`running`, `completed`, `failed`) |
| `step` | `index`, `phase`, `summary`, `sequence?`, `turn_id?` | Orchestrator / subagent step visibility |
| `plan` | `index`, `phase`, `title`, `detail?`, `sequence?`, `turn_id?` | Plan clarify / ready events |
| `activity` | `index`, `phase`, `summary`, `detail?`, `sequence?` | Heartbeat / coarse phase (`thinking`, `tool_running`, …) |
| `done` | `index` | Stream complete |
| `error` | `message`, `code` | Terminal failure |

Fixtures: [`fixtures/stream_events/`](../fixtures/stream_events/). Operation feedback design: [OPERATION_FEEDBACK.md](OPERATION_FEEDBACK.md).

`error` objects expose stable `code` for routing — [ERROR_HANDLING.md](ERROR_HANDLING.md).

| Code | Meaning | Retry guidance |
|---|---|---|
| `daemon_unavailable` | Daemon not reachable | Retry after daemon start |
| `sidecar_unavailable` | Sidecar required but unavailable | Fix sidecar setup; see [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md) |
| `inference_config` | Inference backend not configured | Edit JSON `inference.openai_compat`; see [CONFIGURATION.md](CONFIGURATION.md) |
| `stream_timeout` | No stream activity in window | Retry usually safe |
| `stream_interrupted` | Mid-flight failure | Retry usually safe |
| `stream_incomplete` | No terminal marker | Inspect daemon; avoid blind retry |
| `cancelled` | User cancelled | No retry unless user resubmits |
| `invalid_response` | Malformed stream line | Inspect daemon logs |
| `spawn_failed` | Client could not start stream | Fix install / PATH |
| `approval_required` | Approval needed before execution | Supply approval after user confirms |
| `workspace_not_configured` | Process cwd unavailable | Run from a valid project directory |
| `workspace_mismatch` | Prompt workspace does not match daemon | Rebind workspace |
| `unknown` | Uncategorized | Manual diagnosis |

## Desktop bootstrap flow

1. Operator launches bare `rex` (desktop) or uses an existing session.
2. Desktop auto-starts daemon when needed ([OPERATOR_UX.md](OPERATOR_UX.md)).
3. User selects mode, enters prompt; desktop calls `StreamInference` over UDS.
4. `rex-stream-ui` maps gRPC stream chunks to the event vocabulary above.
5. UI routes `chunk` / `done` / `error` and additive events to transcript, timeline, and approval surfaces.

## Reliability and trace correlation

Bounded retries for daemon-unavailable races; interrupted streams terminate with `error`. Trace correlation uses daemon and desktop logs — [CONFIGURATION.md](CONFIGURATION.md).

## Non-goals

Public subprocess NDJSON on stdout; Node gRPC in a VS Code extension; plugin lifecycle inside thin clients; fully autonomous unattended execution without approvals.

## Related documents

| Document | Topic |
|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Whole-system architecture + ADRs |
| [OPERATOR_UX.md](OPERATOR_UX.md) | Desktop operator path |
| [ERROR_HANDLING.md](ERROR_HANDLING.md) | Error code catalog |
| [OPERATION_FEEDBACK.md](OPERATION_FEEDBACK.md) | Stream feedback design |
| [WEB_UI_ARCHITECTURE.md](WEB_UI_ARCHITECTURE.md) | Desktop streaming bridge |
