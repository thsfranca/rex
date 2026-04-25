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

Examples:

```json
{"event":"chunk","index":0,"text":"mock: he"}
{"event":"chunk","index":1,"text":"llo"}
{"event":"done","index":2}
```

```json
{"event":"error","message":"daemon is unavailable at /tmp/rex.sock; start rex-daemon and retry"}
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

## Out of scope for MVP

- Full extension UX polish.
- Direct extension gRPC transport.
- Plugin runtime management inside extension.
