# Developer Experience Guide For REX

This guide defines how to develop REX with AI assistance and how to design plugin-facing changes without breaking contracts.

## 1) Project purpose and architecture in 2 minutes

REX provides a local AI runtime with one daemon as the system authority.

| Component | Responsibility |
|---|---|
| `rex-daemon` | Own model lifecycle, scheduling, policy, and stream production. |
| `rex-cli` | Stay thin; issue commands and render responses. |
| `rex-proto` | Define and generate the shared gRPC contract (`rex.v1`). |

Architecture intent:
- Keep one stable protocol boundary for all clients.
- Keep communication local by default (`/tmp/rex.sock` over UDS).
- Evolve functionality behind stable contracts instead of client-specific logic.

## 2) Setup and working modes

### Local setup

Prerequisites:
- Rust toolchain
- `protoc` (see `docs/DEPENDENCIES.md`)

Core commands:

```bash
cargo build --workspace
cargo run -p rex-daemon
cargo run -p rex-cli -- status
cargo run -p rex-cli -- complete "hello from rex"
```

### Working modes

Use lightweight modes instead of rigid process:

| Mode | Primary goal | Exit criterion |
|---|---|---|
| Exploration | Understand constraints and boundaries | You can point to exact files/contracts to change |
| Implementation | Make one focused behavior change | Build and tests pass for changed scope |
| Verification | Confirm behavior and regressions | Local checks and manual validation pass |

### Human and AI responsibilities

Human responsibilities:
- Set scope and accept/reject trade-offs.
- Approve contract changes and release-impacting decisions.
- Confirm final validation before merge.

AI responsibilities:
- Draft focused code/doc updates.
- Keep changes aligned with existing contracts and rules.
- Produce explicit verification steps.

### Common failures and recovery

| Symptom | Likely cause | Recovery |
|---|---|---|
| CLI cannot connect | Daemon not running yet | Start daemon, retry command |
| Bind fails on `/tmp/rex.sock` | Stale socket file | Stop daemon cleanly; remove stale socket and restart |
| Protobuf build error | `protoc` missing or incompatible | Reinstall prerequisites from `docs/DEPENDENCIES.md` |
| Installed command not found | PATH not updated after install | Re-run install script and reload shell |

## 3) Development workflow and quality gates

Required local checks before PR:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```

Use this review checklist before accepting AI-generated changes:
- Scope: one primary concern per PR.
- Contract safety: no accidental `rex.v1` breaking changes.
- Reliability: failure paths are explicit and testable.
- Documentation: setup/behavior changes are documented.
- Verification: command-level or test-level proof is present.

CI and PR expectations:
- CI gate behavior follows `docs/CI.md`.
- PR description follows `.github/pull_request_template.md`.

## 4) Plugin developer track (current and next phase)

Current status:
- Plugin lifecycle is not implemented in MVP.
- Direction is runtime-managed gRPC sidecars.

What plugin developers should treat as stable today:
- gRPC/Protobuf contract patterns in `proto/rex/v1/rex.proto`.
- Streaming completion expectations (`done = true` terminal behavior).
- Local-runtime assumptions (daemon-centered orchestration).

What remains post-MVP work:
- Plugin manifest/config schema
- Health handshake contract
- Startup/restart/shutdown policy details
- Conformance test harness for plugin implementations

## 5) Contract and compatibility policy

Public API boundary for this project:
- `proto/rex/v1/rex.proto` and generated `rex.v1` surfaces

Allowed by default (same major version):
- Additive fields/RPCs with backward-compatible defaults.
- New optional behavior that does not break old clients.

Disallowed without major version change:
- Removing or renaming existing fields/RPCs.
- Reusing removed protobuf field numbers.
- Changing behavior in ways that break reasonable existing clients.

Deprecation policy:
- Mark APIs as deprecated before removal.
- Keep at least one transition period with clear migration direction.

References:
- [Semantic Versioning 2.0.0](https://semver.org/)
- [Google AIP-180 Backwards compatibility](https://google.aip.dev/180)
- [Protocol Buffers: Updating a Message Type](https://protobuf.dev/programming-guides/proto3/#updating)

## 6) Testing strategy by risk

| Change type | Minimum tests |
|---|---|
| CLI formatting or text-only output | Targeted unit tests or command verification |
| Daemon behavior/path changes | Unit tests + relevant integration path |
| Transport/stream lifecycle changes | Unit + E2E (`crates/rex-daemon/tests/uds_e2e.rs`) |
| Contract/proto evolution | Regenerate/build + compatibility-focused integration checks |

Testing principles:
- Prefer meaningful behavior checks over line-only coverage.
- Add E2E tests for startup race, daemon unavailable, and stream terminal behavior when affected.

## 7) Security and operational baseline

Security baseline:
- Keep transport local through UDS by default.
- Use filesystem/socket permissions to limit access.
- Avoid remote listeners unless explicitly introduced by spec.

Operational baseline:
- Keep daemon lifecycle deterministic (startup, shutdown, cleanup).
- Keep failure messages actionable for CLI and logs.
- Preserve CI observability conventions from `docs/CI.md`.

## 8) Roadmap to plugin-ready DX

Next artifacts to build:
1. Plugin manifest schema (`runtime`, `version`, `entrypoint`, `capabilities`, contract version).
2. Plugin lifecycle contract (health, readiness timeout, restart policy, shutdown behavior).
3. Plugin conformance test harness (contract and lifecycle verification).

Recommended order:
- Start with manifest schema to lock integration inputs.
- Add lifecycle contract to reduce runtime ambiguity.
- Add conformance tests to keep compatibility enforceable.

Plugin ecosystem reference:
- [VS Code Proposed API model](https://code.visualstudio.com/api/advanced-topics/using-proposed-api)
- [Open Source Guides – Best Practices for maintainers](https://opensource.guide/best-practices/)
- [GitHub contributing guideline discoverability](https://docs.github.com/en/communities/setting-up-your-project-for-healthy-contributions/setting-guidelines-for-repository-contributors)
