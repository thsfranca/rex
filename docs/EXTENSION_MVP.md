# Extension MVP Consumer Path

This guide defines the first integration path for a Cursor extension.

## Goal

- Use `rex-cli` as the extension boundary.
- Stream completion output in a machine-readable format.
- Keep extension code thin while MVP daemon behavior stabilizes.

## Why this path in MVP

| Option | MVP decision | Reason |
|---|---|---|
| Direct Node gRPC over UDS | Deferred | Adds transport complexity during early reliability work. |
| Extension -> CLI (`ndjson`) | Chosen | Fastest path to a usable and testable feature. |

## Command contract

Use:

```bash
rex-cli complete "<prompt>" --format ndjson
```

Output contract:

- One JSON object per line.
- Event types: `chunk`, `done`, `error`.
- Exactly one terminal event (`done` or `error`).
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

Examples:

```json
{"event":"chunk","index":0,"text":"mock: he"}
{"event":"chunk","index":1,"text":"llo"}
{"event":"done","index":2}
```

```json
{"event":"error","message":"daemon is unavailable at /tmp/rex.sock; start rex-daemon and retry","code":"daemon_unavailable"}
```

## Extension bootstrap flow

1. Extension command collects prompt text.
2. Extension starts `rex-cli complete "<prompt>" --format ndjson`.
3. Extension reads stdout line-by-line.
4. Extension routes events:
   - `chunk`: append streamed text to UI buffer.
   - `done`: finalize UI state as success.
   - `error`: finalize UI state as failure and show message.
5. Extension handles process exit as a secondary signal only.

## Reliability expectations

- Startup race: CLI performs bounded retries for daemon-unavailable startup window.
- Non-retryable errors: CLI fails fast and emits terminal `error`.
- Interrupted stream: CLI emits terminal `error` and exits non-zero.
- The extension serializes cancel/send transitions so one stream ID resolves with one terminal UI state.

## Trace correlation and terminal latency

- The extension runtime creates a per-request trace id and passes it to `rex-cli` via `REX_TRACE_ID`.
- `rex-cli` forwards trace metadata (`x-rex-trace-id`) to daemon stream requests.
- Daemon logs include `trace_id` and terminal `elapsed_ms` for completion and failure paths.
- Extension host logs include start/terminal trace lifecycle markers for cross-process triage.

## Out of scope for MVP

- Full extension UX polish.
- Direct extension gRPC transport.
- Plugin runtime management inside extension.
