# ADR 0009: Centralized agent execution approvals and checkpoints

- **Date:** 2026-05-05
- **Status:** Accepted

## Context

`rex-daemon` already owns **stream lifecycle**, **adapter envelope**, and now (**R007**) the **policy seam** that decides cache outcomes ([`docs/ARCHITECTURE_GUIDELINES.md`](../../ARCHITECTURE_GUIDELINES.md), [`docs/CACHING.md`](../../CACHING.md)). **Agent-mode** safety, however, lives only in the **extension UX**: today's "approval" rules and mode guardrails are described in [`docs/EXTENSION.md`](../../EXTENSION.md) (`ask` / `plan` / `agent` modes), [`docs/ARCHITECTURE.md`](../../ARCHITECTURE.md) (security viewpoint, "Elevation"), and [`docs/CONTEXT_EFFICIENCY.md`](../../CONTEXT_EFFICIENCY.md) (`Human approvals + sandbox for tools` row, marked **partial**).

Pressures forcing this decision:

1. **Multiple clients, one safety surface.** `rex-cli`, the VS Code / Cursor extension, and any future scripted client all reach the same `rex.v1` daemon. Approval logic that lives only in one client cannot enforce safety for the others ([`docs/EXTENSION_ROADMAP.md`](../../EXTENSION_ROADMAP.md) explicitly excludes "unattended multi-file coordinated agent runs without user approvals" because the thin extension cannot replace daemon-side tool policy).
2. **Backlog row R008** in [`docs/ROADMAP.md`](../../ROADMAP.md) calls for **centralized** agent execution approvals / checkpoints. The R007 policy seam (`crates/rex-daemon/src/policy.rs`) gives the daemon a single, tested place for `agent`-mode-specific code paths to hook in without re-touching `service.rs`.
3. **Architecture guidelines layering.** [`docs/ARCHITECTURE_GUIDELINES.md`](../../ARCHITECTURE_GUIDELINES.md) "When to write or update an ADR vs this file" requires a new ADR for a yes/no boundary decision before code lands; this is that decision.

## Decision

1. **Daemon owns the approval policy boundary.** A new daemon trait `ApprovalGate` is the single decision point for whether an `agent`-mode request is allowed to proceed (and, in future, for intermediate tool / write checkpoints). Policy lives in `rex-daemon`, not in any specific client.
2. **Extension still owns approval UX.** Capturing user intent (prompt, granted permissions, "approve this step" buttons) and surfacing approval-required states stays in [`extensions/rex-vscode`](../../../extensions/rex-vscode/) per [`docs/EXTENSION.md`](../../EXTENSION.md). The extension communicates approval context to the daemon; the daemon authorizes.
3. **Minimum contract** (names only — concrete code lands in subsequent PRs):
   - `ApprovalGate` — daemon trait with a single decision method that consults a per-request `ApprovalContext` and returns an `ApprovalDecision`.
   - `ApprovalContext` — request-scoped inputs the gate may inspect (mode, runtime, prompt directives, future client-supplied permissions).
   - `ApprovalDecision { Allow, Deny { reason }, Checkpoint { reason } }` — `Checkpoint` reserved for future tool / write-step gating without changing the trait shape.
   - **Default impl:** `AlwaysAllow` so introducing the gate is a behavior-preserving refactor.
4. **Activation is opt-in via env.** Enforcement of `agent`-mode denial when no approval context is present ships behind an environment flag (e.g. `REX_AGENT_APPROVALS=1`), following the precedence catalog in [`docs/CONFIGURATION.md`](../../CONFIGURATION.md). Default off; existing flows unchanged until a client supplies approval context.
5. **`ask` and `plan` are out of scope.** The gate is consulted only when policy already routes a request through the `agent` branch of [`docs/architecture/decisions/0003-layered-cache-agent-mode-policy.md`](0003-layered-cache-agent-mode-policy.md). `ask` and `plan` keep today's behavior.
6. **Anti-patterns.** Reject: per-client approval logic that diverges between `rex-cli` and the extension; ambient "auto-approve everything" environment switches with no observability; bypassing the policy seam to hardcode approval inside `service.rs`.

| Do | Do not |
|----|--------|
| Add the gate behind the existing `policy.rs` seam (R007) so `service.rs` stays a thin wiring layer | Hardcode approval branches inside `stream_inference` |
| Make `AlwaysAllow` the default so the trait can land before any UX wiring | Default-deny `agent` mode without giving clients a way to express approval |
| Emit a stable observability label for gate decisions (planned: `approval=allow|deny|checkpoint`) when enforcement is active | Silently drop denied requests; always surface a typed gRPC error |

## Consequences

- **Positive:**
  - One daemon-owned safety surface for all clients (CLI, extension, future scripts).
  - Builds on the R007 policy seam without adding new architectural layers.
  - Default `AlwaysAllow` keeps the introduction PR a true no-op for existing tests and dogfood flows.
- **Negative:**
  - Adds a second policy concern (`ApprovalGate`) alongside `ResponseCache` in `policy.rs`; module grows and may eventually want a sub-module split.
  - Real enforcement requires a way for clients to ship approval context; the extension UX wiring is parked as follow-up after the daemon trait lands.
- **Risks / follow-up:**
  - Defining what counts as a "checkpoint" beyond simple allow/deny will drive further design (likely tied to future tool-execution work). Treat `Checkpoint { reason }` as **reserved** until that design exists.
  - `REX_AGENT_APPROVALS` becomes a new entry in the [`docs/CONFIGURATION.md`](../../CONFIGURATION.md) `REX_*` catalog when enforcement ships.

## Related

- [ADR 0001](0001-daemon-owns-agent-orchestration-and-economics.md) — daemon owns orchestration and economics.
- [ADR 0003](0003-layered-cache-agent-mode-policy.md) — `agent` mode is the explicit boundary this gate guards.
- [`docs/ARCHITECTURE_GUIDELINES.md`](../../ARCHITECTURE_GUIDELINES.md) — policy vs mechanism, ownership matrix.
- [`docs/EXTENSION.md`](../../EXTENSION.md) — extension owns the approval UX surface.
- [`docs/ROADMAP.md`](../../ROADMAP.md) — backlog row **R008**.
