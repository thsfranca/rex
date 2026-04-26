# REX MVP Spec (Phase 1)

This document defines the first shippable slice for REX.

## MVP goal

- Enable a practical dogfooding loop: use the VS Code/Cursor extension (via `rex-cli`) to develop `rex` in the same workspace.
- Keep that loop usable and testable with stable NDJSON streaming, explicit terminal states, and deterministic failure handling.
- Prove local daemon–client communication over UDS and server-streaming behavior through gRPC. **Default inference plugins** (built-in `InferenceRuntime` implementations) are part of the MVP: **mock** remains the default; the **Cursor CLI** plugin can be **enabled** to **forward prompts** to Cursor’s CLI so day-to-day work is **AI-assisted**, not mock-only. Apple MLX and full sidecar processes stay out of Phase 1 (see [ADAPTERS.md](ADAPTERS.md), [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md)).

## In scope

| Item | Definition |
|---|---|
| Daemon | Rust service that binds `/tmp/rex.sock` and serves gRPC methods. |
| CLI | Rust client that connects to daemon and exercises MVP RPCs. |
| Protocol | `rex.v1` package with status and streaming RPCs. |
| Mock inference | Token/chunk streaming with small delays. |
| Shutdown lifecycle | Graceful termination and socket cleanup. |
| Extension-facing contract | CLI `complete` supports machine-readable stream output for editor integration. |
| Dogfooding workflow | Extension + CLI path is reliable enough for day-to-day `rex` development tasks in the IDE. |
| Default inference plugins | Built-in runtimes the operator can turn on or off; **mock** is always available as the safe default. |
| Cursor CLI plugin | Optional **enableable** default that **forwards prompts** to the Cursor CLI (frontier / account-bound models). When enabled, streaming and terminal NDJSON semantics match the mock path; timeouts and spawn errors are bounded and legible. Config and env: [CONFIGURATION.md](CONFIGURATION.md), [ADAPTERS.md](ADAPTERS.md). |
| Startup reliability | CLI retries bounded daemon-unavailable startup races before failing. |
| Configuration policy (documentation) | `CONFIGURATION.md` defines precedence (defaults, env, and future file/CLI), the Phase 1 `REX_*` catalog, and what remains **unimplemented** until a follow-up. |

## Out of scope

- Apple MLX runtime integration.
- **gRPC sidecar process supervision** and the full **multi-plugin** platform described under “Sidecar platform” in [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md) (MVP uses **in-process** inference plugins/adapters only).
- Direct editor-to-daemon transport (the MVP keeps `rex-cli` as the extension boundary; no editor-specific gRPC client or daemon RPC surface is required).
- Remote networking, TLS, and production authentication.
- **Configuration implementation beyond env:** on-disk user config, project-local config files, `rex config` (or similar) subcommands, and **global** CLI flags on `rex-daemon` / `rex-cli` that override environment variables. These ship in a later phase; Phase 1 **documents** the policy in `CONFIGURATION.md` and keeps **runtime** behavior **env + defaults** as implemented in Rust.

## Protocol requirements (`rex.v1`)

| RPC | Type | Requirement |
|---|---|---|
| `GetSystemStatus` | Unary | Return daemon version, uptime, and active mock model id. |
| `StreamInference` | Server streaming | Return one or more chunks and end with `done = true`. |

`StreamInferenceRequest` includes the prompt plus optional `model` and `mode` string fields. **Backward compatibility:** clients that send only `prompt` behave as before (the daemon applies defaults, for example `ask` and the active mock model id in Phase 1). The extension can adopt these fields later; they are not required for the NDJSON line contract.

## Message expectations

- Request includes prompt text.
- Stream response includes token text, monotonic index, and done flag.
- Client handles abrupt stream termination without deadlock.
- If a server implementation would end `StreamInference` without a `done = true` chunk, the REX **daemon** emits a terminal gRPC `INTERNAL` error (instead of an ambiguous empty end-of-stream) so clients can map to a single user-visible failure. Unit tests in `crates/rex-daemon` cover this path; well-behaved runtimes still end with a final `done` chunk as in the table above.

## Mock inference behavior

- Emit deterministic or pseudo-random chunks.
- Add small async delays to simulate generation.
- Keep inference behind an abstraction so MLX can replace mock logic.

## CLI expectations

| Command shape | Expected behavior |
|---|---|
| `rex-cli status` | Print status from `GetSystemStatus`. |
| `rex-cli complete "<prompt>"` | Render streamed chunks until final done chunk. |
| `rex-cli complete "<prompt>" --format ndjson` | Emit one NDJSON event per stream step (`chunk`, `done`, `error`). |

## Extension consumer contract (MVP)

Cursor extension work in MVP uses the CLI boundary first.

| Event | Required fields | Meaning |
|---|---|---|
| `chunk` | `event`, `index`, `text` | Streamed output chunk. |
| `done` | `event`, `index` | Terminal success marker. |
| `error` | `event`, `message` | Terminal failure marker. |

Contract rules:

- CLI emits one JSON object per line in `ndjson` mode.
- Stream must end with exactly one terminal event (`done` or `error`).
- CLI keeps default human-readable output when `--format` is omitted.

## Inference plugins in MVP vs sidecars (later)

**MVP** ships **in-process** inference plugins (adapters) behind one `InferenceRuntime` seam: **mock** (default) and an **enableable** **Cursor CLI** plugin that **forwards prompts** per [ADAPTERS.md](ADAPTERS.md). That is the minimum to **develop `rex` with AI assistance** (dogfooding) while **CI and automation** keep **mock** as the default (see [DEPENDENCIES.md](DEPENDENCIES.md), [CI.md](CI.md)).

**After MVP** (or in parallel with hardening), REX can promote heavy or third-party work to **gRPC sidecar** processes; start with **gRPC sidecars** and defer **Wasm** for the same reasons as today.

| Decision factor | gRPC sidecar (next for out-of-tree work) | Wasm plugin (deferred) |
|---|---|---|
| Team speed | Reuse existing Python/Go components quickly | Requires Wasm target setup and host ABI design |
| Integration effort | Straightforward process + gRPC contract | Requires runtime embedding and capability model |
| Debugging | Standard process tooling and logs | Additional Wasm host/runtime debugging layer |
| Isolation model | OS process boundary | Strong sandbox, but more host integration work |

**Design pointers:** `ARCHITECTURE.md` (inference adapters), `ADAPTERS.md`, `CACHING.md`, and the **Cursor CLI inference adapter** table in `PLUGIN_ROADMAP.md` (phased track, including cache and `model`/`mode` evolution).

## Runtime-managed sidecars (design baseline)

This is the baseline for the **gRPC sidecar** plugin-enabled phase after the **in-process** MVP slice is stable.

### High-level flow

1. Daemon reads plugin config (runtime, version, entrypoint, capabilities).
2. Daemon starts plugin process with controlled environment variables.
3. Plugin serves gRPC using the shared plugin contract.
4. Daemon performs health checks and routes requests to plugin.
5. Daemon enforces timeout, restart, and shutdown policy.

### Minimal requirements

| Area | Requirement |
|---|---|
| Runtime declaration | Plugin must declare language runtime and required version. |
| Entry point | Plugin must declare startup command or executable path. |
| Compatibility | Plugin must declare supported contract version. |
| Health | Plugin must answer health checks before receiving traffic. |
| Failure behavior | Daemon must surface clear startup and runtime errors. |

## Success criteria

1. CLI connects to daemon through `/tmp/rex.sock`.
2. `GetSystemStatus` returns valid data.
3. `StreamInference` returns multiple chunks and terminates correctly.
4. `rex-cli complete --format ndjson` emits parseable `chunk` and terminal events.
5. CLI handles short daemon startup races with bounded retry behavior.
6. Daemon shutdown removes socket file.
7. CLI fails clearly when daemon is unavailable.
8. Cursor-extension path can stream completions via CLI NDJSON and recover cleanly from terminal failures.
9. The extension + CLI path is stable enough to complete routine `rex` development tasks without leaving the IDE loop.
10. With the **Cursor CLI** inference plugin **enabled** (per `CONFIGURATION.md`), `StreamInference` routes prompts through that path and the CLI/extension still see exactly one terminal NDJSON event and bounded failure behavior.
11. A reader can list configuration **precedence** and every **Phase 1** `REX_*` variable from `CONFIGURATION.md` (canonical with `ARCHITECTURE.md` for boundaries).

### Success criteria to evidence (tests and docs)

| Criterion | Evidence | Notes |
|---|---|---|
| 1. CLI via `/tmp/rex.sock` (default) | `crates/rex-daemon/tests/uds_e2e.rs` (gRPC over UDS to default path in tests) | UDS e2e uses a temp path per test; same transport as spec. |
| 2. `GetSystemStatus` | `crates/rex-daemon/tests/uds_e2e.rs` (`get_system_status`); `crates/rex-daemon/src/service.rs` (unit tests) | |
| 3. `StreamInference` streaming | `crates/rex-daemon/tests/uds_e2e.rs`; `crates/rex-daemon/src/service.rs` tests | |
| 4. NDJSON `chunk` / terminal | `crates/rex-cli/src/runtime.rs` (NDJSON and terminal count unit tests); `crates/rex-cli/src/command.rs` tests | |
| 5. Bounded startup retry | `crates/rex-cli/src/runtime.rs` (`should_retry_stream_start` and related); `crates/rex-daemon/tests/uds_e2e.rs` (race / readiness paths) | |
| 6. Shutdown removes socket | `crates/rex-daemon/src/runtime.rs` (`remove_file` on shutdown path); manual checklist below | **No** single automated test that only asserts post-process socket absent; use checklist + local run. |
| 7. Clear failure when daemon unavailable | `crates/rex-cli/src/runtime.rs` tests; `crates/rex-daemon/src/service.rs` (terminal error when no `done` chunk) | |
| 8. Extension path NDJSON and recovery | `extensions/rex-vscode/src/test/streamClient.test.ts`, `ndjsonParser.test.ts`, `errorTaxonomy.test.ts` | **Plus** [EXTENSION_LOCAL_E2E.md](EXTENSION_LOCAL_E2E.md) and manual chat send/cancel. |
| 9. Stable routine IDE development | [EXTENSION_ROADMAP.md](EXTENSION_ROADMAP.md) (“What remains”); [EXTENSION_LOCAL_E2E.md](EXTENSION_LOCAL_E2E.md) | **Human / dogfooding**; no full automation. |
| 10. `REX_INFERENCE_RUNTIME=cursor-cli` | `crates/rex-daemon/tests/uds_e2e.rs` (Cursor CLI **stub** via `REX_CURSOR_CLI_COMMAND`); [ADAPTERS.md](ADAPTERS.md) | **Optional** real `cursor-agent`; CI uses **mock** / stubs per [CI.md](CI.md). |
| 11. `CONFIGURATION.md` | [CONFIGURATION.md](CONFIGURATION.md) (Phase 1 `REX_*` and precedence) | Re-verify when env wiring changes in the daemon or CLI. |

## Manual validation checklist

Use this list for end-to-end confidence before a release. For day-to-day automation, also run the Rust gate script (`./scripts/ci/test_enforce_rust_gate.sh` or the checks described in [`CI.md`](CI.md)) and `cd extensions/rex-vscode && npm test` when you touch the extension.

**Preflight (automated):** Run [`scripts/verify_mvp_local.sh`](../scripts/verify_mvp_local.sh) from the repo root (see [CI.md](CI.md)) so the workspace build, Rust verify, and extension checks match CI before you spend time on editor-only steps.

- [ ] Run `./scripts/verify_mvp_local.sh` (or the equivalent commands in [`CI.md`](CI.md)).
- [ ] Build workspace with `cargo build --workspace`.
- [ ] Run daemon and confirm socket exists at `/tmp/rex.sock`.
- [ ] Run status command and validate output.
- [ ] Run streaming command and verify incremental output.
- [ ] Run `rex-cli complete "<prompt>" --format ndjson` and confirm `chunk` then terminal event (`done` or `error`).
- [ ] Stop daemon and confirm socket cleanup.
- [ ] Restart daemon and confirm clean bind.
- [ ] Verify bounded startup race handling by running `complete` while daemon starts.
- [ ] Skim `CONFIGURATION.md` for precedence, the Phase 1 `REX_*` list, and “not implemented” items so the document stays accurate when settings change.
- [ ] (Optional, AI-assisted path) Set `REX_INFERENCE_RUNTIME=cursor-cli` and confirm `rex-cli complete "hello" --format ndjson` streams and terminates; use a local Cursor CLI or the stub pattern from [ADAPTERS.md](ADAPTERS.md) if the real binary is not installed.

## Recommended repository structure for MVP

```text
.
├── Cargo.toml
├── README.md
├── docs/
│   ├── README.md
│   ├── DOCUMENTATION.md
│   ├── CONFIGURATION.md
│   ├── DEPENDENCIES.md
│   ├── EXTENSION_MVP.md
│   ├── ARCHITECTURE.md
│   └── MVP_SPEC.md
├── proto/rex/v1/rex.proto
└── crates/
    ├── rex-proto/
    ├── rex-daemon/
    │   └── src/
    │       ├── main.rs
    │       ├── runtime.rs
    │       ├── service.rs
    │       └── domain.rs
    └── rex-cli/
        └── src/
            ├── main.rs
            ├── runtime.rs
            ├── command.rs
            ├── transport.rs
            ├── error.rs
            └── domain.rs
```
