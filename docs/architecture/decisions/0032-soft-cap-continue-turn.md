# ADR 0032: Soft cap pause and ContinueTurn

- **Date:** 2026-06-13
- **Status:** Superseded by [ADR 0034](0034-remove-tool-step-caps.md)

## Context

Agent tool loops hit hard step caps (`agent.max_tool_steps_*`) without operator control. Cap-terminal turns waste partial progress and frustrate simple research prompts. R063 adds a soft threshold before the hard limit with an explicit Continue action.

## Decision

1. **Threshold:** When `agent.soft_cap_enabled` is true, pause before the next productive tool batch when `tool_steps >= int(max_steps * soft_cap_fraction)` and `tool_steps < max_steps`.
2. **Pause surface:** Sidecar emits NDJSON `activity` with `phase=awaiting_continue` and a `continue_token` in `detail`. The turn stream ends with `done=true` (soft pause, not hard cap-terminal).
3. **Checkpoint:** Sidecar stores serialized graph state in an in-process map keyed by `continue_token` (single-process; not durable across restarts).
4. **ContinueTurn:** New `rex.sidecar.v1.ContinueTurn` RPC loads the checkpoint, extends `max_steps` by `+10`, clears the pause flag, and resumes `graph.stream()` from saved state.
5. **Daemon correlation:** `StreamInferenceRequest.continue_token` passthrough to sidecar `ContinueTurn`; daemon owns turn/trace correlation on the stream envelope.
6. **Defaults (R063):** Ask/plan/agent caps raised to 15/25/25; `soft_cap_enabled` default true; fraction default 2/3 (thresholds 10/16/16).

## Consequences

- **Positive:** Operators can extend budget without restarting context assembly; extension and CLI reuse existing NDJSON activity handling.
- **Negative:** Checkpoints are process-local; sidecar restart invalidates tokens. Multi-instance sidecar fleets need a follow-up durable store.

## Related

- [AGENT_GRAPH_ARCHITECTURE.md](../../AGENT_GRAPH_ARCHITECTURE.md#loop-optimization-r060r065)
- [OPERATION_FEEDBACK.md](../../OPERATION_FEEDBACK.md)
- [ADR 0030](0030-incremental-run-turn-streaming.md)
