# ADR 0039: Terminal harness — presentation layer and daemon intelligence

**Status:** Superseded by [0042](0042-web-desktop-presentation-pivot.md) (presentation framework)  
**Date:** 2026-07-01  
**Supplements:** [0035-cli-operator-ux-daemon-lifecycle-and-terminal-ui.md](0035-cli-operator-ux-daemon-lifecycle-and-terminal-ui.md)  
**Aligns with:** [0038-cli-ndjson-stream-transport.md](0038-cli-ndjson-stream-transport.md)

## Context

[ADR 0035](0035-cli-operator-ux-daemon-lifecycle-and-terminal-ui.md) accepted daemon auto-start and a multi-pane TUI intent. [ADR 0038](0038-cli-ndjson-stream-transport.md) made **`rex complete --format ndjson`** the primary client contract after in-repo extension removal. Deep design research (2026-07) resolved open questions on rendering stack, async topology, intelligence placement, and phased delivery for **R072–R073**.

Hubs: [CLI_OPERATOR_UX.md](../../CLI_OPERATOR_UX.md), [TERMINAL_HARNESS_ARCHITECTURE.md](../../TERMINAL_HARNESS_ARCHITECTURE.md).

## Decision

1. **Architecture pattern:** **Option C** — CLI is a **`ratatui`** presentation layer connected via UDS gRPC; LSP, git orchestration, memory retrieval, and MCP policy registration live in **`rex-daemon`**.

2. **TUI framework:** **`ratatui`** with **`crossterm`** backend (immediate-mode IMGUI).

3. **Streaming markdown:** **`mdstream`** incremental parser (committed blocks + pending tail)—not full-buffer reparsing per chunk.

4. **Async design:** Dedicated tokio NDJSON consumer task → **`tokio::sync::mpsc`** → UI draw loop; no blocking I/O on the UI thread.

5. **Transport:** Interactive TUI uses UDS gRPC for streams and unary control (cancel, approval, model override). **`rex complete --format ndjson`** on non-TTY stdout remains the canonical automation contract unchanged.

6. **NDJSON parity:** Core event-loop parsing and tool-output truncation in TUI must match the automation subprocess path.

7. **LSP runtime:** Language server clients run in **`rex-daemon`**, scoped per workspace (**R076** Later).

8. **Git pre-edit policy:** When **`git.auto_commit_dirty`** is true (default), daemon broker auto-commits dirty files before approving **`fs.write`** (**R077** Later).

9. **Memory index:** SQLite FTS5 for project memory resides under per-project **`.rex/`** per [ADR 0014](0014-long-term-memory-boundary.md).

10. **Synchronized output:** Emit terminal **`?2026`** sequences when **`cli.ui.sync_output`** is true and terminal supports it; degrade gracefully otherwise.

11. **Modes and permissions:** **`Shift+Tab`** cycles `ask` / `plan` / `agent`; `/mode` slash command as fallback. **`Ctrl+Y`** toggles mid-session bypass for non-destructive file mutations.

12. **MCP approval UI:** TUI generates modals dynamically from MCP JSON schemas (**R078** Later).

13. **Daemon recovery:** On **`daemon_unavailable`**, TUI preserves transcript and offers auto-restart with operator confirmation.

14. **Theming:** Adaptive colors from terminal environment capabilities—not a single hardcoded palette.

15. **Cancellation:** Single **Ctrl+C** cancels the current turn (same as Esc); double **Ctrl+C** exits the CLI.

## Consequences

- **Positive:** Terminal becomes the primary operator surface with deterministic NDJSON projection; daemon intelligence shared across all clients; clear phased path **R072** → **R073** → v2 rows.
- **Negative:** Two presentation paths (TUI vs NDJSON pipe) require parity tests; **`ratatui`** + **`mdstream`** add CLI dependencies.
- **Risks / follow-up:** Implement **R072–R073** in order; **R076–R078** after TUI MVP; LangFuse program deferred until harness lands per roadmap pivot.

## Related

- [CLI_OPERATOR_UX.md](../../CLI_OPERATOR_UX.md)
- [TERMINAL_HARNESS_ARCHITECTURE.md](../../TERMINAL_HARNESS_ARCHITECTURE.md)
- [ADR 0035](0035-cli-operator-ux-daemon-lifecycle-and-terminal-ui.md)
- [ADR 0038](0038-cli-ndjson-stream-transport.md)
- [NDJSON_STREAM.md](../../NDJSON_STREAM.md)
