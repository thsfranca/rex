# REX

**This repository is an experimental study project:** it exists to explore patterns (daemon-hosted inference, contracts, tooling, tests)—not as a finished product or supported distribution.

REX is a **local AI runtime** for macOS (Apple Silicon): a Rust **daemon** owns inference and stream lifecycle, while **thin clients** (CLI, VS Code/Cursor extension, scripts) talk to it over **gRPC on a Unix domain socket**. The repo is also a **learning lab** for AI infrastructure patterns and a **small, testable reference** for how to keep editor and automation surfaces stable while the engine evolves.

## Purpose

Canonical **purpose and operating principles** (single source of truth): **[docs/PURPOSE_AND_PRINCIPLES.md](docs/PURPOSE_AND_PRINCIPLES.md)**.

**Configuration policy** (precedence, `REX_*` catalog, and roadmap for CLI and user config files): [docs/CONFIGURATION.md](docs/CONFIGURATION.md).

## What works today (high-value capabilities)

| Capability | What it gives you |
|---|---|
| **gRPC + UDS** | Low-latency local transport on `/tmp/rex.sock` with generated types from one `rex.v1` contract. |
| **Streaming completion** | `StreamInference`-style chunks with deterministic lifecycle logging on the daemon for triage. |
| **`rex-cli`** | `status` and `complete`; human output by default, **`--format ndjson`** for one JSON event per line (`chunk`, `done`, `error`) so extensions and CI can parse streams safely. |
| **Startup and failure behavior** | Bounded retry when the daemon is still booting; clear CLI errors for unavailable daemon, interrupted streams, and bad stream endings. |
| **VS Code / Cursor extension** | Activity-bar chat with streaming markdown, selection-based commands, optional **daemon auto-start**, and install/release docs—see [`extensions/rex-vscode/README.md`](extensions/rex-vscode/README.md) and [`docs/EXTENSION_RELEASE.md`](docs/EXTENSION_RELEASE.md). |
| **Development agent (extension)** | Chat with **`ask` / `plan` / `agent`**, guarded apply/insert, NDJSON via **`rex-cli`** — assistant runtime in a **daemon-supervised sidecar** ([docs/EXTENSION.md](docs/EXTENSION.md), [docs/MVP_SPEC.md](docs/MVP_SPEC.md)). |
| **Brokered HTTP inference** | Daemon invokes **OpenAI-compatible** backend on behalf of the sidecar (`REX_OPENAI_COMPAT_*`); **mock** / **cursor-cli** for tests only — [docs/ADAPTERS.md](docs/ADAPTERS.md), [docs/CONFIGURATION.md](docs/CONFIGURATION.md). |
| **Quality gates** | Workspace `cargo` checks plus UDS end-to-end tests covering failure paths, startup races, and post-interruption behavior (see [Operational checks](#operational-checks)). |

## Project status

- **Experimental scope:** APIs, docs, and behavior can change as the study evolves; use this workspace for learning and prototypes, not production SLAs.
- **Local operator path:** **clone → configure HTTP backend → daemon (sidecar) → REX chat** — [`docs/EXTENSION_LOCAL_E2E.md`](docs/EXTENSION_LOCAL_E2E.md), `./scripts/verify_mvp_local.sh` ([`docs/CI.md`](docs/CI.md)). Product shape: [`docs/MVP_SPEC.md`](docs/MVP_SPEC.md).
- **Done / v1.0:** [`docs/V1_0.md`](docs/V1_0.md) (**RC-*** release criteria, **`1.0.0`** tag gate); [`docs/ROADMAP.md`](docs/ROADMAP.md) tracks open gaps; [`docs/PRIORITIZATION.md`](docs/PRIORITIZATION.md) buckets work.
- Engineering focus: **stream reliability** plus **stable NDJSON** across CLI/extension.
- Product-learning focus: daemon **economics** (routing, compaction, caches, metrics per [docs/CONTEXT_EFFICIENCY.md](docs/CONTEXT_EFFICIENCY.md)); implementation incremental.
- VS Code/Cursor extension baseline is **shipped** (chat UX, NDJSON streaming integration, opt-in daemon auto-start, and release/install pipeline); ongoing work is incremental hardening and follow-on capabilities.
- Not primary scope yet: production-grade local runtime adapters beyond the Phase 1 shape (for example **MLX**), remote networking/TLS, production auth, multi-plugin sidecar fleets. See [`docs/MVP_SPEC.md`](docs/MVP_SPEC.md) (scope) and [`docs/V1_0.md`](docs/V1_0.md) (done).

## Local operator path

Linear recipe from a clone to **REX** chat in the editor (requires **HTTP backend** for brokered inference and a **sidecar agent** per [docs/MVP_SPEC.md](docs/MVP_SPEC.md)):

1. **Build** the Rust workspace: `cargo build --workspace` — or one shot: `chmod +x ./scripts/dev-rex-extension.sh && ./scripts/dev-rex-extension.sh`.
2. **Put** `rex-cli` (and the daemon if you use auto-start) where the **editor** can find them — [`scripts/install-cli.sh`](scripts/install-cli.sh).
3. **Configure** brokered HTTP (example Ollama): `export REX_OPENAI_COMPAT_BASE_URL=http://127.0.0.1:11434/v1` and `REX_OPENAI_COMPAT_MODEL=…` — [docs/CONFIGURATION.md](docs/CONFIGURATION.md). Enable the supervised sidecar: `export REX_SIDECAR_ENABLED=1` and ensure `rex-sidecar-stub` is on `PATH` (or set `REX_SIDECAR_BINARY`) — [docs/SIDECAR_RUNTIME.md](docs/SIDECAR_RUNTIME.md).
4. **Run** `rex-daemon` with those env vars (supervisor spawns the sidecar when enabled). Socket: `/tmp/rex.sock` (override with `REX_DAEMON_SOCKET` for the stub broker path).
5. **Install** the extension — [docs/EXTENSION_LOCAL_E2E.md](docs/EXTENSION_LOCAL_E2E.md).
6. **In the editor:** **REX: Open Chat**, try **agent** mode, send a prompt, cancel once, and apply a code block with approval. Verify sidecar health in daemon logs and brokered `fs.read` via a prompt containing `__rex_read:<path>`.

Details: [docs/EXTENSION_LOCAL_E2E.md](docs/EXTENSION_LOCAL_E2E.md), [docs/MVP_SPEC.md](docs/MVP_SPEC.md).

## Why this shape

REX keeps clients thin and centralizes model/runtime policy in one daemon boundary.

| Component | Role |
|---|---|
| Daemon | Own inference orchestration, stream lifecycle, and future scheduling and system policy. |
| Clients | Own UX only (terminal, editor, scripts) and speak one protocol. |
| Protocol | gRPC over UDS for typed, local, low-latency calls. |

## Quickstart

1) Build the workspace:

```bash
cargo build --workspace
```

This compiles `rex-proto`, `rex-daemon`, and `rex-cli`.

2) Start the daemon:

```bash
cargo run -p rex-daemon
```

This starts the local gRPC server on `/tmp/rex.sock`.

3) In another terminal, run CLI commands:

```bash
cargo run -p rex-cli -- status
cargo run -p rex-cli -- complete "hello from rex"
cargo run -p rex-cli -- complete "hello from rex" --format ndjson
```

This verifies status, server-streaming behavior, and extension-consumable NDJSON output.

4) (Optional) Install the REX VS Code / Cursor extension:

```bash
chmod +x ./scripts/dev-rex-extension.sh
./scripts/dev-rex-extension.sh
```

This runs `cargo build --workspace`, installs `rex-cli` / `rex-daemon` via [`scripts/install-cli.sh`](scripts/install-cli.sh), then runs [`scripts/install-extension.sh`](scripts/install-extension.sh): it builds `extensions/rex-vscode`, packages `rex-vscode.vsix`, installs it with the `cursor` or `code` CLI (auto-detects the host when you run it from an integrated terminal), and requests **Developer: Reload Window** on the last active window. Pass flags through to the installer, for example `./scripts/dev-rex-extension.sh --verify`. Use `./scripts/install-extension.sh --help` for installer flags (`--verify`, `--editor vscode`, `--no-reload`, and so on). The dev script does **not** start `rex-daemon`; use step 2 above or auto-start as in the E2E doc.

- **Full checklist** (daemon, editor `PATH`, verification): [`docs/EXTENSION_LOCAL_E2E.md`](docs/EXTENSION_LOCAL_E2E.md).
- Extension-only: `chmod +x ./scripts/install-extension.sh && ./scripts/install-extension.sh`.
- Manual path: `cd extensions/rex-vscode && npm install && npm run package`, then **Extensions: Install from VSIX...**
- Release VSIX: download from a `rex-vscode-vX.Y.Z` GitHub Release.
- Set `"rex.daemonAutoStart": true` if you want the extension to spawn `rex-daemon` on activation. See [`docs/EXTENSION_RELEASE.md`](docs/EXTENSION_RELEASE.md) for install, auto-start, and troubleshooting details.

## Operational checks

Use these checks when touching daemon/CLI lifecycle behavior:

```bash
./scripts/ci/test_enforce_rust_gate.sh
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

The script reinstalls with `--force` and does not modify shell dotfiles by default.

Prebuilt binaries for `rex-cli`, `rex-daemon`, and `rex-sidecar-stub` are attached to GitHub Releases tagged `vX.Y.Z` after merging the release-plz Release PR. See [`docs/RELEASE.md`](docs/RELEASE.md).

If you want automatic `zsh` PATH configuration:

```bash
./scripts/install-cli.sh --configure-shell
source ~/.zshrc
```

## Manual troubleshooting

- Start `rex-daemon` before invoking `rex-cli` commands.
- If the daemon is still booting, rerun `status` or `complete` after `/tmp/rex.sock` exists.
- `rex-cli complete` surfaces deterministic lifecycle errors for:
  - daemon unavailable;
  - interrupted streams;
  - incomplete stream termination.
- `rex-cli complete --format ndjson` emits one JSON event per line with terminal `done` or `error`.
- `rex-daemon` emits request-scoped stream lifecycle logs (`starting`, `streaming`, `completed`, `interrupted`, `failed`) plus `request_id` and first-chunk markers for troubleshooting.

## Phase 1 boundaries

In scope for the first product shape:

- Local daemon–client communication over UDS.
- Unary status RPC and server-streaming completion RPC.
- **HTTP OpenAI-compat** inference and shutdown lifecycle reliability.
- **Editor path:** extension + **`rex-cli` NDJSON**; modes and guarded apply — [docs/EXTENSION.md](docs/EXTENSION.md).
- **Dogfooding loop:** develop `rex` from the IDE against a live HTTP backend.
- **Test harness:** `REX_INFERENCE_RUNTIME=mock` for CI only — [docs/MVP_SPEC.md](docs/MVP_SPEC.md).

Out of scope for Phase 1 (see [`docs/MVP_SPEC.md`](docs/MVP_SPEC.md)):

- Apple MLX runtime integration.
- **gRPC** sidecar plugin supervision and multi-plugin orchestration in the daemon (MVP uses **in-process** inference plugins only; see [`docs/PLUGIN_ROADMAP.md`](docs/PLUGIN_ROADMAP.md)).
- Remote networking, TLS, and production authentication.

## Documentation map

| Document | Purpose |
|---|---|
| [`docs/README.md`](docs/README.md) | Documentation index and reading order. |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | SAD-style architecture (C4 views, quality attributes, observability/security) + links to ADRs. |
| [`docs/architecture/decisions/`](docs/architecture/decisions/) | ADRs (daemon/agent boundary, adapters, cache policy, routing vs gateway). |
| [`docs/MVP_SPEC.md`](docs/MVP_SPEC.md) | Phase 1 product architecture and scope (done: [`docs/V1_0.md`](docs/V1_0.md) only). |
| [`docs/V1_0.md`](docs/V1_0.md) | v1.0 release criteria (**RC-***), SemVer `1.0.0` meaning, tagging gate. |
| [`docs/ROADMAP.md`](docs/ROADMAP.md) | Progress toward v1.0; engineering backlog IDs (**R004**–**R012**). |
| [`docs/EXTENSION.md`](docs/EXTENSION.md) | NDJSON consumer contract, extension bootstrap path, component layout (replaces superseded MVP/architecture stubs). |
| [`docs/EXTENSION_ROADMAP.md`](docs/EXTENSION_ROADMAP.md) | Phased roadmap for the VS Code/Cursor extension. |
| [`docs/RELEASE.md`](docs/RELEASE.md) | Core and extension release automation (Release PRs, tags, binaries, VSIX). |
| [`docs/EXTENSION_RELEASE.md`](docs/EXTENSION_RELEASE.md) | Install, daemon auto-start, troubleshooting, and extension release pipeline. |
| [`docs/PLUGIN_ROADMAP.md`](docs/PLUGIN_ROADMAP.md) | Sidecar agent platform; brokered inference adapters. |
| [`docs/ADAPTERS.md`](docs/ADAPTERS.md) | Inference adapter contract, capabilities, and Cursor CLI profile. |
| [`docs/CACHING.md`](docs/CACHING.md) | Layered response cache design: keys, mode safety, bypass. |
| [`docs/CONTEXT_EFFICIENCY.md`](docs/CONTEXT_EFFICIENCY.md) | Context pipeline + **economics lever matrix** (routing, caches, compaction, MCP, approvals, LTM linkage). |
| [`docs/DEPENDENCIES.md`](docs/DEPENDENCIES.md) | Local build/runtime prerequisites by layer. |
| [`docs/CI.md`](docs/CI.md) | CI strategy, gate contracts, and merge protections. |
| [`docs/DOCUMENTATION.md`](docs/DOCUMENTATION.md) | Documentation checklist and writing standards. |
| [`docs/LONG_TERM_MEMORY.md`](docs/LONG_TERM_MEMORY.md) | Long-term memory design hub (**bets**, optimization-first; not Phase 1). |

## Workspace layout

- `rex-proto`: protobuf/gRPC contract generation (`rex.v1`).
- `rex-daemon`: daemon runtime with status and mock streaming RPCs over UDS.
- `rex-cli`: thin client with `status` and `complete` commands.
- `extensions/rex-vscode`: VS Code/Cursor extension (chat UI, `rex-cli` integration, optional daemon auto-start).

## Contributing and validation baseline

- Keep changes focused and align with repository docs.
- Treat generated/runtime outputs as disposable and keep commits source-first:
  - do not commit `target/`, temporary test outputs, or scratch/handoff files;
  - treat local VSIX artifacts as build outputs unless a release flow explicitly requires them.
- Run CI-aligned local checks before PRs:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```

For reliability-specific work, follow the full sequence in [`docs/CI.md`](docs/CI.md).

Branch protection should require only `ci-checks`, which aggregates `rust-checks` and `extension-checks`.

## License

REX is licensed under MIT (`workspace.package.license` in `Cargo.toml`).
