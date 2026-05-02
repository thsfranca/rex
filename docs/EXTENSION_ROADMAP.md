# Extension Roadmap

This document records the phased delivery of the REX editor extension for both VS Code and Cursor and tracks what remains after that foundation shipped.

## Current purpose

- Keep one hybrid VS Code and Cursor extension on the **`rex-cli` NDJSON** contract summarized in **[EXTENSION.md](EXTENSION.md)**.
- Maintain Cursor-class chat UX (streaming markdown, highlighted code blocks, Apply-to-file with native diff) without forking per-editor code paths.
- Keep Cursor-specific features optional behind runtime capability detection so the same VSIX installs cleanly in plain VS Code.
- Keep daemon lifecycle user-managed by default; opt-in auto-start remains behind `rex.daemonAutoStart` (default off).

## Scope principles

- KISS: smallest design that delivers the current phase; avoid speculative abstractions.
- CLI boundary stays stable; the extension handles markdown and code-block extraction client-side.
- All new user-facing behavior defaults stay inert until opt-in, so installation never surprises users.
- Extension docs live under `docs/EXTENSION_*.md` for an easy future repo extraction.

## Hybrid editor strategy

- Core path uses only standard `vscode.*` APIs.
- Cursor-only features live in `src/platform/cursorAdapter.ts` behind `capabilities.hasCursor()`.
- First Cursor feature: `vscode.cursor.plugins.registerPath` for bundled plugin content.
- MCP server registration stays deferred behind a feature flag until a REX MCP endpoint exists.

## Repository layout

- Monorepo at `extensions/rex-vscode/`.
- Self-contained folder: own `package.json`, `tsconfig.json`, `esbuild` configs, `CHANGELOG.md`, `README.md`, `LICENSE`.
- No imports into Rust crates; depends only on the `rex-cli` binary contract.
- Release versioning uses `rex-vscode-vX.Y.Z` git tags so tag history is portable.

## Delivery snapshot

Foundation milestones (three internal delivery waves covering scaffold + chat UX + packaging) shipped; **git tag / PR history holds file-level receipts**. Operational install path: **[EXTENSION_RELEASE.md](EXTENSION_RELEASE.md)**, checklist **[EXTENSION_LOCAL_E2E.md](EXTENSION_LOCAL_E2E.md)**.

## Non-goals

- Unattended **multi-file coordinated agent runs** **without** user approvals (thin extension cannot replace daemon-side tool policy).
- **MCP-native tool orchestration inside the extension** until `rex-daemon` exposes an MCP-aligned endpoint (`ARCHITECTURE.md` interoperability rows).
- Inline ghost-text completions.
- Workspace @-mentions requiring bespoke indexing servers.
- Direct Node **gRPC** transport (stay on **`rex-cli`** boundary).

## What remains after phase delivery

- Continue **stress hardening** for very long chat sessions (output channel / queue pressure) and rare status-bar edge cases. **stderr** from `rex-cli` is **capped** for exit-code messages (`cappedString` / `streamClient`). **Status bar** `unavailable` tooltips **elide** overlong `reason` text (`src/ui/statusBar.ts` + `elideForTooltip`) so huge spawn errors or wrapped logs do not blow past reasonable tooltip size. The **core** path still enforces a single NDJSON terminal event, cancel-to-idle, and structured chat logging (see `src/runtime/streamClient.ts` and tests under `src/test/`).
- Keep release automation and install docs aligned with CLI/daemon lifecycle changes.
- Add follow-up features only when they preserve the stable CLI NDJSON boundary and keep behavior symmetric across Cursor and plain VS Code.

## Success signals

- DX: extension activates with zero configuration when `rex-cli` is on `PATH`.
- DX: cancellation always returns the UI to a clean idle state.
- Reliability: status bar reflects daemon state within roughly 2 seconds of activation.
- Portability: the same VSIX runs in plain VS Code and in Cursor.
- Distribution: `.vsix` publishes to Open VSX for Cursor users.

## Extraction triggers

Revisit the monorepo decision if any of the following become true:

- Extension gains a distinct audience, license, or contributor model.
- Extension release cadence diverges strongly from REX core.
- Extension CI footprint materially slows the main gate.

## Related documents

- [ARCHITECTURE.md](ARCHITECTURE.md): REX system architecture.
- [MVP_SPEC.md](MVP_SPEC.md): phase 1 protocol and acceptance criteria.
- **[EXTENSION.md](EXTENSION.md)**: consolidated extension contract + component layout.
