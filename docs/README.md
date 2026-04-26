# Documentation Index

This folder keeps project documentation organized by purpose.

Use `README.md` at the repository root as the runnable onboarding path (understand -> run -> verify -> troubleshoot). Use this index to choose deeper references after first setup.

## Structure

| Path | Purpose |
|---|---|
| `docs/README.md` | Entry point for documentation. |
| `docs/DOCUMENTATION.md` | Project writing checklist and standards. |
| `docs/DEPENDENCIES.md` | Local build and runtime dependency checklist. |
| `docs/CI.md` | Path-aware CI strategy and `main` branch protection settings. |
| `docs/EXTENSION_MVP.md` | Extension bootstrap flow using `rex-cli` NDJSON stream output. |
| `docs/EXTENSION_ROADMAP.md` | Phased roadmap for the VS Code/Cursor extension. |
| `docs/EXTENSION_ARCHITECTURE.md` | Extension component layout and typed message bus. |
| `docs/EXTENSION_RELEASE.md` | Install, auto-start, troubleshooting, and release pipeline for the extension. |
| `docs/EXTENSION_LOCAL_E2E.md` | Ordered checklist from clone to **REX ready** in the editor (daemon, CLI, PATH, extension install). |
| `scripts/install-extension.sh` | One-command local VSIX build + install into Cursor/VS Code (see `docs/EXTENSION_RELEASE.md`). |
| `scripts/dev-rex-extension.sh` | Build Rust workspace, `install-cli.sh`, then `install-extension.sh` with pass-through args (see `docs/EXTENSION_LOCAL_E2E.md`). |
| `docs/PLUGIN_ROADMAP.md` | Sidecar-first plugin roadmap and built-in decision gates. |
| `docs/ADAPTERS.md` | Inference adapter contract, Cursor CLI profile, capabilities checklist. |
| `docs/CACHING.md` | Layered response cache: keys, mode safety, bypass, optional semantic layer. |
| `docs/CONTEXT_EFFICIENCY.md` | Token budget, plugin boundaries, and telemetry defaults. |
| `docs/CONFIGURATION.md` | Precedence, `REX_*` catalog, and roadmap for flags and user config. |
| `ARCHITECTURE.md` | System architecture and technical direction. |
| `MVP_SPEC.md` | Phase 1 specification and success criteria. |

## Source vs generated artifacts

- Treat docs and source files as canonical; generated/runtime outputs are disposable.
- Do not commit local runtime/build artifacts such as `target/`, temporary test outputs, or ad-hoc local logs.
- Extension packaging can generate `extensions/rex-vscode/rex-vscode.vsix` for local install and release workflows; treat it as build output unless a release process explicitly requires it.

## How to use this documentation

- Start with `README.md` for project context.
- Read `ARCHITECTURE.md` for long-term design.
- Read `MVP_SPEC.md` for current implementation scope.
- Read `docs/CONFIGURATION.md` for the configuration policy, `REX_*` variables, and future precedence.
- Read `docs/EXTENSION_MVP.md` for MVP extension integration contract.
- Read `docs/EXTENSION_ROADMAP.md` for the phased plan of the VS Code/Cursor extension.
- Read `docs/EXTENSION_ARCHITECTURE.md` for the extension component layout and message bus.
- Read `docs/EXTENSION_RELEASE.md` to install the extension, enable daemon auto-start, or ship a release.
- Read `docs/EXTENSION_LOCAL_E2E.md` for a single path from clone to a working extension plus daemon (includes macOS editor `PATH` notes).
- Run `./scripts/install-extension.sh` from the repo root for the fastest local install into Cursor or VS Code (see that doc for prerequisites).
- Read `docs/PLUGIN_ROADMAP.md` for post-MVP plugin sequence, Cursor CLI adapter phases, and promotion criteria.
- Read `docs/ADAPTERS.md` and `docs/CACHING.md` for the inference-adapter seam and response-cache design.
- Read `docs/CONTEXT_EFFICIENCY.md` for context optimization architecture, capability-aware policy, and contracts.
- Use `REX_INFERENCE_RUNTIME` and the Cursor adapter env flags documented in `docs/PLUGIN_ROADMAP.md` and `docs/DEPENDENCIES.md` to exercise the Cursor path locally; keep mock as the default in CI.
- Read `docs/DEPENDENCIES.md` to set up local prerequisites.
- Read `docs/CI.md` for CI gate and merge queue settings.
- Follow `docs/DOCUMENTATION.md` before editing docs.
