# ADR 0035: CLI operator UX — daemon lifecycle parity and terminal UI

- **Date:** 2026-06-21
- **Status:** Accepted (design)

## Context

Terminal operators today run **`rex daemon`** in a foreground session before **`rex status`** or **`rex complete`**. That session blocks the terminal and prints daemon logs. The Rust CLI connects over UDS only; on failure it returns **`daemon_unavailable`** and instructs the user to start the daemon manually.

The rex CLI already implements opt-in auto-start: probe the socket, spawn **`rex daemon`** if needed, poll until ready, optionally tear down an owned child on deactivate ([`src/runtime/daemonLifecycle.ts`](../../../src/runtime/daemonLifecycle.ts)). CLI and extension lifecycle semantics diverge, which hurts operators who use both surfaces.

CLI feedback during streams is thin compared to the extension: **`--verbose`** stderr lines and NDJSON forwarding ([OPERATION_FEEDBACK.md](../../OPERATION_FEEDBACK.md)). Operators asked for a **full terminal UI** and **legible status** so they understand what Rex is doing without a dedicated daemon terminal.

Constraints:

- Daemon remains system authority ([ADR 0001](0001-daemon-owns-agent-orchestration-and-economics.md)).
- Extension keeps **`rex complete --format ndjson`** as the primary streaming path ([ADR 0007](0007-editor-extension-hybrid-transport-cli-and-grpc.md)).
- NDJSON event catalog and error codes stay stable ([NDJSON_STREAM.md](../../NDJSON_STREAM.md), [ERROR_HANDLING.md](../../ERROR_HANDLING.md)).

## Decision

1. **CLI daemon ensure:** When merged JSON **`daemon.auto_start`** is **`true`** (default **`true`**), the CLI may spawn a **detached** **`rex daemon`** before client RPCs, poll **`GetSystemStatus`** until ready or timeout, and serialize concurrent ensure calls (single-flight). CLI-spawned daemons **remain running** after the CLI exits; the extension may still kill only the child it owns on deactivate. Opt out: **`daemon.auto_start: false`** or **`--no-daemon-autostart`**.

2. **Extension alignment:** CLI and extension share lifecycle states **`unavailable` → `starting` → `ready`**, the same spawn command (**`rex daemon`**), and the same readiness probe. JSON **`daemon.auto_start`** is the canonical config key; **`rex.daemonAutoStart`** mirrors it for editor sessions.

3. **Full terminal UI:** Interactive terminal sessions use a **multi-pane TUI** (**`rex tui`**, and TTY **`rex complete`** when **`cli.ui.enabled`** is **`auto`** or **`true`**). The TUI consumes the same NDJSON event stream internally as the extension subprocess path.

4. **Operator messaging (Must):** A structured mapping from lifecycle phases and NDJSON events to curated plain-language strings is **required** for the CLI operator program. This is the primary “friendly” layer—no LLM on the critical path.

5. **LLM narrator (Could):** An optional post-turn natural-language summary may be added later (**R074**), off by default (config key not in schema until shipped).

6. **Automation path preserved:** **`rex complete --format ndjson`** on non-TTY stdout remains the canonical extension, CI, and scripting contract. TUI must not alter NDJSON wire shape or break pipe consumers.

7. **Logs:** Auto-started daemons redirect stdout/stderr to **`daemon.log_path`** (default under layout root), not the operator’s interactive terminal. Foreground **`rex daemon`** remains for debugging.

Design detail and acceptance criteria: [CLI_OPERATOR_UX.md](../../CLI_OPERATOR_UX.md).

## Consequences

- **Positive:** Terminal operators can use Rex without babysitting a daemon session; shared lifecycle vocabulary with the extension; TUI parity with extension tool/activity visibility; local-demo UX improves.
- **Negative:** Two CLI presentation paths (TUI vs NDJSON pipe) to test; spawn race handling required despite socket bind exclusivity.
- **Risks / follow-up:** Implement **R071–R073** in order; evaluate TUI library and markdown rendering in **R073**; document extension ↔ JSON config precedence; optional **launchd**/systemd tier remains out of scope.

## Related

- [CLI_OPERATOR_UX.md](../../CLI_OPERATOR_UX.md) — feature hub
- [ADR 0007](0007-editor-extension-hybrid-transport-cli-and-grpc.md) — hybrid transport
- [OPERATION_FEEDBACK.md](../../OPERATION_FEEDBACK.md) — stream events
- [CLI_OPERATOR_UX.md](../../CLI_OPERATOR_UX.md) — extension auto-start
- [README.md](README.md) (ADR index)
