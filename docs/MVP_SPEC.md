# REX MVP Spec (Phase 1)

This document defines the first shippable slice for REX.

## MVP goal

- Deliver one usable and testable local completion feature that a Cursor extension can consume.
- Prove local daemon-client communication over UDS and server-streaming behavior through gRPC.
- Keep inference mocked, with a clean seam for MLX integration later.

## In scope

| Item | Definition |
|---|---|
| Daemon | Rust service that binds `/tmp/rex.sock` and serves gRPC methods. |
| CLI | Rust client that connects to daemon and exercises MVP RPCs. |
| Protocol | `rex.v1` package with status and streaming RPCs. |
| Mock inference | Token/chunk streaming with small delays. |
| Shutdown lifecycle | Graceful termination and socket cleanup. |
| Extension-facing contract | CLI `complete` supports machine-readable stream output for editor integration. |
| Startup reliability | CLI retries bounded daemon-unavailable startup races before failing. |
| Configuration policy (documentation) | `CONFIGURATION.md` defines precedence (defaults, env, and future file/CLI), the Phase 1 `REX_*` catalog, and what remains **unimplemented** until a follow-up. |

## Out of scope

- Apple MLX runtime integration.
- Full plugin lifecycle implementation.
- Editor extension integration (as a **Phase 1** daemon/CLI deliverable; a companion [VS Code / Cursor extension](EXTENSION_ROADMAP.md) now consumes the `rex-cli` NDJSON contract and does not change the core gRPC/UDS **runtime** requirements above).
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

## Plugin approach decision (post-MVP)

MVP does not implement plugins. For the next phase, start with **gRPC sidecar plugins** and defer **Wasm plugins**.

### Why sidecar first

| Decision factor | gRPC sidecar (chosen first) | Wasm plugin (deferred) |
|---|---|---|
| Team speed | Reuse existing Python/Go components quickly | Requires Wasm target setup and host ABI design |
| Integration effort | Straightforward process + gRPC contract | Requires runtime embedding and capability model |
| Debugging | Standard process tooling and logs | Additional Wasm host/runtime debugging layer |
| Isolation model | OS process boundary | Strong sandbox, but more host integration work |
| MVP alignment | Faster path to prove extension points | Better for later hardening and performance tuning |

### Decision statement

- Choose gRPC sidecar first for delivery speed and simpler integration.
- Revisit Wasm when plugin APIs stabilize and performance/sandbox priorities increase.

**Post-MVP (design only, out of Phase 1 scope):** REX is intended to use an **agnostic inference-adapter** seam for backends such as the mock engine, a future local MLX path, and **Cursor CLI** as a frontier-model gateway. See `ARCHITECTURE.md` (Inference adapters), `ADAPTERS.md`, `CACHING.md`, and the **Cursor CLI inference adapter** table in `PLUGIN_ROADMAP.md` for the phased design track. MVP does not implement these; this pointer exists so future work does not conflict with the MVP contract.

## Runtime-managed sidecars (design baseline)

This is the baseline for the first plugin-enabled phase after MVP.

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
8. Cursor-extension bootstrap path can consume streaming completion via CLI contract.
9. A reader can list configuration **precedence** and every **Phase 1** `REX_*` variable from `CONFIGURATION.md` (canonical with `ARCHITECTURE.md` for boundaries).

## Manual validation checklist

Use this list for end-to-end confidence before a release. For day-to-day automation, also run the Rust gate script (`./scripts/ci/test_enforce_rust_gate.sh` or the checks described in [`CI.md`](CI.md)) and `cd extensions/rex-vscode && npm test` when you touch the extension.

- [ ] Build workspace with `cargo build --workspace`.
- [ ] Run daemon and confirm socket exists at `/tmp/rex.sock`.
- [ ] Run status command and validate output.
- [ ] Run streaming command and verify incremental output.
- [ ] Run `rex-cli complete "<prompt>" --format ndjson` and confirm `chunk` then terminal event (`done` or `error`).
- [ ] Stop daemon and confirm socket cleanup.
- [ ] Restart daemon and confirm clean bind.
- [ ] Verify bounded startup race handling by running `complete` while daemon starts.
- [ ] Skim `CONFIGURATION.md` for precedence, the Phase 1 `REX_*` list, and “not implemented” items so the document stays accurate when settings change.

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
