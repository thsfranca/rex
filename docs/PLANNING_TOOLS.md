# Planning tools (design hub)

**Status:** **implemented** — [ADR 0024](architecture/decisions/0024-plan-mode-artifacts-and-plan-save-broker.md). **R037** / **E-PLAN01** shipped (broker, sidecar stream, extension plan card).

Canonical design for Rex **`plan` mode**: concise tools, stream contract, prompts, and extension handoff to **`agent`**. Baseline: [Cursor Plan Mode](https://cursor.com/docs/agent/plan-mode).

## Purpose

Turn **`plan` mode** from read-only ReAct into a **plan-first product path**: structured artifacts, scoped persistence under `.rex/plans/`, and **Build** handoff without granting general `fs.write` or `exec.shell`.

## Scope

**In**

- Tools: `fs.read`, `fs.list`, `plan.save` (broker), `plan.clarify` / `plan.final` (sidecar JSON).
- Additive NDJSON `plan` event (`draft` | `clarify` | `ready`).
- Extension plan card, Save, Build; path policy mirrored for extension Save.
- Compact plan mode prompt (sidecar interim; daemon `prompts/mode/plan.md` when [ADR 0012](architecture/decisions/0012-layered-prompt-assemblies.md) lands).

**Out**

- Rigid plan-and-execute graph ([ADR 0022](architecture/decisions/0022-viewer-editor-subagent-topology.md)).
- `fs.write` / `exec.shell` in plan mode outside `.rex/plans/`.
- L1 cache for `plan` ([CACHING.md](CACHING.md)).

## Cursor baseline → Rex

| Cursor | Rex |
|--------|-----|
| No execution in plan | Broker denies write/exec except **`plan.save`** under `.rex/plans/` |
| Codebase research | `fs.read` / `fs.list` + daemon context pipeline |
| Clarifying questions | `plan.clarify` → NDJSON `plan` `phase=clarify` |
| Editable plan | Plan card + **`plan.save`** |
| Build in Agent | Extension **Build** → `agent` + plan reference in prompt |

## Tool contracts

See hub sections for `plan.final`, `plan.clarify`, and `plan.save` JSON/RPC shapes.

**NDJSON `plan` event:** `{"event":"plan","index":N,"phase":"ready|clarify|draft","title":"...","detail":"..."}`

Fixture: [`fixtures/ndjson_contract/plan_stream.ndjson`](../fixtures/ndjson_contract/plan_stream.ndjson).

## Error codes

`plan_save_denied`, `plan_path_invalid`, `plan_too_large`, `plan_parse_error` — [ERROR_HANDLING.md](ERROR_HANDLING.md).

## Cross-links

- [EXTENSION.md](EXTENSION.md) · [AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md) · [ROADMAP.md](ROADMAP.md) **R037**
