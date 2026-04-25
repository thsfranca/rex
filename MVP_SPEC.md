# REX MVP Spec (Phase 1)

This document defines the first shippable slice for REX.

## MVP goal

- Prove local daemon-client communication over UDS.
- Prove server-streaming behavior through gRPC.
- Keep inference mocked, with a clean seam for MLX integration later.

## In scope

| Item | Definition |
|---|---|
| Daemon | Rust service that binds `/tmp/rex.sock` and serves gRPC methods. |
| CLI | Rust client that connects to daemon and exercises MVP RPCs. |
| Protocol | `rex.v1` package with status and streaming RPCs. |
| Mock inference | Token/chunk streaming with small delays. |
| Shutdown lifecycle | Graceful termination and socket cleanup. |

## Out of scope

- Apple MLX runtime integration.
- Full plugin lifecycle implementation.
- Editor extension integration.
- Remote networking, TLS, and production authentication.

## Protocol requirements (`rex.v1`)

| RPC | Type | Requirement |
|---|---|---|
| `GetSystemStatus` | Unary | Return daemon version, uptime, and active mock model id. |
| `StreamInference` | Server streaming | Return one or more chunks and end with `done = true`. |

## Message expectations

- Request includes prompt text.
- Stream response includes token text, monotonic index, and done flag.
- Client handles abrupt stream termination without deadlock.

## Mock inference behavior

- Emit deterministic or pseudo-random chunks.
- Add small async delays to simulate generation.
- Keep inference behind an abstraction so MLX can replace mock logic.

## CLI expectations

| Command shape | Expected behavior |
|---|---|
| `rex-cli status` | Print status from `GetSystemStatus`. |
| `rex-cli complete "<prompt>"` | Render streamed chunks until final done chunk. |

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
4. Daemon shutdown removes socket file.
5. CLI fails clearly when daemon is unavailable.

## Manual validation checklist

- [ ] Build workspace with `cargo build --workspace`.
- [ ] Run daemon and confirm socket exists at `/tmp/rex.sock`.
- [ ] Run status command and validate output.
- [ ] Run streaming command and verify incremental output.
- [ ] Stop daemon and confirm socket cleanup.
- [ ] Restart daemon and confirm clean bind.

## Recommended repository structure for MVP

```text
.
├── Cargo.toml
├── README.md
├── ARCHITECTURE.md
├── MVP_SPEC.md
├── docs/
│   ├── README.md
│   └── DOCUMENTATION.md
├── proto/rex/v1/rex.proto
└── crates/
    ├── rex-proto/
    ├── rex-daemon/
    └── rex-cli/
```
