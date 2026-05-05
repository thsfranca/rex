# Developer Experience Guide For REX

This guide defines how to develop REX with AI assistance and how to design plugin-facing changes without breaking contracts.

## 1) Project purpose and architecture in 2 minutes

Canonical **purpose and principles**: [PURPOSE_AND_PRINCIPLES.md](PURPOSE_AND_PRINCIPLES.md).

REX provides a local AI runtime with one daemon as the **system authority** for **streaming contracts, adapter policy, caches, pipelines, and the agent/economics roadmap** ([ADR 0001](architecture/decisions/0001-daemon-owns-agent-orchestration-and-economics.md)). Isolated **agent runtime environments** (when implemented) remain **supervised and policy-bound** to the daemon—see [ADR 0005](architecture/decisions/0005-rex-owns-sidecar-environment-not-agent-implementations.md). **Sidecar ↔ daemon** integration uses a **dedicated brokered API**, not **`rex.v1`** — [ADR 0008](architecture/decisions/0008-dedicated-sidecar-control-plane-api.md).

The editor extension keeps **`rex-cli` NDJSON** as the **primary** streaming path; optional unary **`rex.v1`** over UDS is allowed per **[ADR 0007](architecture/decisions/0007-editor-extension-hybrid-transport-cli-and-grpc.md)**.

| Component | Responsibility |
|---|---|
| `rex-daemon` | Model/agent **policy trajectory**, adapters, caches, **`StreamInference`** lifecycle, queues. |
| `rex-cli` | Thin transport façade; deterministic NDJSON for editors. |
| `rex-proto` | `rex.v1` gRPC contract. |

Architecture intent:

- Stable protocol boundary across clients.
- Default local UDS transport.
- Product logic migrates toward daemon-first economics ([CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md)).

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
- Follow [ADR 0006](architecture/decisions/0006-gitignored-temp-tracking-sequential-handoffs.md) for repo-touching plans: keep a gitignored root `TEMP_*.md` record and, for sequential tasks, include **handoff instructions** (**purpose of the whole slice**, preferred response formats, and structure) for the next session.

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

### Documentation conventions

Major product features use **one canonical hub doc** under `docs/` plus **pointers** in roadmaps and overviews—not parallel full design lists across files. Rules and hubs live in [DOCUMENTATION.md](DOCUMENTATION.md).

CI and PR expectations:
- CI gate behavior follows `docs/CI.md`.
- PR description follows `.github/pull_request_template.md` and stays about this repository and the change (no required coupling to external trackers for now).

## 4) Extensibility developer track

**Stable today**

- [`proto/rex/v1/rex.proto`](../proto/rex/v1/rex.proto) unary + streaming RPC semantics.
- `InferenceRuntime` + capability negotiation ([ADAPTERS.md](ADAPTERS.md)).
- Daemon-owned L1/cache hooks ([CACHING.md](CACHING.md)).

**Near roadmap**

- Optional **single** supervised sidecar aligning with **[PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md)** (failure isolation—not the default economics path).

**Later**

- Rich multi-plugin fleets, Wasm bridges—only after sidecar MVP evidence.

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

## 8) Roadmap to extensibility-ready DX

**Daemon-first prerequisites (higher leverage now)**

1. Router / metering fields landing per [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) matrix rows.
2. Deterministic **`InferenceRuntime`** tests + NDJSON conformance already in-tree.

**Optional sidecar-era artifacts (defer until isolation need is proven)**

- Plugin manifest (`runtime`, `entrypoint`, `capabilities`, contract semver).
- Health + restart policy + conformance harness (`PLUGIN_ROADMAP.md` snapshot).

Suggested reading for maintainers pitching integrations:
- [VS Code Proposed API model](https://code.visualstudio.com/api/advanced-topics/using-proposed-api)
- [Open Source Guides – Best Practices for maintainers](https://opensource.guide/best-practices/)
- [GitHub contributing guideline discoverability](https://docs.github.com/en/communities/setting-up-your-project-for-healthy-contributions/setting-guidelines-for-repository-contributors)
