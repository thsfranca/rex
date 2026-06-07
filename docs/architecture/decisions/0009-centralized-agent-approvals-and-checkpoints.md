# ADR 0009: Centralized agent execution approvals and checkpoints

- **Date:** 2026-05-05
- **Status:** Accepted

## Context

`rex-daemon` already owns **stream lifecycle**, **adapter envelope**, and (**R007**) the **policy seam** that decides cache outcomes ([`docs/ARCHITECTURE_GUIDELINES.md`](../../ARCHITECTURE_GUIDELINES.md), [`docs/CACHING.md`](../../CACHING.md)). **Agent-mode** enforcement now includes a daemon **`ApprovalGate`** ([`crates/rex-daemon/src/approvals.rs`](../../../crates/rex-daemon/src/approvals.rs)) with opt-in `REX_AGENT_APPROVALS=1`; the **extension** still owns approval **UX** and passes `--approval-id` / `StreamInferenceRequest.approval_id` when required. Mode guardrails remain in [`docs/EXTENSION.md`](../../EXTENSION.md); full tool **sandbox** broker matrix is **partial** ([`docs/CONTEXT_EFFICIENCY.md`](../../CONTEXT_EFFICIENCY.md)).

Pressures forcing this decision:

1. **Multiple clients, one safety surface.** `rex-cli`, the VS Code / Cursor extension, and any future scripted client all reach the same `rex.v1` daemon. Approval logic that lives only in one client cannot enforce safety for the others ([`docs/EXTENSION_ROADMAP.md`](../../EXTENSION_ROADMAP.md) explicitly excludes "unattended multi-file coordinated agent runs without user approvals" because the thin extension cannot replace daemon-side tool policy).
2. **Backlog row R008** in [`docs/ROADMAP.md`](../../ROADMAP.md) calls for **centralized** agent execution approvals / checkpoints. The R007 policy seam ([`POLICY_ENGINE.md`](../../POLICY_ENGINE.md)) gives the daemon a single, tested place for `agent`-mode-specific code paths without widening the stream service surface.
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
6. **Anti-patterns.** Reject: per-client approval logic that diverges between `rex-cli` and the extension; ambient "auto-approve everything" environment switches with no observability; bypassing the policy seam to hardcode approval in the stream service layer.

| Do | Do not |
|----|--------|
| Add the gate behind the existing policy seam (R007) so the gRPC service stays a thin wiring layer | Hardcode approval branches inside stream handling |
| Make `AlwaysAllow` the default so the trait can land before any UX wiring | Default-deny `agent` mode without giving clients a way to express approval |
| Emit a stable observability label for gate decisions (`approval=allow|deny|checkpoint` on agent-mode requests) | Silently drop denied requests; always surface a typed gRPC error |

## Consequences

- **Positive:**
  - One daemon-owned safety surface for all clients (CLI, extension, future scripts).
  - Builds on the R007 policy seam without adding new architectural layers.
  - Default `AlwaysAllow` keeps the introduction PR a true no-op for existing tests and dogfood flows.
- **Negative:**
  - Adds a second policy concern (`ApprovalGate`) alongside `ResponseCache` in `policy.rs`; module grows and may eventually want a sub-module split.
  - **Shipped:** `rex-cli` and the extension pass `approval_id`; daemon logs `approval=allow|deny|checkpoint` on agent-mode streams.
- **Risks / follow-up:**
  - Defining what counts as a "checkpoint" beyond simple allow/deny will drive further design (tool / write-step gating). Today `Checkpoint { reason }` is **reserved** and proceeds without blocking the stream until phase-2 semantics land.
  - `REX_AGENT_APPROVALS` is cataloged in [`docs/CONFIGURATION.md`](../../CONFIGURATION.md).

## Implementation status (2026-06)

- **Activation:** `agent.approvals_enabled` in JSON config (`approval_gate_from_config` at daemon boot). Default off → `AlwaysAllow`.
- **Clients:** Extension and `rex complete` pass `--approval-id` when enforcement is on — [V1_0.md](../../V1_0.md) RC-06.
- **Stable deny string:** Retains `REX_AGENT_APPROVALS=1` label in `ENFORCEMENT_DENY_REASON` for client/test compatibility; env var is **not read**.
- **Access policy broker:** Mode × capability matrix complete per **R020** — [POLICY_ENGINE.md](../../POLICY_ENGINE.md), [ADR 0013](0013-access-policy-broker-completion.md).
- **Reserved:** `Checkpoint` decision variant for future tool-step gating.

## Related

- [ADR 0001](0001-daemon-owns-agent-orchestration-and-economics.md) — daemon owns orchestration and economics.
- [ADR 0003](0003-layered-cache-agent-mode-policy.md) — `agent` mode is the explicit boundary this gate guards.
- [`docs/ARCHITECTURE_GUIDELINES.md`](../../ARCHITECTURE_GUIDELINES.md) — policy vs mechanism, ownership matrix.
- [`docs/EXTENSION.md`](../../EXTENSION.md) — extension owns the approval UX surface.
- [`docs/POLICY_ENGINE.md`](../../POLICY_ENGINE.md) · [`docs/AGENT_ACCESS_POLICY.md`](../../AGENT_ACCESS_POLICY.md) — policy pipeline and access capabilities.
- [`docs/ROADMAP.md`](../../ROADMAP.md) — backlog row **R008**.
