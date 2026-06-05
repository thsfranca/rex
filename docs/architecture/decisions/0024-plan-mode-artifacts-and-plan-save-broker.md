# ADR 0024: Plan mode artifacts and plan.save broker

- **Date:** 2026-06-04
- **Status:** Accepted

## Context

Rex **`plan` mode** enforced read/list tools but lacked structured plans, workspace persistence, and **Build → agent** handoff. Rex adds product behavior without general writes in plan mode ([ADR 0013](0013-access-policy-broker-completion.md)).

## Decision

1. **`BrokerSavePlan`** / sidecar `plan.save` — only write path in `plan` mode.
2. Paths under **`.rex/plans/*.md`** only; protected-path rules apply.
3. Sidecar **`plan.clarify`** and **`plan.final`** JSON; NDJSON **`plan`** events.
4. Hub: [PLANNING_TOOLS.md](../../PLANNING_TOOLS.md).

### Mode matrix (amendment)

| Capability | `ask` | `plan` | `agent` |
|------------|-------|--------|---------|
| `plan.save` | Deny | Allow `.rex/plans/*.md` | Deny |

## Related

- [ADR 0013](0013-access-policy-broker-completion.md)
- [PLANNING_TOOLS.md](../../PLANNING_TOOLS.md)
