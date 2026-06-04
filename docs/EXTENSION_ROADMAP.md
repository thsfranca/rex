# Extension Roadmap

This document records the phased delivery of the REX editor extension for both VS Code and Cursor and tracks what remains after that foundation shipped.

## Current purpose

- Keep one hybrid VS Code and Cursor extension on the **`rex-cli` NDJSON** contract summarized in **[EXTENSION.md](EXTENSION.md)**; optional unary **`rex.v1`** over UDS is allowed per **[ADR 0007](architecture/decisions/0007-editor-extension-hybrid-transport-cli-and-grpc.md)** when maintainers choose it.
- Deliver **integrated editor+agent UX** per **[EXTENSION_UX.md](EXTENSION_UX.md)** (webview-first; stable CLI boundary).
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
- **MCP-native tool orchestration inside the extension** until REX ships **brokered** MCP/tool capability via the **sidecar ↔ daemon** story ([ADR 0008](architecture/decisions/0008-dedicated-sidecar-control-plane-api.md), [ARCHITECTURE.md](ARCHITECTURE.md) interoperability, [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) matrix); formal MCP ADR when that implementation is scheduled.
- Inline ghost-text completions.
- Workspace @-mentions requiring bespoke indexing servers.
- **Node `StreamInference` streaming** replacing the NDJSON **`rex-cli`** path (would need a **future ADR**; hybrid unary-only policy is **[ADR 0007](architecture/decisions/0007-editor-extension-hybrid-transport-cli-and-grpc.md)**).
- Native VS Code **Chat Participant** as primary surface (webview-first per [EXTENSION_UX.md](EXTENSION_UX.md)).

## Shipped (foundation + recent slices)

- **Daemon approval context:** extension passes `--approval-id` on **agent** sends when execution approval is required and `REX_AGENT_APPROVALS=1` ([ADR 0009](architecture/decisions/0009-centralized-agent-approvals-and-checkpoints.md)). **Plan** mode uses in-UI mutation approvals only (no `--approval-id` on the CLI path).
- Get Started walkthrough documents **sidecar + HTTP** env for plan/agent.
- **`rex.modelId`** setting passes `--model` on every `complete` when non-empty.
- **Core** path: single NDJSON terminal, cancel-to-idle, **`--mode`** on every `complete`.
- **RC-S2 Met:** automated cancel-to-idle coverage in [`appState.test.ts`](../extensions/rex-vscode/src/test/appState.test.ts) and [`chatPanel.test.ts`](../extensions/rex-vscode/src/test/chatPanel.test.ts).

## Extension UX program (E-UX01…E-UX11)

Canonical design: **[EXTENSION_UX.md](EXTENSION_UX.md)**. One PR slice per row where feasible; merge-wait between slices.

| ID | Phase | Feature | Source(s) | Status |
|----|-------|---------|-----------|--------|
| **E-UX01** | P1 | Secondary sidebar chat + activity-bar fallback (VS Code ≥1.106) | [EXTENSION_UX.md](EXTENSION_UX.md) §E-UX01 | Done |
| **E-UX02** | P1 | “Open REX in Editor” panel (`createWebviewPanel`) | [EXTENSION_UX.md](EXTENSION_UX.md) §E-UX02 | Done |
| **E-UX03** | P1 | Theme / a11y / narrow-width webview hardening | [EXTENSION_UX.md](EXTENSION_UX.md) §E-UX03 | Done |
| **E-UX04** | P1 | Keybindings + Get Started walkthrough updates | [EXTENSION_UX.md](EXTENSION_UX.md) §E-UX04 | Done |
| **E-UX05** | P2 | Persisted chat sessions (workspace-scoped) | [EXTENSION_UX.md](EXTENSION_UX.md) §E-UX05 | Done |
| **E-UX06** | P2 | @-style context picker (files + document symbols) | [EXTENSION_UX.md](EXTENSION_UX.md) §E-UX06 | Done |
| **E-UX07** | P2 | Composer slash commands (`/ask`, `/plan`, `/agent`, `/clear`) | [EXTENSION_UX.md](EXTENSION_UX.md) §E-UX07 | Planned |
| **E-UX08** | P2 | Terminal selection → REX context menu | [EXTENSION_UX.md](EXTENSION_UX.md) §E-UX08 | Planned |
| **E-UX09** | P3 | Tool / step cards from structured host events | [EXTENSION_UX.md](EXTENSION_UX.md) §E-UX09 | Planned |
| **E-UX10** | P3 | Inline edit on selection (virtual-doc apply path) | [EXTENSION_UX.md](EXTENSION_UX.md) §E-UX10 | Planned |
| **E-UX11** | P3 | Multi-file diff review batch | [EXTENSION_UX.md](EXTENSION_UX.md) §E-UX11 | Planned |

## What remains (non-UX)

- Keep release automation and install docs aligned with CLI/daemon lifecycle changes.
- Add follow-up features only when they preserve the stable CLI NDJSON boundary and keep behavior symmetric across Cursor and plain VS Code.

## Success signals

- DX: extension activates with zero configuration when `rex-cli` is on `PATH`.
- DX: cancellation always returns the UI to a clean idle state.
- Reliability: status bar reflects daemon state within roughly 2 seconds of activation.
- Portability: the same VSIX runs in plain VS Code and in Cursor.
- Distribution: `.vsix` publishes to Open VSX for Cursor users.
- UX: operator can chat beside the editor and attach file/symbol context without a custom index server ([EXTENSION_UX.md](EXTENSION_UX.md) acceptance).

## Extraction triggers

Revisit the monorepo decision if any of the following become true:

- Extension gains a distinct audience, license, or contributor model.
- Extension release cadence diverges strongly from REX core.
- Extension CI footprint materially slows the main gate.

## Related documents

- [ARCHITECTURE.md](ARCHITECTURE.md): REX system architecture.
- [MVP_SPEC.md](MVP_SPEC.md): Phase 1 product architecture; done: [V1_0.md](V1_0.md).
- **[EXTENSION.md](EXTENSION.md)**: consolidated extension contract + component layout.
- **[EXTENSION_UX.md](EXTENSION_UX.md)**: integrated extension UX design hub.
