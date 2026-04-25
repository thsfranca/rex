# REX

REX is a **local AI runtime** for macOS (Apple Silicon): a Rust **daemon** owns inference and stream lifecycle, while **thin clients** (CLI, VS Code/Cursor extension, scripts) talk to it over **gRPC on a Unix domain socket**. The repo is both a **learning lab** for AI infrastructure patterns and a **small, testable reference** for how to keep editor and automation surfaces stable while the engine evolves.

## Purpose

- **Single runtime boundary.** One long-lived process holds model/runtime policy, queueing, and shutdown semantics so every client sees the same behavior.
- **Stable tool contract.** Scripts and editors integrate through a narrow surface (`rex-cli` today, shared protobuf types in `rex-proto`) instead of each tool embedding inference details.
- **Streaming-first correctness.** Server-streaming RPCs, explicit terminal states (`done` / `error`), and tests around UDS races and interruption mirror what production local runtimes need before real models land.
- **Room to grow.** Mock inference sits behind a seam intended for Apple MLX, sidecar plugins, and richer policy later—without rewriting clients first.

**Who it is for:** engineers studying daemon-hosted inference, gRPC streaming over UDS, and editor integration patterns; anyone building toward a personal or team **local-first** assistant on Mac.

## What works today (high-value capabilities)

| Capability | What it gives you |
|---|---|
| **gRPC + UDS** | Low-latency local transport on `/tmp/rex.sock` with generated types from one `rex.v1` contract. |
| **Streaming completion** | `StreamInference`-style chunks with deterministic lifecycle logging on the daemon for triage. |
| **`rex-cli`** | `status` and `complete`; human output by default, **`--format ndjson`** for one JSON event per line (`chunk`, `done`, `error`) so extensions and CI can parse streams safely. |
| **Startup and failure behavior** | Bounded retry when the daemon is still booting; clear CLI errors for unavailable daemon, interrupted streams, and bad stream endings. |
| **VS Code / Cursor extension** | Activity-bar chat with streaming markdown, selection-based commands, optional **daemon auto-start**, and install/release docs—see [`extensions/rex-vscode/README.md`](extensions/rex-vscode/README.md) and [`docs/EXTENSION_RELEASE.md`](docs/EXTENSION_RELEASE.md). |
| **Quality gates** | Workspace `cargo` checks plus UDS end-to-end tests covering failure paths, startup races, and post-interruption behavior (see [Operational checks](#operational-checks)). |

## Project status

- MVP implementation is **in progress**; inference is **mocked** with a clean swap-in path for MLX ([`MVP_SPEC.md`](MVP_SPEC.md)).
- Current engineering focus: **reliable daemon–client streaming** and a **stable NDJSON contract** for the extension and other consumers.
- VS Code/Cursor extension baseline is **shipped** (chat UX, NDJSON streaming integration, opt-in daemon auto-start, and release/install pipeline); ongoing work is incremental hardening and follow-on capabilities.
- Not primary scope yet: MLX-backed models, remote networking/TLS, production auth, full plugin sidecar lifecycle.

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

Install or reinstall local binaries:

```bash
chmod +x scripts/install-cli.sh
./scripts/install-cli.sh
```

The script reinstalls with `--force` and does not modify shell dotfiles by default.

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

## MVP boundaries

In scope now:

- Local daemon–client communication over UDS.
- Unary status RPC and server-streaming completion RPC.
- Mock inference and shutdown lifecycle reliability.
- **Editor path:** the VS Code/Cursor extension consumes **`rex-cli`** (including NDJSON streaming); the daemon does not host editor-specific RPCs.

Out of scope for Phase 1 (see [`MVP_SPEC.md`](MVP_SPEC.md)):

- Apple MLX runtime integration.
- Full plugin sidecar lifecycle in the daemon.
- Remote networking, TLS, and production authentication.

## Documentation map

| Document | Purpose |
|---|---|
| [`docs/README.md`](docs/README.md) | Documentation index and reading order. |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | System architecture and long-term direction. |
| [`MVP_SPEC.md`](MVP_SPEC.md) | Phase 1 scope, protocol, and acceptance criteria. |
| [`docs/EXTENSION_MVP.md`](docs/EXTENSION_MVP.md) | Cursor extension bootstrap path using CLI NDJSON streaming. |
| [`docs/EXTENSION_ROADMAP.md`](docs/EXTENSION_ROADMAP.md) | Phased roadmap for the VS Code/Cursor extension. |
| [`docs/EXTENSION_ARCHITECTURE.md`](docs/EXTENSION_ARCHITECTURE.md) | Extension component layout and typed message bus. |
| [`docs/EXTENSION_RELEASE.md`](docs/EXTENSION_RELEASE.md) | Install, daemon auto-start, troubleshooting, and release pipeline for the extension. |
| [`docs/PLUGIN_ROADMAP.md`](docs/PLUGIN_ROADMAP.md) | Sidecar-first plugin roadmap, built-in promotion gates, Cursor adapter phased track. |
| [`docs/ADAPTERS.md`](docs/ADAPTERS.md) | Inference adapter contract, capabilities, and Cursor CLI profile. |
| [`docs/CACHING.md`](docs/CACHING.md) | Layered response cache design: keys, mode safety, bypass. |
| [`docs/CONTEXT_EFFICIENCY.md`](docs/CONTEXT_EFFICIENCY.md) | Token budget contracts, context pipeline, capability-aware policy, telemetry defaults. |
| [`docs/DEPENDENCIES.md`](docs/DEPENDENCIES.md) | Local build/runtime prerequisites by layer. |
| [`docs/CI.md`](docs/CI.md) | CI strategy, gate contracts, and merge protections. |
| [`docs/DOCUMENTATION.md`](docs/DOCUMENTATION.md) | Documentation checklist and writing standards. |

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
