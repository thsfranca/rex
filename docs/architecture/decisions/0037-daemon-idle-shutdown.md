# ADR 0037: Daemon idle lifecycle and auto-shutdown

- **Date:** 2026-06-21
- **Status:** Accepted

## Context

Detached and CLI-spawned Rex daemons previously ran until an operator sent SIGINT/SIGTERM ([ADR 0035](0035-cli-operator-ux-daemon-lifecycle-and-terminal-ui.md)). That held sidecars and inference helpers in memory after short CLI sessions with no ongoing benefit.

Operators also lacked a distinct signal for “daemon up but not doing work” versus “daemon busy”.

## Decision

1. **Operational lifecycle** exposed on `GetSystemStatus`:
   - **`ready`** — active work in flight (`StreamInference` stream and/or broker RPCs).
   - **`idle`** — daemon reachable, no active work.

2. **Idle shutdown:** when `daemon.idle_shutdown_secs` (default **300**) elapses without **both** work and client status contact, the daemon exits gracefully, stops supervised children, and removes its UDS socket. **`Some(0)`** disables idle shutdown (long-running debug).

3. **Client presence:** `GetSystemStatus` updates last client contact but does **not** count as work and does **not** flip lifecycle to `ready`. Extension periodic probes therefore keep the daemon alive while the editor is open.

4. **Client FSM:** shared states become **`unavailable` → `starting` → `ready` | `idle`**. `ensureRunning` treats **`ready` and `idle`** as satisfied (daemon reachable).

5. **Auto-start unchanged:** clients may spawn a fresh daemon after idle shutdown via existing `daemon.auto_start` behavior.

## Consequences

- **Positive:** Reclaims RAM/CPU after CLI-only use; clearer operator status; extension probes prevent surprise shutdown during open editor sessions.
- **Negative:** Additive proto fields; clients must parse new `rex status` lines.
- **Follow-up:** Document `idle_shutdown_secs: 0` for operators who want a persistent foreground daemon.

## Related

- [CLI_OPERATOR_UX.md](../../CLI_OPERATOR_UX.md)
- [CONFIGURATION.md](../../CONFIGURATION.md)
- [ADR 0035](0035-cli-operator-ux-daemon-lifecycle-and-terminal-ui.md)
