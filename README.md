# REX

**This repository is an experimental study project:** it exists to explore patterns (daemon-hosted inference, contracts, tooling, tests)—not as a finished product or supported distribution.

REX is a **local AI runtime** for macOS (Apple Silicon): a Rust **daemon** owns inference and stream lifecycle, while the **desktop app** talks to it over **gRPC on a Unix domain socket**. The repo is also a **learning lab** for AI infrastructure patterns and a **small, testable reference** for how to keep client surfaces stable while the engine evolves.

## Purpose

Canonical **purpose and operating principles** (single source of truth): **[docs/PURPOSE_AND_PRINCIPLES.md](docs/PURPOSE_AND_PRINCIPLES.md)**.

**Configuration policy** (JSON only; sole product env var **`REX_ROOT`**): [docs/CONFIGURATION.md](docs/CONFIGURATION.md).

## What works today (high-value capabilities)

| Capability | What it gives you |
|---|---|
| **gRPC + UDS** | Low-latency local transport; socket path from `$REX_ROOT/config.json` (`daemon.socket`). |
| **Streaming completion** | `StreamInference` chunks with deterministic lifecycle logging on the daemon for triage. |
| **Desktop operator app** | Bare **`rex`** opens the Tauri + React workspace; daemon auto-starts on launch ([docs/OPERATOR_UX.md](docs/OPERATOR_UX.md)). |
| **Setup and doctor commands** | `rex config`, `rex proto`, `rex sidecar`, `rex gateway`, `rex omlx` for layout and health checks. |
| **Development agent** | **`ask` / `plan` / `agent`** modes in the desktop UI — assistant runtime in a **daemon-supervised sidecar** ([docs/MVP_SPEC.md](docs/MVP_SPEC.md), [docs/SIDECAR_RUNTIME.md](docs/SIDECAR_RUNTIME.md)). |
| **Brokered HTTP inference** | Daemon invokes **OpenAI-compatible** backend on behalf of the sidecar (`inference.openai_compat` in JSON); **mock** / **cursor-cli** for tests — [docs/ADAPTERS.md](docs/ADAPTERS.md), [docs/CONFIGURATION.md](docs/CONFIGURATION.md). |
| **Quality gates** | Workspace `cargo` checks, UDS end-to-end tests, and **rex-ui-harness** UI verification (see [Operational checks](#operational-checks)). |

## Project status

- **Experimental scope:** APIs, docs, and behavior can change as the study evolves; use this workspace for learning and prototypes, not production SLAs.
- **Local operator path:** **clone → configure HTTP backend → `rex config init` → build web UI → `rex` (desktop)** — [docs/OPERATOR_UX.md](docs/OPERATOR_UX.md), `./scripts/verify_mvp_local.sh` ([docs/CI.md](docs/CI.md)). Product shape: [docs/MVP_SPEC.md](docs/MVP_SPEC.md).
- **Done / v1.0:** [docs/V1_0.md](docs/V1_0.md) (**RC-*** release criteria, **`1.0.0`** tag gate) — **not Met** until observability **RC-LF1** (LangFuse Cloud export) closes; [docs/ROADMAP.md](docs/ROADMAP.md) tracks open gaps; [docs/LANGFUSE_INTEGRATION.md](docs/LANGFUSE_INTEGRATION.md) is the observability hub.
- **Product agent (partial — shipped):** [docs/AGENT_DELIVERY_ROADMAP.md](docs/AGENT_DELIVERY_ROADMAP.md) — **`rex-agent`** (**R017–R019** Done); daemon prerequisites **R020–R022** Done; JSON config (**R015**) and unified **`rex`** entry (**R014**) shipped. **`rex config init`** writes the **rex-agent** operator template; **`rex-sidecar-stub`** remains the CI harness default.
- Engineering focus: **stream reliability** and **desktop presentation** ([docs/WEB_UI_ROADMAP.md](docs/WEB_UI_ROADMAP.md)).
- Product-learning focus: daemon **economics** (routing, compaction, caches, metrics per [docs/CONTEXT_EFFICIENCY.md](docs/CONTEXT_EFFICIENCY.md)); implementation incremental.
- Not primary scope yet: production-grade local runtime adapters beyond the Phase 1 shape (for example **MLX**), remote networking/TLS, production auth, multi-plugin sidecar fleets. See [docs/MVP_SPEC.md](docs/MVP_SPEC.md) (scope) and [docs/V1_0.md](docs/V1_0.md) (done).

## Local operator path

Linear recipe from a clone to a working desktop session (requires **HTTP backend** for brokered inference and a **sidecar agent** per [docs/MVP_SPEC.md](docs/MVP_SPEC.md)):

1. **Build** the Rust workspace: `cargo build --workspace`.
2. **Install** `rex` on PATH — [scripts/install-cli.sh](scripts/install-cli.sh).
3. **Configure** brokered HTTP in `$REX_ROOT/config.json` — run `rex config init` (rex-agent + mock web search by default), then edit `inference.openai_compat` — [docs/CONFIGURATION.md](docs/CONFIGURATION.md), [docs/SIDECAR_RUNTIME.md](docs/SIDECAR_RUNTIME.md).
4. **Build** the web UI bundle: `cd apps/rex-web && npm ci && npm run build`.
5. **Run** `rex` — opens the **desktop web UI** (Tauri) and ensures a detached daemon. Socket defaults to `/tmp/rex.sock` unless overridden in JSON — [docs/OPERATOR_UX.md](docs/OPERATOR_UX.md).
6. **Verify** in the app: submit a prompt; confirm streaming text and sidecar health in daemon logs.

Details: [docs/OPERATOR_UX.md](docs/OPERATOR_UX.md), [docs/MVP_SPEC.md](docs/MVP_SPEC.md).

## Why this shape

REX keeps the presentation client thin and centralizes model/runtime policy in one daemon boundary.

| Component | Role |
|---|---|
| Daemon | Own inference orchestration, stream lifecycle, and future scheduling and system policy. |
| Desktop app | Own operator UX; consumes `StreamInference` over UDS via Tauri. |
| Protocol | gRPC over UDS for typed, local, low-latency calls. |

## Quickstart

1) Build the workspace:

```bash
cargo build --workspace
```

This compiles the full workspace (`rex-proto`, `rex-config`, `rex`, `rex-daemon`, `rex-desktop`, `rex-stream-ui`, `rex-sidecar-stub`, and related crates).

2) Initialize config and build the web UI:

```bash
cargo run -p rex -- config init
cd apps/rex-web && npm ci && npm run build && cd -
```

3) Launch the desktop app (macOS):

```bash
cargo run -p rex --
```

Bare `rex` opens the desktop workspace and auto-starts the daemon when needed. Setup subcommands:

```bash
rex config <init|show|path|validate>
rex proto <install|path|doctor>
rex sidecar <list|init|doctor>
rex gateway <init|doctor>
rex omlx <init|doctor>
```

Session flags: `rex --continue`, `rex --last`, `rex --debug`.

## Operational checks

Use these checks when touching daemon or desktop lifecycle behavior:

```bash
./scripts/ci/test_enforce_ci_gate.sh
cargo test -p rex-daemon --test uds_e2e -- --nocapture
./scripts/verify_mvp_local.sh
```

Coverage includes:
- daemon unavailable connection failure path;
- deterministic startup race recovery path (`unavailable -> ready`);
- stream terminal behavior after daemon interruption;
- MVP product-path smoke (sidecar + brokered HTTP fixture).

UI verification: [docs/WEB_UI_AGENT_VALIDATION.md](docs/WEB_UI_AGENT_VALIDATION.md), `./scripts/setup_ui_probe_env.sh`.

## Install as terminal commands

Install or reinstall local binaries (from source):

```bash
chmod +x scripts/install-cli.sh
./scripts/install-cli.sh
```

The script reinstalls with `--force`, adds `~/.cargo/bin` to your shell profile by default (use `--skip-shell-path` to opt out), and runs `rex config init` when the layout is missing.

Prebuilt binaries for `rex`, `rex-sidecar-stub`, and the legacy `rex-daemon` shim are attached to GitHub Releases tagged `vX.Y.Z` after merging the release-plz Release PR. See [docs/RELEASE.md](docs/RELEASE.md).

## Manual troubleshooting

- Bare **`rex`** auto-starts the daemon; no separate foreground daemon session is required — [docs/OPERATOR_UX.md](docs/OPERATOR_UX.md).
- If the app shows unavailable status, wait for `/tmp/rex.sock` (or your configured socket) and retry; check daemon logs under `$REX_ROOT`.
- Build the web bundle before launching: `cd apps/rex-web && npm run build`.
- The daemon emits request-scoped stream lifecycle logs (`starting`, `streaming`, `completed`, `interrupted`, `failed`) plus `request_id` and first-chunk markers for troubleshooting.

## Phase 1 boundaries

In scope for the first product shape:

- Local daemon–client communication over UDS.
- Unary status RPC and server-streaming completion RPC.
- **HTTP OpenAI-compat** inference and shutdown lifecycle reliability.
- **Desktop operator path** — modes and approvals in the web UI — [docs/OPERATOR_UX.md](docs/OPERATOR_UX.md), [docs/WEB_UI_DESIGN.md](docs/WEB_UI_DESIGN.md).
- **Dogfooding loop:** develop Rex from the desktop app against a live HTTP backend.
- **Test harness:** `inference.runtime: "mock"` in test `config.json` for CI only — [docs/MVP_SPEC.md](docs/MVP_SPEC.md).

Out of scope for Phase 1 (see [docs/MVP_SPEC.md](docs/MVP_SPEC.md)):

- Apple MLX runtime integration.
- Multi-active sidecar broadcast (**R016** deferred — [ADR 0017](docs/architecture/decisions/0017-single-active-sidecar-phase-1.md)). Phase 1 uses a **supervised sidecar** + daemon broker ([docs/MVP_SPEC.md](docs/MVP_SPEC.md), [docs/SIDECAR_RUNTIME.md](docs/SIDECAR_RUNTIME.md)).
- Remote networking, TLS, and production authentication.

## Documentation map

| Document | Purpose |
|---|---|
| [docs/README.md](docs/README.md) | Documentation index and reading order. |
| [docs/OPERATOR_UX.md](docs/OPERATOR_UX.md) | Desktop operator path and session flags. |
| [docs/WEB_UI_DESIGN.md](docs/WEB_UI_DESIGN.md) | Web desktop design system and acceptance gate. |
| [docs/WEB_UI_ARCHITECTURE.md](docs/WEB_UI_ARCHITECTURE.md) | Tauri + React architecture. |
| [docs/WEB_UI_ROADMAP.md](docs/WEB_UI_ROADMAP.md) | Web UI phased delivery. |
| [docs/WEB_UI_AGENT_VALIDATION.md](docs/WEB_UI_AGENT_VALIDATION.md) | rex-ui-harness MCP validation. |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | SAD-style architecture (C4 views, quality attributes) + links to ADRs. |
| [docs/architecture/decisions/](docs/architecture/decisions/) | ADRs (daemon/agent boundary, adapters, cache policy, routing vs gateway). |
| [docs/MVP_SPEC.md](docs/MVP_SPEC.md) | Phase 1 product architecture and scope (done: [docs/V1_0.md](docs/V1_0.md) only). |
| [docs/V1_0.md](docs/V1_0.md) | v1.0 release criteria (**RC-***), SemVer `1.0.0` meaning, tagging gate. |
| [docs/ROADMAP.md](docs/ROADMAP.md) | v1.0 closure queue and engineering backlog. |
| [docs/RELEASE.md](docs/RELEASE.md) | Core release automation (Release PRs, tags, binaries). |
| [docs/PLUGIN_ROADMAP.md](docs/PLUGIN_ROADMAP.md) | Sidecar agent platform; brokered inference adapters. |
| [docs/ADAPTERS.md](docs/ADAPTERS.md) | Inference adapter contract, capabilities, and Cursor CLI profile. |
| [docs/CACHING.md](docs/CACHING.md) | Layered response cache design: keys, mode safety, bypass. |
| [docs/CONTEXT_EFFICIENCY.md](docs/CONTEXT_EFFICIENCY.md) | Context pipeline + **economics lever matrix**. |
| [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md) | Local build/runtime prerequisites by layer. |
| [docs/CI.md](docs/CI.md) | CI strategy, gate contracts, and merge protections. |
| [docs/DOCUMENTATION.md](docs/DOCUMENTATION.md) | Documentation checklist and writing standards. |
| [docs/DEVELOPMENT_ASSISTANCE_CAPABILITIES.md](docs/DEVELOPMENT_ASSISTANCE_CAPABILITIES.md) | Daemon-owned context, turn contract, budget pipeline (ADRs 0011–0017). |
| [docs/LONG_TERM_MEMORY.md](docs/LONG_TERM_MEMORY.md) | Long-term memory design hub (**bets**, optimization-first; ADR 0014). |

See [docs/README.md](docs/README.md) for **CONFIGURATION**, **SIDECAR_RUNTIME**, **AGENT_DELIVERY**, **ERROR_HANDLING**, and **DEVELOPER_EXPERIENCE_GUIDE**.

## Workspace layout

- `crates/rex-proto`: protobuf/gRPC contract generation (`rex.v1`).
- `crates/rex-config`: JSON config load/merge (`$REX_ROOT/config.json`).
- `crates/rex`: unified entry (`rex` opens desktop; config/proto/sidecar helpers).
- `crates/rex-daemon`: daemon library/runtime (legacy shim binary `rex-daemon`).
- `crates/rex-desktop`: Tauri backend (UDS proxy, daemon lifecycle, menu bar).
- `crates/rex-stream-ui`: stream event projection for the desktop shell.
- `crates/rex-capability-mock`: mock capability sidecar crate.
- `crates/rex-sidecar-stub`: harness sidecar for CI.
- `crates/rex-ui-harness`: MCP + Playwright UI validation.
- `apps/rex-web`: React presentation client.
- `sidecars/rex-agent`: product LangGraph ReAct sidecar.

## Contributing and validation baseline

- Keep changes focused and align with repository docs.
- Treat generated/runtime outputs as disposable and keep commits source-first:
  - do not commit `target/`, temporary test outputs, or scratch/handoff files.
- Run CI-aligned local checks before PRs:

```bash
./scripts/ci/run_rust_verify.sh
```

Includes `cargo fmt --check`, clippy, **`cargo audit`**, and workspace tests. When touching sidecars, also run the sidecar verify gate in [docs/CI.md](docs/CI.md). For UI work, see [docs/WEB_UI_AGENT_VALIDATION.md](docs/WEB_UI_AGENT_VALIDATION.md).

Branch protection should require **`ci-checks`** and **`Conventional PR title`**. Do not require `rust-verify` (it skips on docs-only PRs).

## License

REX is licensed under MIT (`workspace.package.license` in `Cargo.toml`).
