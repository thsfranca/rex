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
