# ADR 0013: Access policy broker completion

- **Date:** 2026-05-24
- **Status:** Accepted

## Context

`access_policy.rs` today centralizes `fs.read` denials for protected paths (**RC-05**). [POLICY_ENGINE.md](../../POLICY_ENGINE.md) describes a full pipeline: envelope → capability → mode → `ApprovalGate` → broker. `rex-agent` will issue many broker calls per turn; denials must be structured and logged to avoid token-wasting retry loops.

## Decision

1. **Trait name:** `AccessPolicy` — evaluates sidecar broker requests before host execution.
2. **Evaluation order** (single path for broker RPCs):
   - Normalize mode and request metadata
   - `ApprovalGate` when `agent` and policy requires approval
   - `AccessPolicy` capability matrix ([AGENT_ACCESS_POLICY.md](../../AGENT_ACCESS_POLICY.md))
   - Execute on host; log `broker.access_policy=allow|deny` with reason code
3. **Mode × capability** (default product path):

   | Capability | `ask` | `plan` | `agent` |
   |------------|-------|--------|---------|
   | `fs.read` / `fs.list` | Allow workspace | Allow workspace | Allow workspace |
   | `fs.write` | Deny | Deny | Allow workspace (policy) |
   | `exec.shell` | Deny | Deny | Allow allowlist |

4. **Output bounds:** Broker truncates `exec.shell` stdout/stderr and large `fs.read` results to **`max_tool_result_bytes`** (config default in R015). Sidecar session scratch uses the same cap (**T5**).
5. **Does not replace** `ApprovalGate` — approvals are UX/human gates; `AccessPolicy` is technical allow/deny.
6. **OS sandbox** (Seatbelt-class) remains a separate envelope profile; this ADR completes the **broker policy** seam.

## Consequences

- **Positive:** One story for Codex-like sandbox + REX broker; lean error surface for LLM.
- **Negative:** More policy code in daemon; mode matrix must stay in sync with extension UX.
- **Risks / follow-up:** Implement unified evaluator — [ROADMAP.md](../../ROADMAP.md) **R020**; extend tests for deny paths per mode.

## Related

- [ADR 0009](0009-centralized-agent-approvals-and-checkpoints.md)
- [ADR 0008](0008-dedicated-sidecar-control-plane-api.md)
- [POLICY_ENGINE.md](../../POLICY_ENGINE.md)

## Market benchmark

- **OpenAI Codex** separates sandbox mode and approval on escape — REX maps to `AccessPolicy` + `ApprovalGate` + broker.
