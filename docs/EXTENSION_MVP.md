# Extension MVP Consumer Path

This guide defines the current MVP for the REX editor extension.

## Goals

- Keep one stable extension boundary through `rex-cli`.
- Support three user-facing modes: `ask`, `plan`, and `agent`.
- Deliver guardrailed execution with explicit approvals for execution and mutation-capable operations.
- Keep UX deterministic so mode transitions and terminal states remain predictable.

## MVP behavior contract

### Mode contract

| Mode | Purpose | Mutation behavior |
|---|---|---|
| `ask` | Research and explanation | Blocks file mutations. |
| `plan` | Structured planning and handoff | Allows mutations only after approval checkpoints. |
| `agent` | Guarded execution flow | Requires approval for execution and mutations. |

Rules:

- Exactly one active mode per session.
- Mode transitions are explicit and visible in the chat UI.
- Inline actions (insert/apply) always respect the active mode policy.

### Stream contract

Use:

```bash
rex-cli complete "<prompt>" --format ndjson
```

Output contract:

- One JSON object per line.
- Event types: `chunk`, `done`, `error`.
- Exactly one terminal event (`done` or `error`) per request path.
- `error` events may include a stable `code` for extension-side UX mapping.

Current `error.code` taxonomy:

| Code | Meaning | Retry guidance |
|---|---|---|
| `daemon_unavailable` | Daemon socket/process not reachable. | Retry after daemon start/check. |
| `stream_timeout` | No stream item arrived within timeout. | Retry is usually safe. |
| `stream_interrupted` | Stream failed mid-flight. | Retry is usually safe. |
| `stream_incomplete` | Stream ended without terminal marker. | Do not auto-retry; inspect daemon logs. |
| `cancelled` | User-initiated cancellation. | No retry needed unless user resubmits. |
| `invalid_response` | Malformed or unknown NDJSON payload. | Treat as contract error; inspect CLI/daemon logs. |
| `spawn_failed` | `rex-cli` could not be launched. | Fix local install/path first. |
| `unknown` | Fallback category for uncategorized failures. | Manual diagnosis required. |

## Extension bootstrap flow

1. User selects mode (`ask`, `plan`, `agent`) in chat.
2. Extension captures prompt and optional editor context.
3. Extension enforces mode policy and approval checkpoints.
4. Extension starts `rex-cli complete "<prompt>" --format ndjson`.
5. Extension reads stdout line-by-line.
6. Extension routes events:
   - `chunk`: append streamed text to UI buffer.
   - `done`: finalize UI state as success.
   - `error`: finalize UI state as failure and show message.

## Reliability expectations

- Startup race: CLI performs bounded retries for daemon-unavailable startup window.
- Non-retryable errors: CLI fails fast and emits terminal `error`.
- Interrupted stream: CLI emits terminal `error` and exits non-zero.
- The extension serializes cancel/send transitions so one stream ID resolves with one terminal UI state.
- Approval-required operations never execute without explicit user approval.

## Trace correlation and terminal latency

- The extension runtime creates a per-request trace id and passes it to `rex-cli` via `REX_TRACE_ID`.
- `rex-cli` forwards trace metadata (`x-rex-trace-id`) to daemon stream requests.
- Daemon logs include `trace_id` and terminal `elapsed_ms` for completion and failure paths.
- Extension host logs include start/terminal trace lifecycle markers for cross-process triage.

## Non-goals for this MVP

- Direct extension gRPC transport.
- Plugin runtime management inside extension.
- Fully autonomous execution without approval checkpoints.
- Multi-file coordinated edits.
