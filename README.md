# REX

REX is a local AI runtime study project for macOS (Apple Silicon), built as a Rust daemon plus thin clients over gRPC/UDS.

## Project status and scope

- MVP implementation is in progress.
- Current focus: reliable local daemon-client streaming behavior.
- Intended use: local development and architecture learning for AI runtime infrastructure.
- Not yet in scope: MLX integration, remote networking/TLS, and production authentication.

## Why REX

REX keeps clients thin and centralizes model/runtime policy in one daemon boundary.

| Component | Role |
|---|---|
| Daemon | Centralize inference, scheduling, and system policy. |
| Clients | Keep UX thin (CLI, editor, scripts) and call one stable protocol. |
| Protocol | Use gRPC over UDS for local, low-latency communication. |

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
- `rex-daemon` emits stream lifecycle logs (`starting`, `streaming`, `completed`) for troubleshooting.

## MVP boundaries

In scope now:
- local daemon-client communication over UDS;
- unary status RPC and server-streaming completion RPC;
- mock inference and shutdown lifecycle reliability.

Out of scope now:
- Apple MLX runtime integration;
- full plugin lifecycle implementation;
- editor extension integration;
- remote networking, TLS, and production authentication.

For full scope details, see [`MVP_SPEC.md`](MVP_SPEC.md).

## Documentation map

| Document | Purpose |
|---|---|
| [`docs/README.md`](docs/README.md) | Documentation index and reading order. |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | System architecture and long-term direction. |
| [`MVP_SPEC.md`](MVP_SPEC.md) | Phase 1 scope, protocol, and acceptance criteria. |
| [`docs/EXTENSION_MVP.md`](docs/EXTENSION_MVP.md) | Cursor extension bootstrap path using CLI NDJSON streaming. |
| [`docs/EXTENSION_ROADMAP.md`](docs/EXTENSION_ROADMAP.md) | Phased roadmap for the VS Code/Cursor extension. |
| [`docs/EXTENSION_ARCHITECTURE.md`](docs/EXTENSION_ARCHITECTURE.md) | Extension component layout and typed message bus. |
| [`docs/PLUGIN_ROADMAP.md`](docs/PLUGIN_ROADMAP.md) | Sidecar-first plugin roadmap and built-in promotion gates. |
| [`docs/CONTEXT_EFFICIENCY.md`](docs/CONTEXT_EFFICIENCY.md) | Token budget contracts, context pipeline, and telemetry defaults. |
| [`docs/DEPENDENCIES.md`](docs/DEPENDENCIES.md) | Local build/runtime prerequisites by layer. |
| [`docs/CI.md`](docs/CI.md) | CI strategy, gate contracts, and merge protections. |
| [`docs/DOCUMENTATION.md`](docs/DOCUMENTATION.md) | Documentation checklist and writing standards. |

## Workspace layout

- `rex-proto`: protobuf/gRPC contract generation (`rex.v1`).
- `rex-daemon`: daemon runtime with status and mock streaming RPCs over UDS.
- `rex-cli`: thin client with `status` and `complete` commands.

## Contributing and validation baseline

- Keep changes focused and align with repository docs.
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
