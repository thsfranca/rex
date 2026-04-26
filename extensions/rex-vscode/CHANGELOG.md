# Changelog

All notable changes to the REX VS Code/Cursor extension are tracked here.

The format loosely follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions use `rex-vscode-vX.Y.Z` git tags so the extension history is portable.

## [Unreleased]

### Added
- Foundations scaffold: extension host entry, typed message bus contracts, settings accessor, NDJSON stream client, daemon lifecycle probe, capability detection, editor context snapshot.
- Status bar with `ready`, `starting`, `unavailable` states.
- Commands `REX: Show Daemon Status`, `REX: How to Start the Daemon`, `REX: Open Output Channel`.
- Activity-bar `REX` view container with a chat webview backed by React 18 and strict CSP (nonce-scoped script, no remote loads).
- Streaming markdown renderer with safe-boundary re-parsing and lazy `shiki` syntax highlighting for common languages.
- Per-code-block actions (`Copy`, `Insert at cursor`, `Apply to file`). Apply opens VS Code's native `vscode.diff` backed by a `rex-proposal:` virtual document and writes via `WorkspaceEdit` only after Accept.
- Context chip in the chat composer showing active file + selection; user can detach before sending.
- Commands `REX: Open Chat`, `REX: Clear Chat`, `REX: Explain/Fix/Refactor Selection` (the selection commands prefill a prompt template and focus the chat view).
- Theme forwarding (light / dark / high-contrast) so the webview restyles with the editor.
- Opt-in daemon auto-start (`rex.daemonAutoStart`): at activation or when the setting flips on, the extension spawns `rex-daemon`, polls `rex-cli status` until ready, and tears the child down on deactivate. Default remains off.
- Integration tests for `DaemonLifecycle.ensureRunning` covering already-ready probe, failed spawn, and shutdown of a lifecycle-owned child.
- Release pipeline `.github/workflows/extension-release.yml` triggered by `rex-vscode-vX.Y.Z` tags: builds + tests, packages a versioned VSIX, validates it with `unzip -t` and `vsce generate-manifest`, publishes to Open VSX when `OVSX_TOKEN` is set, optionally publishes to the VS Code Marketplace when `VSCE_PAT` is set, and attaches the VSIX to a GitHub Release.
- `docs/EXTENSION_RELEASE.md` covering install, auto-start guidance, troubleshooting, and the release checklist.
- Repo script `scripts/install-extension.sh` for one-command local VSIX build, CLI install into Cursor or VS Code, and optional window reload after install.
- `docs/EXTENSION_LOCAL_E2E.md` and `scripts/dev-rex-extension.sh`: ordered path from clone to **REX ready** in the editor (Rust build, `install-cli.sh`, extension install; daemon steps documented, not auto-started by the script).

### Changed
- When `rex-cli` or `rex-daemon` cannot be spawned (`ENOENT`), error text now points to `rex.cliPath` / `rex.daemonBinaryPath` and to `docs/EXTENSION_LOCAL_E2E.md` in this repository for local setup.
- `streamComplete`: clearer terminal-state handling for `done` / `error` and cancellation wakeups.
- Periodic daemon refresh uses the same auto-start vs probe policy as activation.
- Chat streaming waits for a ready daemon when `rex.daemonAutoStart` is enabled.
- Stream error handling now uses a stable code taxonomy (`daemon_unavailable`, `stream_timeout`, `stream_interrupted`, `stream_incomplete`, `cancelled`, `invalid_response`, `spawn_failed`, `unknown`) for deterministic extension UX behavior.
- Each chat stream now carries a trace id across extension host -> `rex-cli` -> daemon logs, and terminal logs include elapsed latency.

### Fixed
- Status bar: very long `unavailable` reasons are elided in the hover tooltip to keep the status surface readable.
- `DaemonLifecycle.ensureRunning` serializes overlapping calls so concurrent chat or status work cannot spawn duplicate daemon children.
- Optional `daemonEnv` merged only into the spawned `rex-daemon` process environment.

### Added (tests / tooling)
- Vitest temp dir `.vitest-tmp/` gitignored; integration tests for flaky status fixtures and ensureRunning single-flight behavior.
- Contract conformance tests for malformed NDJSON, duplicate terminal markers, cancellation terminal uniqueness, and lifecycle trace callbacks.
