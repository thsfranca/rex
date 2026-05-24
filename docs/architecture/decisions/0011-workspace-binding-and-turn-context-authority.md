# ADR 0011: Workspace binding and turn context authority

- **Date:** 2026-05-24
- **Status:** Accepted

## Context

Broker RPCs, the lexical indexer, and L1 cache fingerprinting scope to `REX_WORKSPACE_ROOT`. The VS Code extension does not set that variable when spawning the daemon, so the process **cwd** often becomes the implicit workspace. Clients also embed editor context in prompt strings, which duplicates daemon retrieval and wastes tokens on remote and local inference paths.

`rex-agent` needs a stable contract for what arrives on `RunTurn` without reading daemon Rust sources.

## Decision

1. **`rex-daemon` is the authority** for resolved `workspace_root`, `TurnContext` assembly at turn start, and `request_id` / `turn_id` issuance.
2. **Clients supply** workspace intent via config/env and optional `client_hints` (active file path, selection range). Clients must **not** rely on prompt text alone to spoof workspace paths for broker operations.
3. **Product path (fail-closed):** If `REX_WORKSPACE_ROOT` is unset and `REX_ALLOW_CWD_WORKSPACE` is not `1`, broker and indexer operations that require a workspace **fail** with a clear error. Harness/CI may set `REX_ALLOW_CWD_WORKSPACE=1` to preserve cwd fallback.
4. **Phase 1 wire:** `RunTurnRequest.prompt` carries the enriched turn string. **Phase 1b:** additive optional `turn_id` and `context_revision` on `rex.sidecar.v1.RunTurnRequest`.
5. **Initial context vs tool deltas:** Daemon injects lexical/knowledge/memory/prompt stages **once per turn start**. The sidecar may call broker `fs.*` / `exec.shell` for **deltas** (post-edit reads, exploration)—not to re-derive the same indexed chunks the daemon already injected.
6. **Multi-root IDEs:** Phase 1 uses the **primary** workspace folder (`workspaceFolders[0]`). When multiple folders are open, log `workspace.warning=multi_root`. Multi-root lists are deferred.

## Consequences

- **Positive:** Consistent broker sandbox; attributable economics; `rex-agent` can document inputs without proto churn in Phase 1.
- **Negative:** Extension and R015 must set workspace root before product use; fail-closed may break scripts that depended on silent cwd.
- **Risks / follow-up:** Migrate extension off full selection-in-prompt (**C1** — **R019**); daemon fail-closed workspace — **R022**; extension supplies `workspace.root` on spawn — **R019** (R015 config shipped).

## Related

- [DEVELOPMENT_ASSISTANCE_CAPABILITIES.md](../../DEVELOPMENT_ASSISTANCE_CAPABILITIES.md) — Turn contract, conflict register
- [ADR 0008](0008-dedicated-sidecar-control-plane-api.md) — Broker authority
- [CONFIGURATION.md](../../CONFIGURATION.md) — `REX_WORKSPACE_ROOT`

## Market benchmark

- **Codex** binds automation to `workspace-write` sandbox and cwd — REX mirrors with explicit root + broker.
- **Cursor / Copilot** use implicit opened folder — REX makes root **explicit and logged**.
