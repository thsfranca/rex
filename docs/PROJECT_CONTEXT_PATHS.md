# Project context paths (design hub)

**Status:** `planned` — **design deferred** (**R066**)

## Purpose

Pre-inject workspace instruction files into daemon turn context before the sidecar agent runs, similar to OpenCode `contextPaths`. Reduces tool rounds for advisory and onboarding prompts when roadmap, rules, and project memory already live in known paths.

## Problem

Rex ask mode starts with the user prompt only. OpenCode loads files such as `OpenCode.md`, `CLAUDE.md`, and `.cursor/rules/` into the system prompt at session start. Rex must tool-read equivalent content, which burns steps and fails on short prompts when retrieval is skipped.

## Scope

**In (this hub stage):**

- Roadmap placeholder (**R066**)
- Problem statement and OpenCode reference path list
- Boundary: daemon `ContextPipeline` owns assembly; sidecar consumes via `RunTurn.prompt` and `injected_files`

**Out (design deferred):**

- Operator-configurable path list and merge order
- Budget shares vs layered prompts and lexical retrieval
- Config schema (`context.paths[]`)
- Whether paths are read once per workspace session or every turn

## Reference paths (OpenCode analogue)

| Path | Role |
|------|------|
| `OpenCode.md` / `opencode.md` | Project memory and commands |
| `CLAUDE.md` / `CLAUDE.local.md` | Agent instructions |
| `.cursor/rules/` | Editor rule snippets |
| `.github/copilot-instructions.md` | Copilot instructions |
| `docs/ROADMAP.md` | Priority queue |
| `docs/PRIORITIZATION.md` | MoSCoW / R-ICE ordering |
| `README.md` | Project overview |

## Boundaries

| Layer | Owns |
|-------|------|
| **Daemon** | Path discovery, bounded read, budget, `[context]` assembly |
| **Sidecar** | Skip redundant `fs.read` when path appears in `injected_files` (R065) |
| **Extension / CLI** | Optional client hints only — not full file preload |

Policy vs mechanism: access policy for protected paths stays in [AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md); this hub covers **what** to inject, not broker enforcement.

## Interfaces (intent)

- Future config key: `context.paths[]` (relative paths under `workspace.root`)
- Future metric: `context_paths=injected|skipped`
- No proto change required for v1 — content flows through existing enriched `RunTurn.prompt`

## Cross-links

- [ROADMAP.md](ROADMAP.md) — **R066**
- [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) — **R067** advisory retrieval (interim until R066 ships)
- [DEVELOPMENT_ASSISTANCE_CAPABILITIES.md](DEVELOPMENT_ASSISTANCE_CAPABILITIES.md) — turn context authority
- [ADR 0012](architecture/decisions/0012-layered-prompt-assemblies.md) — layered prompts
- [AGENT_GRAPH_ARCHITECTURE.md](AGENT_GRAPH_ARCHITECTURE.md) — **R065** injected files manifest

## Acceptance (when implemented)

- Operator-configured paths appear in turn context without sidecar tool calls
- Sidecar `injected_files` manifest includes injected path entries
- Protected paths denied at broker layer unchanged
