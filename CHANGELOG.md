# Changelog

All notable changes to the REX workspace (Rust daemon, CLI, sidecar stub, and `rex.v1` contract) are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions use git tags `vX.Y.Z` on `main`.

## [Unreleased]

### Changed

- **`rex config init`** operator template enables mock web search (`search.enabled: true`, `search.provider: mock`).
- Install scripts always attempt `rex-agent` setup (warn when pip missing); removed `--no-agent` / `rex.productAgentConfig` opt-in flags.
- Missing `rex-agent` errors include install hints (`rex sidecar doctor`, daemon auto-start, bare `rex` desktop launch).

## [1.0.0] - (awaiting tag)

Workspace version **`1.0.0`** preparatory in tree; **`v1.0.0` git tag** lands when a maintainer merges the release-plz Release PR **after** all Must **RC-*** rows are **Met** (**RC-01–RC-10** and **RC-S3–RC-S5**) — [RELEASE.md](docs/RELEASE.md), [V1_0.md](docs/V1_0.md).

### Added

- Unified **`rex`** binary with setup subcommands (`config`, `proto`, `sidecar`, `gateway`, `omlx`); bare `rex` launches the Tauri desktop on macOS.
- JSON-first config (`rex config init`); legacy `REX_*` env vars ignored with warning.
- Product sidecar **`rex-agent`** (LangGraph ReAct); harness default **`rex-sidecar-stub`** for CI and `rex config init`.
- Access policy broker (RC-05); routing logs with **`route=`** and **`decision_id=`** (RC-09).
- Web desktop operator surface (**W100–W118**); terminal TUI and in-repo VS Code extension removed.

### Changed

- Streaming/agent Must **RC-01…RC-10** Met per [V1_0.md](docs/V1_0.md); observability Must **RC-S3–RC-S5** remain open (v1.0 tag blocked).

## [0.1.0] - 2026-05-21

### Added

- Initial workspace: `rex-daemon`, `rex-cli`, `rex-proto`, `rex-sidecar-stub`.
- gRPC over UDS, NDJSON streaming contract for editors, brokered HTTP inference path, and PR CI gates.
