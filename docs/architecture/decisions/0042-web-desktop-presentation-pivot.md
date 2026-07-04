# ADR 0042: Web desktop presentation pivot

**Status:** Accepted  
**Date:** 2026-07-04  
**Supersedes (presentation intent):** [0039](0039-terminal-harness-presentation-and-daemon-intelligence.md), [0041](0041-tui-hybrid-compositor-and-tiered-frame-budget.md)  
**Aligns with:** [0038](0038-cli-ndjson-stream-transport.md), [0040](0040-harness-session-transcript-authority.md)

## Context

Rex shipped a **ratatui** terminal harness as the primary operator surface ([ADR 0039](0039-terminal-harness-presentation-and-daemon-intelligence.md), [ADR 0041](0041-tui-hybrid-compositor-and-tiered-frame-budget.md)). The character grid limits GPU-composited motion, semantic color validation, and premium trust signals during long inference. Planned **R090–R096** TUI visual identity work is cancelled in favor of a web-native desktop shell.

Intelligence, policy, and tool execution remain in **`rex-daemon`** per [ADR 0001](0001-daemon-owns-agent-orchestration-and-economics.md). Only the presentation layer changes.

Hub: [WEB_UI_ARCHITECTURE.md](../../WEB_UI_ARCHITECTURE.md).

## Decision

1. **Primary operator surface:** **Tauri 2** desktop application embedding a **React 19** webview. Bare **`rex`** launches or focuses this window with a native menu bar. No terminal TUI fallback.

2. **Desktop shell choice:** **Tauri 2** over Electron and Wails — native OS webview (WKWebView on macOS), Rust backend aligned with existing crates, ~30–80 MB idle RAM vs Electron’s 150–400 MB.

3. **Architecture pattern:** Preserve **Option C** from ADR 0039 — presentation client + daemon intelligence. Replace ratatui with webview; keep UDS gRPC as the client transport.

4. **IPC hybrid model:**
   - **Unary control plane:** Tauri commands → tonic UDS (`GetSystemStatus`, `FetchSessionEvents`, `RespondToToolApproval`, …).
   - **Streaming plane:** Rust backend subscribes to `StreamInference`, forwards events via `tauri::ipc::Channel` (not JSON-RPC command round-trips per chunk).
   - **Backpressure:** ring buffer per subscription on the Rust side.
   - **Reconnection:** Rust backend probes UDS and emits status events to the webview.

5. **Monorepo layout:**
   - `crates/rex-desktop/` — Tauri Rust backend (UDS proxy, daemon lifecycle, deep links).
   - `apps/rex-web/` — React presentation (stateless visualizer of stream events).
   - `crates/rex-ui-harness/` — MCP + Playwright validation for AI-agent PR gates.

6. **Stream normalization:** Reuse [`rex-stream-ui`](../../../crates/rex-stream-ui/) in the Tauri backend; webview consumes normalized effects via Channel + Zustand.

7. **Validation:** Replace tuiwright PTY snapshots with **rex-ui-harness** MCP (CIEDE2000 token asserts, Playwright clock mocking, tauri-plugin-playwright for WKWebView).

8. **Configuration:** No new JSON keys for UI mode — process context (`rex` spawn) selects the desktop app ([KISS config surface](../../CONFIGURATION.md)).

9. **Platform scope (v1):** macOS Apple Silicon first; code signing/notarization CI deferred to post-MVP (W107).

## Consequences

- **Positive:** GPU motion, fluid typography, deterministic visual regression for agent-driven UI PRs; 20×–50× smaller binary vs Electron; daemon boundary unchanged.
- **Negative:** Two new crates + npm workspace; WKWebView vs Chrome dev-server parity until native harness lands; large deletion of TUI/compositor code.
- **Risks / follow-up:** react-virtuoso scroll anchoring during streaming; CIEDE2000 threshold tuning on CI macOS runners; implement W100–W106 per [WEB_UI_ROADMAP.md](../../WEB_UI_ROADMAP.md).

## Superseded ADR amendments

- **ADR 0039:** Presentation framework decisions (ratatui, mdstream, crossterm) superseded for the product path; Option C and daemon intelligence placement remain valid.
- **ADR 0041:** TUI compositor and tiered frame budget superseded; motion tiers move to web effect graph in [WEB_UI_DESIGN.md](../../WEB_UI_DESIGN.md).
