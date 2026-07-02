# ADR 0036: Per-workspace daemon routing

- **Date:** 2026-06-21
- **Status:** Accepted

## Context

Rex used a single global daemon socket (`/tmp/rex.sock`) with config frozen at daemon startup. Running `rex complete` from project B while a daemon started in project A served the wrong broker sandbox. Operators need concurrent work in multiple repositories without manual daemon restarts.

[ADR 0011](0011-workspace-binding-and-turn-context-authority.md) keeps **one resolved workspace root per daemon process** as authority; this ADR adds **routing** so each workspace gets its own daemon + host sidecar pair.

## Decision

1. **Default product path:** `daemon.socket_scope: per_workspace` (implicit when unset). Derive daemon and host sidecar UDS paths from a stable hash of canonical `workspace.root` under `$REX_ROOT/sockets/`.
2. **Legacy escape hatch:** `daemon.socket_scope: global` uses explicit `daemon.socket` (single daemon for all clients).
3. **CLI ensure:** resolve workspace from merged JSON + cwd; autostart with `current_dir` = workspace root; per-socket autostart lock; write `.rex/config.json` `workspace.root` when missing (product path).
4. **Status contract:** `GetSystemStatusResponse.workspace_root` exposes the daemon-bound root; clients treat mismatch as unavailable and restart owned daemons.
5. **Sidecar socket:** host sidecar socket is overridden at daemon spawn to the derived `-sidecar.sock` path (not committed in project JSON).

## Consequences

- **Positive:** Concurrent projects isolate broker, indexer, L1 fingerprint, and sidecar; cwd-aligned CLI behavior matches extension.
- **Negative:** Multiple daemons consume more resources; orphan processes until operator cleanup (follow-up: `rex daemon list/stop`).
- **Risks / follow-up:** VS Code multi-root Phase 2 ([ADR 0011](0011-workspace-binding-and-turn-context-authority.md)); extension folder-switch rebinding (R075 PR2).

## Related

- [CONFIGURATION.md](../../CONFIGURATION.md) — `daemon.socket_scope`, derived paths
- [CLI_OPERATOR_UX.md](../../CLI_OPERATOR_UX.md) — ensure semantics
- [ADR 0011](0011-workspace-binding-and-turn-context-authority.md) — workspace authority
- [ADR 0035](0035-cli-operator-ux-daemon-lifecycle-and-terminal-ui.md) — CLI auto-start
