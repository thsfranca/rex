# Product agent delivery

**Status: partial — product loop shipped; harness default unchanged.** **`rex-agent`** implements LangGraph ReAct with broker-only LLM and tools (**R018**). CI and harness still default to **`rex-sidecar-stub`**. Operator settings use **JSON config** ([CONFIGURATION.md](CONFIGURATION.md)). Implementation order: [ROADMAP.md](ROADMAP.md) **R013–R022**.

## Problem

[MVP_SPEC.md](MVP_SPEC.md) describes a **basic development agent** whose reasoning lives in a daemon-supervised sidecar. v1.0 **RC-*** are **Met** on platform + **`rex-sidecar-stub`**, which uses one `BrokerInference` call and `__rex_*` prompt directives—not a product tool loop.

## Target architecture

**Shipped (**R018**):** monolithic LangGraph ReAct loop. **Target:** Orchestrator + **Viewer** + **Editor** subgraphs with `RexBrokerChatModel` and intra-turn compaction — canonical design [AGENT_GRAPH_ARCHITECTURE.md](AGENT_GRAPH_ARCHITECTURE.md), [ADR 0022](architecture/decisions/0022-viewer-editor-subagent-topology.md).

```mermaid
flowchart TB
  subgraph clients [Clients]
    Ext[VS_Code_extension]
    CLI[rex_cli]
  end
  subgraph daemon [rex_daemon]
    SI[StreamInference]
    CP[ContextPipeline]
    Pol[Policy_and_Approvals]
    Sup[SidecarSupervisor]
    Brk[Broker_RPCs]
    HTTP[http_openai_compat]
  end
  subgraph sidecar [rex_agent_sidecar]
    GRPC[rex_sidecar_v1_gRPC]
    Orch[Orchestrator]
    Viewer[ViewerSubgraph]
    Editor[EditorSubgraph]
    RexLLM[RexBrokerChatModel]
    RexTools[BrokerTool_wrappers]
    GRPC --> Orch
    Orch --> Viewer
    Orch --> Editor
    Viewer --> RexTools
    Editor --> RexTools
    Orch --> RexLLM
    Viewer --> RexLLM
    Editor --> RexLLM
    RexLLM --> Brk
    RexTools --> Brk
  end
  Ext --> CLI
  CLI --> SI
  SI --> CP --> Pol --> Sup
  Sup -->|RunTurn| GRPC
  Brk --> HTTP
```

| Layer | Owns |
|-------|------|
| Extension / CLI | UX, NDJSON, approvals |
| Daemon | Context injection, policy, stream contract, HTTP to LLM, host execution |
| Python sidecar | Graph state, subagent routing, tool-loop logic, streaming text to daemon |
| LangGraph | Subgraph structure (Orchestrator/Viewer/Editor target), iteration limits |

**Capability contracts** (what the daemon assembles vs what the sidecar owns): [DEVELOPMENT_ASSISTANCE_CAPABILITIES.md](DEVELOPMENT_ASSISTANCE_CAPABILITIES.md) and [sidecars/rex-agent/DESIGN.md](../sidecars/rex-agent/DESIGN.md).

## Product agent (`rex-agent`)

| Item | Planned shape |
|------|----------------|
| Location | `sidecars/rex-agent/` in monorepo — [DESIGN.md](../sidecars/rex-agent/DESIGN.md) |
| Binary name | **`rex-agent`** (LangGraph is internal only) |
| LLM | **Broker-only** — `BrokerInference` via daemon; no direct OpenAI keys in sidecar |
| Tools | Broker RPCs: `fs.read`, `fs.list`, `fs.write`, `exec.shell` (mode-gated) |
| Modes | `ask` (no tools), `plan` (read/list), `agent` (read/list/write/exec) |
| Harness | **`rex-sidecar-stub`** stays for CI; switch via config or `REX_SIDECAR_*` |
| Python lint (**planned**) | **R025** — Ruff on `rex-agent` in CI — [CI_QUALITY_GATES.md](CI_QUALITY_GATES.md) |

### One `RunTurn` flow (target)

1. Daemon calls `RunTurn(prompt, mode, model?)` with context-enriched prompt.
2. Sidecar selects graph by `mode`.
3. Graph runs: LLM → optional tools → LLM until no tool calls or `max_iterations`.
4. Sidecar streams `RunTurnChunk` to daemon; daemon passthrough to `rex.v1` clients.

## Platform enablers (R013)

**Status: implemented.** Additive proto and daemon changes before agent dogfood:

| Change | Why |
|--------|-----|
| **`BrokerListDir`** on `rex.v1` | Agent must explore workspace; `fs.read` alone is insufficient |
| **`RunTurnRequest.model`** | Extension `--model` should reach broker inference |
| **Sidecar stream passthrough** | Long graph runs need incremental chunks, not full-turn buffer |

## Unified CLI (R014)

**Status: implemented.** One **`rex`** binary replaces separate **`rex-cli`** and **`rex-daemon`** entrypoints for operators and the extension:

| Subcommand | Purpose |
|------------|---------|
| `rex daemon` | Run daemon (was `rex-daemon`) |
| `rex status` / `rex complete` | Client RPCs (was `rex-cli`) |
| `rex config` | `init`, `show`, `path`, `validate` |
| `rex proto` | `doctor`, `install`, `path` |
| `rex sidecar` | `list`, `init`, `doctor` |

Extension defaults: **`rex`** + `["daemon"]` for auto-start. Compatibility shims **`rex-cli`** / **`rex-daemon`** delegate to the same libraries with deprecation hints.

## JSON configuration (R015)

**Status: implemented.** Precedence (low → high): defaults → `$REX_ROOT/config.json` → `.rex/config.json` → CLI flags on `rex complete`.

Layout root: **`REX_ROOT`** (default `~/.rex`). Bootstrap with `rex config init`.

### Minimal example (illustrative)

```json
{
  "version": 1,
  "daemon": { "socket": "/tmp/rex.sock" },
  "sidecars": {
    "active": "agent",
    "required": true,
    "list": [
      { "name": "agent", "binary": "rex-agent", "enabled": true },
      { "name": "stub", "binary": "rex-sidecar-stub", "enabled": false }
    ]
  },
  "inference": {
    "openai_compat": {
      "base_url": "http://127.0.0.1:11434/v1",
      "model": "llama3.2"
    }
  },
  "workspace": { "root": "." },
  "agent": { "max_tool_steps": 12 }
}
```

### Proto layout (language-neutral)

```
$REX_ROOT/
  config.json
  proto/
    src/               # canonical .proto sources (repo also ships proto/)
    gen/               # flat generated stubs — path from `rex proto path`
```

- Generated stubs live at **`$REX_ROOT/proto/gen`** (flat layout; no per-sidecar `proto.python_gen_path`).
- **`rex proto install`** materializes stubs when `.proto` changes; use **`rex proto path`** to print the gen directory.

## Daemon prerequisites (R020–R022)

Prerequisites for **`rex-agent`** dogfood (**R017–R018** Done). Design: [DEVELOPMENT_ASSISTANCE_CAPABILITIES.md](DEVELOPMENT_ASSISTANCE_CAPABILITIES.md).

### R020 — Broker access policy completion

**Status: Done.** Completes [ADR 0013](architecture/decisions/0013-access-policy-broker-completion.md) after **R012** (RC-05 read/list protected paths).

| Outcome | Notes |
|---------|--------|
| Mode × capability matrix on all broker RPCs | `ask`/`plan` deny `fs.write` and `exec.shell` |
| Protected-path checks on `fs.write` / `exec.shell` | Same class as read/list |
| `max_tool_result_bytes` from JSON config | Align broker truncation with sidecar scratch (**T5**) |
| Structured deny + `broker.access_policy=` logs | Per capability |

### R021 — Turn correlation Phase 1b

**Status: Done.** Populate optional `turn_id` and `context_revision` on `RunTurn` ([sidecar.proto](../proto/rex/sidecar/v1/sidecar.proto)); correlate logs on stream and broker paths (`turn_id=`, `context_revision=`). C1 hook strips extension `File:`/`Selection:` trailers when retrieval runs. Sidecars forward `x-rex-turn-id` on broker metadata.

### R022 — Workspace binding (daemon)

**Status: Done.** Product path: fail-closed when `workspace.root` unset ([ADR 0011](architecture/decisions/0011-workspace-binding-and-turn-context-authority.md)); harness cwd fallback via `workspace.allow_cwd_fallback` or `REX_ALLOW_CWD_WORKSPACE` in [CONFIGURATION.md](CONFIGURATION.md). Extension supplies root under **R019**.

## R019 — Integration and E2E acceptance

**Status: Done.** Extension workspace binding, `client_hints` on CLI/daemon wire, operator checklist in [EXTENSION_LOCAL_E2E.md](EXTENSION_LOCAL_E2E.md#8-r019-acceptance--live-model-operator-not-ci), and extension operator alignment with **`rex-agent`** (JSON setup hints, **`rex.productAgentConfig`** default, NDJSON **`tool`**/**`step`** cards).

| Criterion | Evidence |
|-----------|----------|
| Extension sets `workspace.root` when auto-starting daemon | Primary `workspaceFolders[0]` |
| Extension **`rex.productAgentConfig`** merges **`rex-agent`** + approvals on auto-start | [extensions/rex-vscode/package.json](../extensions/rex-vscode/package.json); `rex config init` defaults **stub** |
| **C1:** thin `client_hints`; reduce duplicate selection-in-prompt | Document interim double-count until migrated |
| [EXTENSION_LOCAL_E2E.md](EXTENSION_LOCAL_E2E.md) with **live model** (not stub echo) | `ask`, `plan`, `agent` modes |
| Optional: refresh [MVP_SPEC.md](MVP_SPEC.md) stub vs product table | When product agent is proven |

## Multi-active sidecars (R016 — open decision, **Could**)

Roadmap target: **`sidecars.active[]`** with daemon **broadcast** of `RunTurn`. Only one process can bind a UDS path today—implementation options (derived socket per name vs future multiplexer) stay **undecided** until R016. **Defer until single-active `rex-agent` is proven** ([ROADMAP.md](ROADMAP.md) — **Could**, ordered after **R019**).

## Implementation order

See [ROADMAP.md — Next — product agent program](ROADMAP.md#next--product-agent-program).

| ID | Theme | Priority |
|----|-------|----------|
| R013 | Platform enablers | Done |
| R014 | Unified `rex` CLI | Done |
| R015 | JSON config + proto install | Done |
| R020 | Broker access policy completion | Done |
| R021 | Turn correlation Phase 1b | Done |
| R022 | Workspace binding (daemon fail-closed) | Done |
| R017 | `rex-agent` scaffold | Done |
| R018 | LangGraph agent core | Done — prompt JSON tool protocol; [sidecars/rex-agent/DESIGN.md](../sidecars/rex-agent/DESIGN.md) |
| R019 | Integration / E2E | Done |
| R016 | Multi-active broadcast | Could |
| **R027** | Broker baseline hardening | **Should** — `RexBrokerChatModel`, parse recovery, streaming buffer |
| **R028** | Viewer/Editor subagents | **Should** — [AGENT_GRAPH_ARCHITECTURE.md](AGENT_GRAPH_ARCHITECTURE.md) |
| **R029** | Intra-turn state compaction | **Should** — `RemoveMessage`, 25% suffix rule |
| **R030** | Diff-only writes | **Should** — sidecar read→patch→write |
| **R031** | Task-aware read pruning | **Could** — payloads >100 lines |
| **R032** | Token playbook + subagent metrics | **Should** — dedup, hard cap, observability |
| **R033** | Native tools + MCP client | **Could** — [ADR 0016](architecture/decisions/0016-mcp-in-sidecar-envelope.md) Phase 2 |

## Out of scope (this program)

- MCP catalog, multi-plugin fleets, Wasm sidecars
- Cross-turn checkpoint DB (Postgres/SQLite checkpointer)
- LangSmith Deployment / K8s as Rex substitute
- Rust rewrite of product agent
- Bumping v1.0 **RC-*** until agent is proven (R019)

## Related

- [AGENT_GRAPH_ARCHITECTURE.md](AGENT_GRAPH_ARCHITECTURE.md) — token-efficient graph target (**R027–R033**)
- [MVP_SPEC.md](MVP_SPEC.md) — Phase 1 architecture
- [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md) — sidecar runtime hub
- [CONFIGURATION.md](CONFIGURATION.md) — settings policy
- [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md) — plugin platform
- [ADR 0005](architecture/decisions/0005-rex-owns-sidecar-environment-not-agent-implementations.md) · [ADR 0008](architecture/decisions/0008-dedicated-sidecar-control-plane-api.md)
