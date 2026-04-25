# Documentation Index

This folder keeps project documentation organized by purpose.

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
| `docs/PLUGIN_ROADMAP.md` | Sidecar-first plugin roadmap and built-in decision gates. |
| `docs/CONTEXT_EFFICIENCY.md` | Token budget, plugin boundaries, and telemetry defaults. |
| `ARCHITECTURE.md` | System architecture and technical direction. |
| `MVP_SPEC.md` | Phase 1 specification and success criteria. |

## How to use this documentation

- Start with `README.md` for project context.
- Read `ARCHITECTURE.md` for long-term design.
- Read `MVP_SPEC.md` for current implementation scope.
- Read `docs/EXTENSION_MVP.md` for MVP extension integration contract.
- Read `docs/EXTENSION_ROADMAP.md` for the phased plan of the VS Code/Cursor extension.
- Read `docs/EXTENSION_ARCHITECTURE.md` for the extension component layout and message bus.
- Read `docs/EXTENSION_RELEASE.md` to install the extension, enable daemon auto-start, or ship a release.
- Read `docs/PLUGIN_ROADMAP.md` for post-MVP plugin sequence and promotion criteria.
- Read `docs/CONTEXT_EFFICIENCY.md` for context optimization architecture and contracts.
- Read `docs/DEPENDENCIES.md` to set up local prerequisites.
- Read `docs/CI.md` for CI gate and merge queue settings.
- Follow `docs/DOCUMENTATION.md` before editing docs.
