# ADR 0011: Workspace binding and turn context authority

- **Date:** 2026-05-24
- **Status:** Accepted (amended 2026-07-04 — cwd-only workspace binding)

## Context

Broker RPCs, the lexical indexer, and L1 cache fingerprinting scope to a resolved **workspace root**. Earlier slices introduced JSON `workspace.root` and a harness-only `workspace.allow_cwd_fallback` flag (R022), which duplicated what operators already express by **where they run `rex`**. Clients also embed editor context in prompt strings, which duplicates daemon retrieval and wastes tokens on remote and local inference paths.

`rex-agent` needs a stable contract for what arrives on `RunTurn` without reading daemon Rust sources.

## Decision

1. **`rex-daemon` is the authority** for resolved `workspace_root`, `TurnContext` assembly at turn start, and `request_id` / `turn_id` issuance.
2. **Workspace root is canonical process cwd** — not a JSON field. CLI spawns the daemon with `current_dir` set to the resolved cwd; broker, indexer, L1 cache, and per-workspace sockets derive from that path. Optional gRPC `client_hints` (active file path, selection text on the wire) supplement context; clients must **not** rely on prompt text alone to spoof workspace paths for broker operations.
3. **Failure mode:** broker paths and `StreamInference` (workspace indexer mode) fail only when `current_dir()` is unavailable (OS-level). There is no fail-closed path for “missing `workspace.root` in JSON.”
4. **Phase 1 wire:** `RunTurnRequest.prompt` carries the enriched turn string. **Phase 1b:** additive optional `turn_id` and `context_revision` on `rex.sidecar.v1.RunTurnRequest`.
5. **Initial context vs tool deltas:** Daemon injects lexical/knowledge/memory/prompt stages **once per turn start**. The sidecar may call broker `fs.*` / `exec.shell` for **deltas** (post-edit reads, exploration)—not to re-derive the same indexed chunks the daemon already injected.
6. **Multi-root / monorepo parent-root scoping:** deferred; cwd is authoritative until a future slice adds explicit override.
7. **Per-folder daemon:** derived socket per [ADR 0036](0036-per-workspace-daemon-routing.md) (**R075** Done).

## Consequences

- **Positive:** Matches CLI north-star (`cd project && rex`); removes dual product/harness config modes; consistent broker sandbox; attributable economics.
- **Negative:** Operators must run `rex` from the intended project directory; monorepo “scope to repo root while cwd is a subdir” is not supported yet.
- **Risks / follow-up:** Thin `client_hints` vs prompt embedding (**C1**); monorepo parent-root override if needed.

## Related

- [DEVELOPMENT_ASSISTANCE_CAPABILITIES.md](../../DEVELOPMENT_ASSISTANCE_CAPABILITIES.md) — Turn contract, conflict register
- [ADR 0008](0008-dedicated-sidecar-control-plane-api.md) — Broker authority
- [CONFIGURATION.md](../../CONFIGURATION.md) — `workspace.indexer` only

## Market benchmark

- **Codex** binds automation to `workspace-write` sandbox and cwd — REX aligns with cwd-as-authority + broker.
- **Cursor / Copilot** use implicit opened folder — REX logs resolved cwd at daemon start (`workspace.root=<path>` in listen log).
