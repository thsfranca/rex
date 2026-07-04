# ADR 0035: CLI operator UX — daemon lifecycle parity and terminal UI

- **Date:** 2026-06-21
- **Status:** Accepted (design)

## Context

Operators need a local AI workspace without babysitting a foreground daemon session or thin stream feedback. The CLI connects over UDS; when the socket is down it must either auto-start the daemon or fail clearly. Interactive sessions need a full terminal UI and legible status so operators understand what Rex is doing.

Constraints:

- Daemon remains system authority ([ADR 0001](0001-daemon-owns-agent-orchestration-and-economics.md)).
- NDJSON event catalog and error codes stay stable for internal TUI consumption ([NDJSON_STREAM.md](../../NDJSON_STREAM.md), [ERROR_HANDLING.md](../../ERROR_HANDLING.md)).

## Decision

1. **CLI daemon ensure:** Before client RPCs, the CLI always ensures a daemon: probe UDS, spawn a **detached** daemon process when needed, poll **`GetSystemStatus`** until ready or timeout, and serialize concurrent ensure calls (single-flight). CLI-spawned daemons **remain running** after the CLI exits.

2. **Lifecycle vocabulary:** Clients share lifecycle states **`unavailable` → `starting` → `ready`** and the same readiness probe.

3. **Full terminal UI:** Interactive terminal sessions use a **multi-pane TUI** (bare **`rex`**). The TUI consumes the NDJSON event stream **internally**.

4. **Operator messaging (Must):** A structured mapping from lifecycle phases and NDJSON events to curated plain-language strings is **required** for the CLI operator program. This is the primary “friendly” layer—no LLM on the critical path.

5. **LLM narrator (Could):** An optional post-turn natural-language summary may be added later (**R074**), off by default (config key not in schema until shipped).

6. **Internal stream contract:** The TUI consumes the NDJSON event stream in-process. Wire shape remains defined in [NDJSON_STREAM.md](../../NDJSON_STREAM.md) for internal consumers and fixtures.

7. **Logs:** Auto-started daemons redirect stdout/stderr to **`daemon.log_path`** (default under layout root), not the operator’s interactive terminal.

Design detail and acceptance criteria: [CLI_OPERATOR_UX.md](../../CLI_OPERATOR_UX.md).

## Consequences

- **Positive:** Terminal operators can use Rex without babysitting a daemon session; shared lifecycle vocabulary; TUI projection of the stream contract; local-demo UX improves.
- **Negative:** Presentation and stream-consumer paths must stay aligned on event semantics.
- **Risks / follow-up:** Optional **launchd**/systemd tier remains out of scope.

## Related

- [CLI_OPERATOR_UX.md](../../CLI_OPERATOR_UX.md) — feature hub
- [OPERATION_FEEDBACK.md](../../OPERATION_FEEDBACK.md) — stream events
- [README.md](README.md) (ADR index)
