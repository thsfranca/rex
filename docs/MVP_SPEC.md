# REX MVP Spec (Phase 1)

This document defines the first shippable slice for REX.

## MVP goal

- Enable a practical dogfooding loop: VS Code/Cursor extension via **`rex-cli`** to develop `rex` in the same workspace ([EXTENSION.md](EXTENSION.md)).
- Keep NDJSON streaming, explicit terminal states, and deterministic failures testable ([ARCHITECTURE.md](ARCHITECTURE.md) observability rows).
- Prove **daemon–client** paths over **UDS** / **server-streaming gRPC**.
- Inference: **built-in adapters** (**mock** default; **optional Cursor CLI** subprocess optional for frontier/account models). MLX + optional **supervised sidecars** post-Phase‑1 ([PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md)).

## Product direction (beyond Phase 1 transport)

Ship a **full REX-native development agent** so **routing, compaction, caches, metering, orchestration hooks** converge in **`rex-daemon`** ([ADR 0001](architecture/decisions/0001-daemon-owns-agent-orchestration-and-economics.md)). **Implementation is incremental**: today’s codebase centers **StreamInference plumbing + adapters**; agent loops/tooling MCP + durable memory appear on the roadmap without claiming they already behave like production agents. Durable memory **design bets** (optimization-first) live in [LONG_TERM_MEMORY.md](LONG_TERM_MEMORY.md); Phase 1 does not include that store.

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
| Cursor CLI adapter | Optional **enableable subprocess** invoking Cursor’s CLI (`REX_INFERENCE_RUNTIME=cursor-cli`) — not the authoritative definition of “the REX agent” ([ADAPTERS.md](ADAPTERS.md)). Streams remain terminal-correct; timeouts/spawn bounded — [CONFIGURATION.md](CONFIGURATION.md). |
| Startup reliability | CLI retries bounded daemon-unavailable startup races before failing. |
| Configuration policy (documentation) | `CONFIGURATION.md` defines precedence (defaults, env, and future file/CLI), the Phase 1 `REX_*` catalog, and what remains **unimplemented** until a follow-up. |

## Out of scope

- Apple MLX runtime integration.
- **gRPC sidecar process supervision** and the full **multi-plugin** platform described under “Sidecar platform” in [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md) (MVP uses **in-process** inference plugins/adapters only).
- **Daemon-supervised isolated agent environments** (VM/container transports, guest–host RPC paths): conceptual only—see [AGENT_RUNTIME_ENVIRONMENT.md](AGENT_RUNTIME_ENVIRONMENT.md); not required for Phase 1 acceptance.
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

Extension MVP uses **`rex-cli`**; authoritative consumer contract consolidated in **[EXTENSION.md](EXTENSION.md)**.

| Event | Required fields | Meaning |
|---|---|---|
| `chunk` | `event`, `index`, `text` | Streamed output chunk. |
| `done` | `event`, `index` | Terminal success marker. |
| `error` | `event`, `message` | Terminal failure marker. |

Contract rules:

- CLI emits one JSON object per line in `ndjson` mode; piped consumers get **prompt line delivery** (flush per line — see [EXTENSION.md](EXTENSION.md) stream contract).
- Stream must end with exactly one terminal event (`done` or `error`).
- CLI keeps default human-readable output when `--format` is omitted.

## Inference adapters vs optional sidecars

**Phase 1** implements **in-process** **`InferenceRuntime` adapters**: **mock** + optional Cursor CLI subprocess (`ADAPTERS.md`). **Daemon-first** caching/pipeline semantics land here first ([CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md)).

**Future** supervised **sidecar processes** reuse the adapter contract boundary — full lifecycle checklist only in **[PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md)** and **[ARCHITECTURE.md](ARCHITECTURE.md)** (avoid duplication here).

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
10. Optional **`REX_INFERENCE_RUNTIME=cursor-cli`**: prompts traverse the Cursor subprocess adapter; CLI/extension observe **exactly one** NDJSON terminal and bounded failures (**stub-supported in CI**) — [`ADAPTERS.md`](ADAPTERS.md).
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

Repository layout: see **[ARCHITECTURE.md](ARCHITECTURE.md)** (canonical tree including `plugins.rs`, `l1_cache.rs`, `adapters.rs`).
