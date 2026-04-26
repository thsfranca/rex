# Dependency Guide

This document describes project dependencies by layer so you can separate what is required now from what is expected in future plugin/tool phases.

## Dependency layers

| Layer | Required now | Purpose |
|---|---|---|
| Build-critical | Yes | Compile code and generate protobuf bindings. |
| Runtime-critical | Yes | Run daemon and CLI locally over UDS. |
| Dev quality/tooling | Yes for maintainers | Keep code quality and CI reliability. |
| Plugin/tool ecosystem | Not yet | Support multi-runtime plugin sidecars and external tools. |

## 1) Build-critical dependencies (required now)

| Dependency | Why it is required | Notes |
|---|---|---|
| Rust toolchain (`rustup`, `cargo`, `rustc`) | Builds all workspace crates and runs binaries. | Workspace uses Rust 2021 edition. |
| `protobuf` / `protoc` | Required by `crates/rex-proto/build.rs` to compile `proto/rex/v1/rex.proto`. | `protoc` must be available on `PATH`. |

Install `protobuf` on macOS:

```bash
brew install protobuf
```

## 2) Runtime-critical dependencies (required now)

| Dependency | Why it is required | Notes |
|---|---|---|
| Unix-like OS with Unix Domain Socket support | Daemon and CLI communicate through `/tmp/rex.sock`. | macOS works out of the box. |
| Local process execution | `rex-daemon` and `rex-cli` run as local processes. | No external DB/cache/broker required in MVP. |

### Optional: Cursor CLI adapter (local only)

- Install Cursor and ensure the CLI is available on your `PATH` when you set `REX_INFERENCE_RUNTIME=cursor-cli`.
- Default CI and headless tests keep the mock runtime; do not require Cursor in automated environments unless the runner already provides it.
- You can also point `REX_CURSOR_CLI_PATH` or `REX_CURSOR_CLI_COMMAND` at a test stub for deterministic local runs.

## 3) Dev quality/tooling dependencies

| Dependency | Why it is required | Notes |
|---|---|---|
| `cargo fmt` | Enforces formatting consistency. | Used in CI. |
| `cargo clippy` | Enforces lint quality gates. | CI runs with `-D warnings`. |
| `cargo test` | Validates behavior with automated tests. | Unit tests exist for CLI parsing and daemon behavior. |
| CI runner with protobuf compiler | Ensures reproducible checks in pull requests. | Workflow installs `protobuf-compiler` on Linux CI. |

## 4) Plugin/tool ecosystem dependencies (planned, not required yet)

These dependencies become required when plugin sidecars are implemented:

| Dependency class | Why it matters |
|---|---|
| Runtime toolchains (for example Python, Go) | Each sidecar plugin may need its own language runtime/interpreter. |
| Plugin-specific binaries/libraries | Plugins may require external tools to execute their workloads. |
| Compatibility metadata and version checks | Daemon must validate plugin/runtime compatibility at startup. |
| Operational tooling (health checks, restart policy, logs) | Keeps multi-process plugin execution reliable. |

## Dependency verification checklist

Run from repository root:

```bash
cargo --version
rustc --version
protoc --version
cargo fmt --version
cargo clippy --version
```

## Build and run (current MVP)

1. Build workspace:

```bash
cargo build --workspace
```

2. Start daemon:

```bash
cargo run -p rex-daemon
```

3. In another terminal, run CLI:

```bash
cargo run -p rex-cli -- status
cargo run -p rex-cli -- complete "hello from rex"
```

Readiness note:
- If the CLI starts before daemon socket initialization, it fails fast with a connection timeout.
- Re-run the command after daemon startup completes and `/tmp/rex.sock` is present.
