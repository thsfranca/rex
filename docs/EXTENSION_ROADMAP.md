# Extension Roadmap

This document records the phased delivery of the REX editor extension for both VS Code and Cursor and tracks what remains after that foundation shipped.

## Current purpose

- Keep one hybrid VS Code and Cursor extension on the stable `rex-cli` NDJSON contract defined in [`docs/EXTENSION_MVP.md`](EXTENSION_MVP.md).
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

## Delivery status snapshot

The initial extension plan shipped in three PRs. This section is a historical delivery record.

### PR 1 - Foundations and headless core

Goal: docs, scaffolding, CI, testable headless modules, and a minimum visible surface.  
Status: shipped.

| Area | Deliverable |
|---|---|
| Docs | `docs/EXTENSION_ROADMAP.md`, `docs/EXTENSION_ARCHITECTURE.md`, index updates |
| Scaffold | `extensions/rex-vscode/` with React 18, esbuild, TS, ESLint |
| CI | `extension-checks` job + `scripts/ci/run_extension_checks.sh` |
| Runtime | `cliBridge`, `streamClient`, `daemonLifecycle` (off by default) |
| Config | `rex.cliPath`, `rex.daemonBinaryPath`, `rex.daemonAutoStart` |
| Platform | `capabilities`, `cursorAdapter`, local `vscode.cursor` type shim |
| Editor | `editor/context` snapshot builder (no UI yet) |
| Shared | `shared/messages` typed message bus contracts |
| Visible | Status bar; commands `rex.showStatus`, `rex.howToStartDaemon`, `rex.openOutput` |
| Tests | NDJSON parser, context snapshot, capability detection |

Acceptance criteria:

- CI green for fmt, lint, type-check, unit tests, and package.
- `npm run package` produces a `.vsix`.
- Status bar reflects daemon state within roughly 2 seconds of activation.
- Activation succeeds in plain VS Code and in Cursor.

### PR 2 - Chat UX with Cursor-parity features

Goal: full user-facing chat experience.  
Status: shipped.

| Area | Deliverable |
|---|---|
| Webview | Activity bar container + React side panel with strict CSP |
| Rendering | Streaming markdown with lazy-loaded syntax-highlighted code blocks |
| Actions | Per-code-block `Copy`, `Insert at cursor`, `Apply to file` |
| Apply flow | `vscode.diff` + `TextDocumentContentProvider` + `WorkspaceEdit` |
| Context | Chip for active file and selection, detachable before send |
| Commands | `REX: Explain selection`, `REX: Fix selection`, `REX: Refactor selection` |
| Tests | `markdownStream` boundary logic, apply/diff integration tests |

Acceptance criteria:

- Streaming markdown renders without flicker in light, dark, and high-contrast.
- Copy, Insert, and Apply actions work; Apply always opens native diff first.
- Context chip attaches and detaches; selection commands prefill prompts.
- Zero remote loads and zero CSP violations in the webview.

### PR 3 - Reliability and distribution

Goal: opt-in auto-start, release pipeline, and docs polish.  
Status: shipped.

| Area | Deliverable |
|---|---|
| Reliability | `rex.daemonAutoStart` enabled path with lifecycle + tests |
| Release | Tag-triggered `vsce package` + VSIX validation (`unzip -t`, `vsce generate-manifest`); optional `ovsx publish` / `vsce publish` when secrets are set |
| Optional | Marketplace publishes gated by `OVSX_TOKEN` / `VSCE_PAT` secrets |
| Docs | `CHANGELOG.md`, install/first-run guide, troubleshooting |

Acceptance criteria:

- Auto-start succeeds on cold start; default behavior (off) unchanged.
- Tag triggers a `.vsix` artifact, VSIX validation, and optional Open VSX / Marketplace publish when tokens are configured.
- Install and troubleshooting docs reviewed and indexed.

## Non-goals

These stay out of scope to protect delivery speed and KISS:

- Agent mode, tool calling, multi-file coordinated edits.
- Inline ghost-text completions (follow-up once chat is solid).
- Workspace @-mentions and file search indexing.
- Direct Node gRPC over UDS (deferred; CLI boundary is sufficient).

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
- [`docs/EXTENSION_MVP.md`](EXTENSION_MVP.md): CLI NDJSON consumer contract.
- [`docs/EXTENSION_ARCHITECTURE.md`](EXTENSION_ARCHITECTURE.md): extension component and message-bus architecture.
