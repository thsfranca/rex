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
| Local process execution | Unified **`rex`** binary (`daemon`, `status`, `complete`); shims `rex-daemon` / `rex-cli`. | No external DB/cache/broker required in MVP. |

### MVP: sidecar agent + brokered HTTP

- **Sidecar binary** supervised by `rex-daemon` — harness **`rex-sidecar-stub`** (default) or product scaffold **`rex-agent`** ([sidecars/rex-agent/README.md](../sidecars/rex-agent/README.md)) — [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md), [MVP_SPEC.md](MVP_SPEC.md).
- Reachable **chat/completions** endpoint for brokered inference (Ollama, LM Studio, vLLM, OpenAI API, or compatible proxy).
- Configure inference via `$REX_ROOT/config.json` — [CONFIGURATION.md](CONFIGURATION.md).

### Python sidecar (`rex-agent`)

| Dependency | Why | Notes |
|---|---|---|
| Python 3.10+ | Runs `rex-agent` (LangGraph state typing) | CI prefers `python3.11` / `python3.10` when available |
| `grpcio`, `protobuf` | gRPC runtime for sidecar + broker client | `pip install` with the sidecar package |
| `langgraph`, `langchain-core` | ReAct graphs (**R018**) | Installed in CI via [`run_rex_agent_checks.sh`](../scripts/ci/run_rex_agent_checks.sh) |
| `grpcio-tools` | Maintainer: `rex proto install` | Same as `rex proto doctor` toolchain |
| `pytest` | Sidecar unit tests | CI: [`run_rex_agent_checks.sh`](../scripts/ci/run_rex_agent_checks.sh) |

### Builtin sidecar CI

| Script | Purpose |
|--------|---------|
| [`run_sidecar_verify.sh`](../scripts/ci/run_sidecar_verify.sh) | CI gate for builtins in [`builtin_sidecars.txt`](../scripts/ci/builtin_sidecars.txt) |
| [`run_stub_sidecar_checks.sh`](../scripts/ci/run_stub_sidecar_checks.sh) | TestExecution slice for `rex-sidecar-stub` (called by verify) |
| [`run_rex_agent_checks.sh`](../scripts/ci/run_rex_agent_checks.sh) | TestExecution slice for `rex-agent` (called by verify) |

### Test harness (non-MVP product path)

- `REX_INFERENCE_RUNTIME=mock` for CI and UDS e2e — no network; **`rex-sidecar-stub`** harness exists for CI; operators use JSON config + live HTTP for dogfood.
- Legacy `cursor-cli` subprocess optional.
- Direct daemon HTTP/mock **without** sidecar — harness only per [MVP_SPEC.md](MVP_SPEC.md).

## 3) Dev quality/tooling dependencies

| Dependency | Why it is required | Notes |
|---|---|---|
| `cargo fmt` | Enforces formatting consistency. | Used in CI. |
| `cargo clippy` | Enforces lint quality gates. | CI runs with `-D warnings`. |
| `cargo test` | Validates behavior with automated tests. | Unit tests exist for CLI parsing and daemon behavior. |
| `cargo-audit` | Scans `Cargo.lock` for RustSec advisories. | Policy: [`.cargo/audit.toml`](../.cargo/audit.toml) (`severity_threshold = "low"`, deny yanked). CI: [`run_rust_supply_chain.sh`](../scripts/ci/run_rust_supply_chain.sh). Install: `cargo install cargo-audit`. |
| GitHub Dependabot | Scheduled dependency update PRs (cargo, npm, pip). | [`.github/dependabot.yml`](../.github/dependabot.yml) — grouped weekly PRs (Mon/Tue/Wed), cooldown on npm/pip; **security updates bypass cooldown**. Pip updates `pyproject.toml` constraints only (no lockfile). |
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
cargo audit --version
```

Supply chain audit (maintainers, Rust-relevant changes):

```bash
cargo audit
# or: ./scripts/ci/run_rust_supply_chain.sh
```

## Build and run (current MVP)

1. Build workspace:

```bash
cargo build --workspace
```

2. Configure JSON and start daemon:

```bash
rex config init
# Edit $REX_ROOT/config.json — inference.openai_compat, sidecars
rex daemon
```

3. In another terminal, run CLI:

```bash
rex status
rex complete "hello from rex" --format ndjson --mode ask
```

Readiness note:
- If the CLI starts before daemon socket initialization, it fails fast with a connection timeout.
- Re-run the command after daemon startup completes and `/tmp/rex.sock` is present.
