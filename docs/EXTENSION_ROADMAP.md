# Extension Roadmap

This document defines the forward delivery plan for the REX editor extension.

## Goals

- Keep one extension codebase on the stable `rex-cli` NDJSON contract.
- Deliver high-quality mode-driven chat (`ask`, `plan`, `agent`) with deterministic safety behavior.
- Keep guarded execution explicit: approval checkpoints for execution and mutation-capable actions.
- Keep host-specific integrations optional behind capability detection so the same VSIX works across compatible hosts.
- Build toward advanced agent workflows in phases, including guarded multi-file edits after single-file safety and mode policy are proven.

## Scope principles

- KISS: smallest design that delivers the current phase; avoid speculative abstractions.
- CLI boundary stays stable; the extension handles markdown and code-block extraction client-side.
- All new user-facing behavior defaults stay inert until opt-in, so installation never surprises users.
- Extension docs live under `docs/EXTENSION_*.md` for an easy future repo extraction.

## Host compatibility strategy

- Core path uses only standard `vscode.*` APIs.
- Host-specific features live behind capability checks in `src/platform/`.
- Optional host integrations remain isolated so baseline behavior is stable.
- MCP server registration stays deferred behind a feature flag until a REX MCP endpoint exists.

## Repository layout

- Monorepo at `extensions/rex-vscode/`.
- Self-contained folder: own `package.json`, `tsconfig.json`, `esbuild` configs, `CHANGELOG.md`, `README.md`, `LICENSE`.
- No imports into Rust crates; depends only on the `rex-cli` binary contract.
- Release versioning uses `rex-vscode-vX.Y.Z` git tags so tag history is portable.

## Delivery plan (two PRs)

### PR 1 - Mode definition + architecture/contracts foundation

Goal: define `ask`/`plan`/`agent` mode behavior and establish core contracts for mode policy, approval checkpoints, and timeline events.

| Area | Deliverable |
|---|---|
| Docs | Update extension contract docs, `EXTENSION_ARCHITECTURE`, and this roadmap for mode behavior |
| Runtime contracts | `ModePolicy`, approval request/decision events, execution-step events |
| Host orchestration | Mode orchestrator boundary and approval-gate entry points |
| Guardrails | `ask` mode blocks mutation-capable actions by policy |

Acceptance criteria:

- Mode behavior language is consistent across extension docs.
- Mode and approval contracts compile and are wired host↔webview.
- Mutation actions are blocked in `ask` mode through host policy checks.

### PR 2 - Mode UX + guarded execution hardening

Goal: deliver full user-facing mode experience and reliable checkpointed execution behavior.

| Area | Deliverable |
|---|---|
| Chat UX | Mode switcher, policy summary, approval cards, execution timeline |
| Runtime behavior | Mode-aware action handling and checkpoint approvals for guarded paths |
| Inline actions | Insert/apply behavior remains deterministic under policy constraints |
| Quality | Tests for mode transitions and approval flows, updated manual E2E checklist |

Acceptance criteria:

- Users can switch modes and understand active policy from the UI.
- Execution and mutation checkpoints require explicit user approval where defined.
- Mode transitions, cancellation, and terminal states remain deterministic.
- Extension checks stay green (`lint`, `typecheck`, `test`, packaging).

## Non-goals (current roadmap cycle)

These stay out of scope for the current cycle to keep delivery focused and reviewable:

- Multi-file coordinated edits in agent mode.
- Fully autonomous execution without user approvals.
- Inline ghost-text completions.
- Workspace `@` mentions and file indexing.
- Direct Node gRPC over UDS (CLI boundary remains the transport seam).

## What remains after this roadmap

- Continue reliability hardening and observability for long-running sessions.
- Keep release automation and install docs aligned with lifecycle changes.
- Add follow-up features only when they preserve the stable CLI NDJSON boundary.
- Introduce guarded multi-file editing workflows with explicit preview and confirmation steps.
- Add richer agent capabilities (workspace-wide planning, coordinated edits, and broader action orchestration) after the multi-file safety model is validated.

## Success signals

- DX: extension activates with zero configuration when `rex-cli` is on `PATH`.
- DX: cancellation always returns the UI to a clean idle state.
- Reliability: status bar reflects daemon state within roughly 2 seconds of activation.
- Portability: the same VSIX runs across compatible editor hosts.
- Distribution: `.vsix` publishes to Open VSX for users.

## Extraction triggers

Revisit the monorepo decision if any of the following become true:

- Extension gains a distinct audience, license, or contributor model.
- Extension release cadence diverges strongly from REX core.
- Extension CI footprint materially slows the main gate.

## Related documents

- [`ARCHITECTURE.md`](../ARCHITECTURE.md): REX system architecture.
- [`docs/EXTENSION_ARCHITECTURE.md`](EXTENSION_ARCHITECTURE.md): extension component and message-bus architecture.
