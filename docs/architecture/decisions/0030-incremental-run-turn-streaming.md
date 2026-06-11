# ADR 0030: Incremental RunTurn streaming

- **Date:** 2026-06-11
- **Status:** Accepted

## Context

`rex-agent` collected `stream_events` in graph state during synchronous `invoke()`, then replayed them after the turn finished. Operators saw silence during multi-step agent turns despite a designed `tool` / `step` NDJSON contract.

## Decision

1. **Stream sink:** Graph nodes emit tool/step/plan/activity events to a per-turn queue when streaming is active.
2. **LangGraph `stream()`:** The streaming path uses `graph.stream(state, stream_mode="updates")` and drains the sink after each update.
3. **Collect path:** `run_turn_with_events()` keeps synchronous `invoke()` for tests and offline collection.
4. **Daemon:** Live `RunTurn` passthrough unchanged; optional pre-sidecar `step` events on `StreamInference`.

## Consequences

- **Positive:** Tool `running` events reach clients before `completed`.
- **Negative:** Ordering relies on monotonic `sequence` fields; tests must cover stream vs collect parity.

## Related

- [OPERATION_FEEDBACK.md](../../OPERATION_FEEDBACK.md)
- [ADR 0007](0007-editor-extension-hybrid-transport-cli-and-grpc.md)
