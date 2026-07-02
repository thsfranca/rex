# NDJSON stream contract

Canonical reference for **`rex complete --format ndjson`**: modes, stream events, terminal invariant, and error codes. Consumers include the **CLI TUI** (internal consumer), **scripts**, and **CI conformance tests**. TUI parsing parity: [TERMINAL_HARNESS_ARCHITECTURE.md](TERMINAL_HARNESS_ARCHITECTURE.md). System context: [`ARCHITECTURE.md`](ARCHITECTURE.md).

## Goals

- Stable boundary through **`rex`** subprocess NDJSON for streaming completion; optional unary **`rex.v1`** over UDS per **[ADR 0038](architecture/decisions/0038-cli-ndjson-stream-transport.md)** (CLI shim).
- Mode-driven behavior: **`ask`**, **`plan`**, **`agent`** with deterministic guardrails and approvals where required.
- **Thin client only:** consumers do **not** host the agent. Assistant reasoning runs in a **daemon-supervised sidecar**; `rex-daemon` brokers inference and tools ([MVP_SPEC.md](MVP_SPEC.md), [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md)).

---

## Mode contract

| Mode | Purpose | Mutation behavior |
|---|---|---|
| `ask` | Research and explanation | Blocks file mutations. |
| `plan` | Structured planning and handoff | Mutations only after approval checkpoints. |
| `agent` | Guarded execution flow | Approval for execution and mutations. |

Rules: one active mode per session; visible transitions.

## Stream contract

```bash
rex complete "<prompt>" --format ndjson --mode <ask|plan|agent> [--model <id>]
```

- Pass **`--mode`** on every `complete` call.
- Optional **`--model`** when passing `--model` on the CLI (otherwise daemon default applies).
- One JSON object per stdout line (`chunk`, `done`, `error`; additive non-terminal `tool`, `step`, `plan`, `activity`).
- **`rex` flushes stdout after each NDJSON line** when the consumer is on a pipe, so chunks are visible promptly instead of sitting in a block buffer.
- Exactly **one** terminal event per request path (`done` **or** `error`); `tool`, `step`, `plan`, and `activity` lines may appear mid-stream and are non-terminal.

**Additive stream events:**

| Event | Fields | Purpose |
|-------|--------|---------|
| `tool` | `index`, `name`, `phase`, `detail?`, `tool_call_id?`, `sequence?`, `elapsed_ms?`, `turn_id?` | Broker tool lifecycle (`running`, `completed`, `failed`) |
| `step` | `index`, `phase`, `summary`, `sequence?`, `turn_id?` | Orchestrator / subagent step visibility |
| `plan` | `index`, `phase`, `title`, `detail?`, `sequence?`, `turn_id?` | Plan clarify / ready events |
| `activity` | `index`, `phase`, `summary`, `detail?`, `sequence?` | Heartbeat / coarse phase (`thinking`, `tool_running`, …) |

Fixtures: [`fixtures/ndjson_contract/`](../fixtures/ndjson_contract/).

Operation feedback design hub: [OPERATION_FEEDBACK.md](OPERATION_FEEDBACK.md).

- `error` objects may expose stable `code` for routing.

**Error codes and message quality:** canonical catalog, templates, and review checklist — [ERROR_HANDLING.md](ERROR_HANDLING.md). Wire-shape table (retry guidance):

| Code | Meaning | Retry guidance |
|---|---|---|
| `daemon_unavailable` | Daemon not reachable | Retry after daemon start |
| `sidecar_unavailable` | Sidecar required but unavailable | Fix sidecar setup; see [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md) |
| `inference_config` | Inference backend not configured | Edit JSON `inference.openai_compat`; see [CONFIGURATION.md](CONFIGURATION.md) |
| `stream_timeout` | No stream activity in window | Retry usually safe |
| `stream_interrupted` | Mid-flight failure | Retry usually safe |
| `stream_incomplete` | No terminal marker | Inspect daemon; avoid blind retry |
| `cancelled` | User cancelled | No retry unless user resubmits |
| `invalid_response` | Malformed NDJSON | Inspect CLI/daemon logs |
| `spawn_failed` | `rex` could not spawn | Fix install / PATH |
| `approval_required` | Approval needed before execution | Supply `--approval-id` after user approval |
| `workspace_not_configured` | Workspace root not bound | Set workspace via CLI flags or config |
| `workspace_mismatch` | Prompt workspace does not match daemon | Rebind workspace |
| `unknown` | Uncategorized | Manual diagnosis |

## Bootstrap flow

1. Select mode. 2. Capture prompt/context. 3. Policy + approvals. 4. Run `rex complete … --format ndjson --mode <mode>`. 5. Parse lines. 6. Route `chunk` / `done` / `error` to consumer.

## Reliability and trace correlation

Bounded CLI retries for daemon-unavailable races; interrupted streams terminate with `error`. Per-request **`--trace-id`** on `rex complete` correlates CLI and daemon logs — [`CONFIGURATION.md`](CONFIGURATION.md).

## Non-goals

Using Node gRPC for **`StreamInference`** instead of **`rex` NDJSON**; plugin lifecycle inside thin clients; fully autonomous unattended execution without approvals.

## Related documents

| Document | Topic |
|---|---|
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | Whole-system architecture + ADRs |
| [`CLI_OPERATOR_UX.md`](CLI_OPERATOR_UX.md) | Terminal operator path |
| [`ERROR_HANDLING.md`](ERROR_HANDLING.md) | Error code catalog |
| [`OPERATION_FEEDBACK.md`](OPERATION_FEEDBACK.md) | Stream feedback design |
