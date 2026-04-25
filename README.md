# REX

REX is a study project for a local AI runtime on macOS (Apple Silicon).

## What this repository contains

- Product and technical documentation.
- Rust workspace scaffolding for the daemon, CLI, and proto crates.
- MVP definition for local gRPC over Unix Domain Sockets (UDS).

## Core idea

| Component | Role |
|---|---|
| Daemon | Centralize inference, scheduling, and system policy. |
| Clients | Keep UX thin (CLI, editor, scripts) and call one stable protocol. |
| Protocol | Use gRPC over UDS for local, low-latency communication. |

## Documentation map

| Document | Purpose |
|---|---|
| [`docs/README.md`](docs/README.md) | Documentation index and structure. |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | System architecture and long-term direction. |
| [`MVP_SPEC.md`](MVP_SPEC.md) | Phase 1 scope, protocol, and acceptance criteria. |
| [`docs/DOCUMENTATION.md`](docs/DOCUMENTATION.md) | Project documentation checklist and writing standards. |

## Current status

- Documentation-first milestone.
- MVP implementation in progress.

## Workspace bootstrap

This repository uses a Rust workspace with three crates:

- `rex-proto`: protobuf/gRPC contract generation (`rex.v1`).
- `rex-daemon`: daemon runtime with status and mock streaming RPCs over UDS.
- `rex-cli`: thin client with `status` and `complete` commands.

Build all workspace members:

```bash
cargo build --workspace
```

## Runtime notes

- Start `rex-daemon` before invoking `rex-cli` commands.
- The CLI now uses conservative connection/request timeouts so startup races fail fast instead of hanging.
- If the daemon is still booting, rerun `status` or `complete` after the socket is ready at `/tmp/rex.sock`.
