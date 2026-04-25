# Temporary Task Handoff

This file is a temporary handoff so another agent can continue the work from the current project state.

## Project

- Name: `rex`
- Type: study project for local AI runtime on macOS (Apple Silicon)
- Current phase: MVP implementation in progress

## MVP Purpose (Current)

Prove local daemon-client communication over Unix Domain Sockets (UDS) with gRPC and server-streaming behavior, using mocked inference.

## Scope Snapshot

Based on `MVP_SPEC.md`:

- In scope:
  - Rust daemon bound to `/tmp/rex.sock`
  - Rust CLI client
  - `rex.v1` protocol with:
    - `GetSystemStatus` (unary)
    - `StreamInference` (server streaming)
  - Mock inference chunk streaming
  - Graceful shutdown + socket cleanup
- Out of scope (for MVP):
  - MLX runtime integration
  - full plugin lifecycle
  - editor extension integration
  - remote networking/TLS/auth

## Established Architecture Vision (KISS-Aligned)

Keep architecture minimal and explicit:

1. `rex-cli` as thin client interface.
2. `rex-daemon` as runtime boundary (transport + orchestration + lifecycle).
3. `rex-proto` as single source of truth for contract.
4. gRPC over UDS (`/tmp/rex.sock`) as stable boundary.
5. One extension seam only: internal inference interface (`InferenceEngine` trait) with mock implementation now and MLX later.

Avoid unnecessary layers (event bus, complex DI, plugin runtime internals) during MVP.

## Rule Changes Already Made (Global Cursor Rules)

Global rules were created/updated under `~/.cursor/rules/`:

- `next-step-suggestions.mdc`
  - Suggestions must be goal-oriented, actionable, and reasoned.
  - Must include:
    - current purpose
    - next goal
    - next step
    - specifics for functionality, code logic, architecture
    - why now
- `kiss-architecture-and-implementation.mdc`
  - Enforces KISS principle in architecture and implementation decisions.
  - Requires simple-first approach and explicit justification for complexity.

## Repository Hygiene Change

- `.gitignore` was created with:
  - `.cursor/`

## Recommended PR Sequence (Small-PR Rule)

Use incremental, reviewable PRs with one primary concern each.

### PR 1: Workspace + proto bootstrap

- Add root Cargo workspace and member crates:
  - `crates/rex-proto`
  - `crates/rex-daemon`
  - `crates/rex-cli`
- Add `proto/rex/v1/rex.proto` with MVP contract:
  - `GetSystemStatus` (unary)
  - `StreamInference` (server streaming)
- Add `rex-proto` generation wiring (build script + generated type exports).
- Keep this PR foundation-only (no runtime behavior yet).

Acceptance criteria:

- `cargo build --workspace` succeeds.
- `proto/rex/v1/rex.proto` defines `GetSystemStatus` and `StreamInference`.
- `rex-proto` exposes generated Rust types/stubs consumable by other crates.
- No daemon bind/listen or CLI command behavior is implemented in this PR.

### PR 2: Daemon transport + status

- Bind daemon on `/tmp/rex.sock`.
- Implement `GetSystemStatus` happy path.
- Add graceful shutdown and socket cleanup.

Acceptance criteria:

- Starting daemon creates socket at `/tmp/rex.sock`.
- `GetSystemStatus` returns valid daemon metadata (version/uptime/model id).
- Stopping daemon removes `/tmp/rex.sock`.
- Restart after shutdown binds cleanly without stale-socket failure.

### PR 3: Daemon streaming behavior

- Implement `StreamInference` server-streaming in daemon.
- Emit deterministic mock chunks with monotonic index.
- Ensure explicit final chunk with `done=true`.
- Keep inference behind daemon-side abstraction (`InferenceEngine` seam).

Acceptance criteria:

- `StreamInference` returns multiple chunks for a non-empty prompt.
- Chunk indexes are monotonic.
- Stream ends with an explicit final chunk where `done=true`.
- Inference logic is accessed via daemon-side abstraction, not inlined in transport handler.

### PR 4: CLI consumption + failure paths

- Add CLI `status` command wired to `GetSystemStatus`.
- Add CLI `complete "<prompt>"` to render streamed chunks incrementally.
- Handle daemon unavailable and abrupt stream termination with clear user-facing errors.

Acceptance criteria:

- `rex-cli status` prints values from `GetSystemStatus`.
- `rex-cli complete "<prompt>"` prints chunks incrementally until completion.
- When daemon is unavailable, CLI exits with clear actionable error output.
- When stream ends abruptly, CLI exits cleanly with clear error message (no deadlock/hang).

## Acceptance Checks for Next Agent

Use MVP success criteria:

- CLI connects via `/tmp/rex.sock`
- `GetSystemStatus` returns valid data
- `StreamInference` returns multiple chunks and terminates correctly
- daemon shutdown removes socket
- CLI failure is clear when daemon unavailable

## Notes

- This file is temporary and can be removed after handoff is complete.
