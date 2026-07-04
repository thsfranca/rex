# REX

**This repository is an experimental study project:** it exists to explore patterns (daemon-hosted inference, contracts, tooling, tests)—not as a finished product or supported distribution.

REX is a **local AI runtime** for macOS (Apple Silicon): a Rust **daemon** owns inference and stream lifecycle, while **thin clients** (CLI, scripts) talk to it over **gRPC on a Unix domain socket**. The repo is also a **learning lab** for AI infrastructure patterns and a **small, testable reference** for how to keep automation surfaces stable while the engine evolves.

## Purpose

Canonical **purpose and operating principles** (single source of truth): **[docs/PURPOSE_AND_PRINCIPLES.md](docs/PURPOSE_AND_PRINCIPLES.md)**.

**Configuration policy** (JSON only; sole product env var **`REX_ROOT`**): [docs/CONFIGURATION.md](docs/CONFIGURATION.md).

## What works today (high-value capabilities)

| Capability | What it gives you |
|---|---|
| **gRPC + UDS** | Low-latency local transport; socket path from `$REX_ROOT/config.json` (`daemon.socket`). |
| **Streaming completion** | `StreamInference`-style chunks with deterministic lifecycle logging on the daemon for triage. |
| **`rex` CLI** | Unified binary: `rex daemon`, `rex status`, `rex complete`; **`--format ndjson`** for one JSON event per line (`chunk`, `done`, `error`) so scripts and CI can parse streams safely. Legacy `rex-cli` / `rex-daemon` shims remain for compatibility. |
| **Startup and failure behavior** | Bounded retry when the daemon is still booting; clear CLI errors for unavailable daemon, interrupted streams, and bad stream endings. |
| **Development agent (CLI)** | **`ask` / `plan` / `agent`** modes via **`rex complete --format ndjson`** — assistant runtime in a **daemon-supervised sidecar** ([docs/NDJSON_STREAM.md](docs/NDJSON_STREAM.md), [docs/MVP_SPEC.md](docs/MVP_SPEC.md)). |
| **Brokered HTTP inference** | Daemon invokes **OpenAI-compatible** backend on behalf of the sidecar (`inference.openai_compat` in JSON); **mock** / **cursor-cli** for tests — [docs/ADAPTERS.md](docs/ADAPTERS.md), [docs/CONFIGURATION.md](docs/CONFIGURATION.md). |
| **Quality gates** | Workspace `cargo` checks plus UDS end-to-end tests covering failure paths, startup races, and post-interruption behavior (see [Operational checks](#operational-checks)). |

## Project status

- **Experimental scope:** APIs, docs, and behavior can change as the study evolves; use this workspace for learning and prototypes, not production SLAs.
- **Local operator path:** **clone → configure HTTP backend → daemon (sidecar) → CLI NDJSON stream** — [`docs/CLI_OPERATOR_UX.md`](docs/CLI_OPERATOR_UX.md), `./scripts/verify_mvp_local.sh` ([`docs/CI.md`](docs/CI.md)). Product shape: [`docs/MVP_SPEC.md`](docs/MVP_SPEC.md).
- **Done / v1.0:** [`docs/V1_0.md`](docs/V1_0.md) (**RC-*** release criteria, **`1.0.0`** tag gate) — **not Met** until observability **RC-LF1** (LangFuse Cloud export) closes; [`docs/ROADMAP.md`](docs/ROADMAP.md) tracks open gaps; [`docs/LANGFUSE_INTEGRATION.md`](docs/LANGFUSE_INTEGRATION.md) is the observability hub.
- **Product agent (partial — shipped):** [`docs/AGENT_DELIVERY_ROADMAP.md`](docs/AGENT_DELIVERY_ROADMAP.md) — **`rex-agent`** (**R017–R019** Done); daemon prerequisites **R020–R022** Done; JSON config (**R015**) and unified **`rex`** CLI (**R014**) shipped. **`rex config init`** writes the **rex-agent** operator template; **`rex-sidecar-stub`** remains the CI harness default.
- Engineering focus: **stream reliability** plus **stable NDJSON** across CLI and automation.
- Product-learning focus: daemon **economics** (routing, compaction, caches, metrics per [docs/CONTEXT_EFFICIENCY.md](docs/CONTEXT_EFFICIENCY.md)); implementation incremental.
- Not primary scope yet: production-grade local runtime adapters beyond the Phase 1 shape (for example **MLX**), remote networking/TLS, production auth, multi-plugin sidecar fleets. See [`docs/MVP_SPEC.md`](docs/MVP_SPEC.md) (scope) and [`docs/V1_0.md`](docs/V1_0.md) (done).

## Local operator path

Linear recipe from a clone to a working **rex complete** stream (requires **HTTP backend** for brokered inference and a **sidecar agent** per [docs/MVP_SPEC.md](docs/MVP_SPEC.md)):

1. **Build** the Rust workspace: `cargo build --workspace`.
2. **Install** `rex` on PATH — [`scripts/install-cli.sh`](scripts/install-cli.sh).
3. **Configure** brokered HTTP in `$REX_ROOT/config.json` — run `rex config init` (rex-agent + mock web search by default), then edit `inference.openai_compat` — [docs/CONFIGURATION.md](docs/CONFIGURATION.md), [docs/SIDECAR_RUNTIME.md](docs/SIDECAR_RUNTIME.md).
4. **Run** `rex` — opens the terminal UI and ensures a detached daemon (**R071**). Socket defaults to `/tmp/rex.sock` unless overridden in JSON — [`docs/CLI_OPERATOR_UX.md`](docs/CLI_OPERATOR_UX.md).
5. **Verify** sidecar health in daemon logs and brokered `fs.read` via a prompt containing `__rex_read:<path>`.

Details: [docs/CLI_OPERATOR_UX.md](docs/CLI_OPERATOR_UX.md), [docs/MVP_SPEC.md](docs/MVP_SPEC.md).

## Why this shape

REX keeps clients thin and centralizes model/runtime policy in one daemon boundary.

| Component | Role |
|---|---|
| Daemon | Own inference orchestration, stream lifecycle, and future scheduling and system policy. |
| Clients | Own UX only (terminal, scripts) and speak one protocol. |
| Protocol | gRPC over UDS for typed, local, low-latency calls. |

## Quickstart

1) Build the workspace:

```bash
cargo build --workspace
```

This compiles the full workspace (`rex-proto`, `rex-config`, `rex`, `rex-daemon`, `rex-cli`, `rex-sidecar-stub`, and related crates).

2) Start the daemon:

```bash
cargo run -p rex -- daemon
```

This starts the local gRPC server on `/tmp/rex.sock`.

3) In another terminal, run CLI commands:

```bash
cargo run -p rex -- status
cargo run -p rex -- complete "hello from rex"
cargo run -p rex -- complete "hello from rex" --format ndjson --mode ask
```

This verifies status, server-streaming behavior, and NDJSON output for scripts and CI.

## Operational checks

Use these checks when touching daemon/CLI lifecycle behavior:

```bash
./scripts/ci/test_enforce_ci_gate.sh
cargo test -p rex-daemon --test uds_e2e -- --nocapture
```

Coverage includes:
- daemon unavailable connection failure path;
- deterministic startup race recovery path (`unavailable -> ready`);
- stream terminal behavior after daemon interruption.

## Install as terminal commands

Install or reinstall local binaries (from source):

```bash
chmod +x scripts/install-cli.sh
./scripts/install-cli.sh
```

The script reinstalls with `--force`, adds `~/.cargo/bin` to your shell profile by default (use `--skip-shell-path` to opt out), and runs `rex config init` when the layout is missing.

Prebuilt binaries for `rex`, `rex-sidecar-stub`, and compatibility shims (`rex-cli`, `rex-daemon`) are attached to GitHub Releases tagged `vX.Y.Z` after merging the release-plz Release PR. See [`docs/RELEASE.md`](docs/RELEASE.md).

## Manual troubleshooting

- Run `rex daemon` before invoking `rex status` or `rex complete` (unless CLI auto-start is enabled — [`docs/CLI_OPERATOR_UX.md`](docs/CLI_OPERATOR_UX.md)).
- If the daemon is still booting, rerun `status` or `complete` after `/tmp/rex.sock` exists.
- `rex complete` surfaces deterministic lifecycle errors for:
  - daemon unavailable;
  - interrupted streams;
  - incomplete stream termination.
- `rex complete --format ndjson` emits one JSON event per line with terminal `done` or `error`.
- The daemon emits request-scoped stream lifecycle logs (`starting`, `streaming`, `completed`, `interrupted`, `failed`) plus `request_id` and first-chunk markers for troubleshooting.

## Phase 1 boundaries

In scope for the first product shape:

- Local daemon–client communication over UDS.
- Unary status RPC and server-streaming completion RPC.
- **HTTP OpenAI-compat** inference and shutdown lifecycle reliability.
- **CLI path:** **`rex` NDJSON**; modes and approvals — [docs/NDJSON_STREAM.md](docs/NDJSON_STREAM.md).
- **Dogfooding loop:** develop `rex` from the terminal against a live HTTP backend.
- **Test harness:** `inference.runtime: "mock"` in test `config.json` for CI only — [docs/MVP_SPEC.md](docs/MVP_SPEC.md).

Out of scope for Phase 1 (see [`docs/MVP_SPEC.md`](docs/MVP_SPEC.md)):

- Apple MLX runtime integration.
- Multi-active sidecar broadcast (**R016** deferred — [ADR 0017](docs/architecture/decisions/0017-single-active-sidecar-phase-1.md)). Phase 1 uses a **supervised sidecar** + daemon broker ([`docs/MVP_SPEC.md`](docs/MVP_SPEC.md), [`docs/SIDECAR_RUNTIME.md`](docs/SIDECAR_RUNTIME.md)).
- Remote networking, TLS, and production authentication.

## Documentation map

| Document | Purpose |
|---|---|
| [`docs/README.md`](docs/README.md) | Documentation index and reading order. |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | SAD-style architecture (C4 views, quality attributes, observability/security) + links to ADRs. |
| [`docs/architecture/decisions/`](docs/architecture/decisions/) | ADRs (daemon/agent boundary, adapters, cache policy, routing vs gateway). |
| [`docs/MVP_SPEC.md`](docs/MVP_SPEC.md) | Phase 1 product architecture and scope (done: [`docs/V1_0.md`](docs/V1_0.md) only). |
| [`docs/V1_0.md`](docs/V1_0.md) | v1.0 release criteria (**RC-***), SemVer `1.0.0` meaning, tagging gate. |
| [`docs/ROADMAP.md`](docs/ROADMAP.md) | Post-v1.0 queue (**R023+**); agent graph **R027–R033**. |
| [`docs/NDJSON_STREAM.md`](docs/NDJSON_STREAM.md) | NDJSON consumer contract for CLI and automation. |
| [`docs/CLI_OPERATOR_UX.md`](docs/CLI_OPERATOR_UX.md) | Terminal operator path and daemon lifecycle UX. |
| [`docs/RELEASE.md`](docs/RELEASE.md) | Core release automation (Release PRs, tags, binaries). |
| [`docs/PLUGIN_ROADMAP.md`](docs/PLUGIN_ROADMAP.md) | Sidecar agent platform; brokered inference adapters. |
| [`docs/ADAPTERS.md`](docs/ADAPTERS.md) | Inference adapter contract, capabilities, and Cursor CLI profile. |
| [`docs/CACHING.md`](docs/CACHING.md) | Layered response cache design: keys, mode safety, bypass. |
| [`docs/CONTEXT_EFFICIENCY.md`](docs/CONTEXT_EFFICIENCY.md) | Context pipeline + **economics lever matrix** (routing, caches, compaction, MCP, approvals, LTM linkage). |
| [`docs/DEPENDENCIES.md`](docs/DEPENDENCIES.md) | Local build/runtime prerequisites by layer. |
| [`docs/CI.md`](docs/CI.md) | CI strategy, gate contracts, and merge protections. |
| [`docs/DOCUMENTATION.md`](docs/DOCUMENTATION.md) | Documentation checklist and writing standards. |
| [`docs/DEVELOPMENT_ASSISTANCE_CAPABILITIES.md`](docs/DEVELOPMENT_ASSISTANCE_CAPABILITIES.md) | Daemon-owned context, turn contract, budget pipeline (ADRs 0011–0017). |
| [`docs/LONG_TERM_MEMORY.md`](docs/LONG_TERM_MEMORY.md) | Long-term memory design hub (**bets**, optimization-first; ADR 0014). |

See [`docs/README.md`](docs/README.md) for **CONFIGURATION**, **SIDECAR_RUNTIME**, **AGENT_DELIVERY**, **ERROR_HANDLING**, and **DEVELOPER_EXPERIENCE_GUIDE**.

## Workspace layout

- `rex-proto`: protobuf/gRPC contract generation (`rex.v1`).
- `rex-config`: JSON config load/merge (`$REX_ROOT/config.json`).
- `rex`: unified CLI (`daemon`, `status`, `complete`; config/proto/sidecar helpers).
- `rex-daemon`: daemon library/runtime (compat shim binary `rex-daemon`).
- `rex-cli`: CLI client library (compat shim binary `rex-cli`).
- `rex-sidecar-stub`: harness sidecar for CI and default `rex config init`.
- `sidecars/rex-agent`: product LangGraph ReAct sidecar.

## Contributing and validation baseline

- Keep changes focused and align with repository docs.
- Treat generated/runtime outputs as disposable and keep commits source-first:
  - do not commit `target/`, temporary test outputs, or scratch/handoff files.
- Run CI-aligned local checks before PRs:

```bash
./scripts/ci/run_rust_verify.sh
```

Includes `cargo fmt --check`, clippy, **`cargo audit`**, and workspace tests. When touching sidecars, also run the sidecar verify gate in [`docs/CI.md`](docs/CI.md). For reliability-specific work, follow the full sequence there.

Branch protection should require **`ci-checks`** and **`Conventional PR title`**. Do not require `rust-verify` (it skips on docs-only PRs).

## License

REX is licensed under MIT (`workspace.package.license` in `Cargo.toml`).
