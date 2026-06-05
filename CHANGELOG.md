# Changelog

All notable changes to the REX workspace (Rust daemon, CLI, sidecar stub, and `rex.v1` contract) are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions use git tags `vX.Y.Z` on `main`.

## [Unreleased]

## [1.0.0] - (awaiting tag)

Workspace version **`1.0.0`** in tree; **`v1.0.0` git tag** lands when a maintainer merges the release-plz Release PR ([RELEASE.md](docs/RELEASE.md)).

### Added

- Unified **`rex`** CLI (`daemon`, `status`, `complete`); `rex-cli` / `rex-daemon` shims for compatibility.
- JSON-first config (`rex config init`); legacy `REX_*` env vars ignored with warning.
- Product sidecar **`rex-agent`** (LangGraph ReAct); harness default **`rex-sidecar-stub`** for CI and `rex config init`.
- Extension **`rex.productAgentConfig`** default merges `rex-agent` + approvals.
- Access policy broker (RC-05); routing logs with **`route=`** and **`decision_id=`** (RC-09).

### Changed

- All Must **RC-01…RC-10** Met per [V1_0.md](docs/V1_0.md).

## [0.1.0] - 2026-05-21

### Added

- Initial workspace: `rex-daemon`, `rex-cli`, `rex-proto`, `rex-sidecar-stub`.
- gRPC over UDS, NDJSON streaming contract for editors, brokered HTTP inference path, and PR CI gates.
