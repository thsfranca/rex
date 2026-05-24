# Roadmap

**Purpose:** track **post-v1.0** **Should** / **Could** work and closure of **Should** release criteria (**RC-S***) in **[V1_0.md](V1_0.md)**. Must **RC-*** remain canonical in that hub. [PURPOSE_AND_PRINCIPLES.md](PURPOSE_AND_PRINCIPLES.md) states intent; [MVP_SPEC.md](MVP_SPEC.md) is Phase 1 **architecture and scope** (no separate completion status). [PRIORITIZATION.md](PRIORITIZATION.md) describes MoSCoW bucketing and light R-ICE scoring.

**Version:** workspace **`1.0.0`** — all Must **RC-*** in [V1_0.md](V1_0.md) are **Met**.

## Release criteria status

Canonical definitions and evidence: **[V1_0.md](V1_0.md)**. Update status there first, then this mirror.

| ID | Status |
|----|--------|
| RC-01 | Met |
| RC-02 | Met |
| RC-03 | Met |
| RC-04 | Met |
| RC-05 | Met |
| RC-06 | Met |
| RC-07 | Met |
| RC-08 | Met |
| RC-09 | Met |
| RC-10 | Met |

### Should criteria (not blocking `1.0.0`)

| ID | Status | Notes |
|----|--------|-------|
| RC-S1 | Met | Extension `rex.modelId` → `--model` — [EXTENSION_ROADMAP.md](EXTENSION_ROADMAP.md) |
| RC-S2 | Open | Long-session extension stress — cancel returns UI to idle |

## Theme order (dependency mental model)

```mermaid
flowchart LR
  core[CoreStreaming_RC01_RC07]
  side[Sidecar_RC03]
  broker[BrokeredHttpTool_RC04]
  ext[Extension_RC02_RC06]
  policyBroker[AccessPolicyBroker_RC05]
  router[RouterObservability_RC09]
  v1[v1_0_all_Must_RC_Met]
  core --> side
  side --> broker
  broker --> ext
  side --> policyBroker
  core --> router
  ext --> v1
  policyBroker --> v1
  router --> v1
```

## Now — v1.0 Must criteria closed

All Must **RC-*** rows in [V1_0.md](V1_0.md) are **Met**. Follow-up work is **Should** / **Could** / **Later** below.

| Priority | What / why | RC-* | Notes |
|----------|------------|------|-------|
| **Should** | Long-session extension stress | RC-S2 | Cancel-to-idle under load — [EXTENSION_ROADMAP.md](EXTENSION_ROADMAP.md) |
| **Should** | Stream/log polish beyond baseline | RC-07 (Met) | Optional hardening only |

## Next — product agent program

Canonical design: **[AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md)**. Today the supervised sidecar is **`rex-sidecar-stub`** (harness); **`rex-agent`** is planned.

**Priority rationale:** Primary focus is **R015 → R019** (JSON config, then single-active **`rex-agent`**). **R013** and **R014** are **Done**. **RC-S2** may run in parallel (extension-only blast radius).

| Order | Theme | ID | Outcome |
|-------|-------|-----|---------|
| 1 | Doc truth (stub vs product) | — | Hubs state planned agent; stub = harness |
| 2 | Platform enablers | **R013** | Done — `BrokerListDir`, `RunTurn.model`, stream passthrough |
| 3 | Unified `rex` CLI | **R014** | Done — single `rex` binary; subcommands |
| 4 | Config + proto SDK | **R015** | JSON config, `rex proto install`, `proto.gen_root` |
| 5 | `rex-agent` scaffold | **R017** | gRPC server + broker client |
| 6 | LangGraph agent core | **R018** | ReAct loop, broker adapters |
| 7 | Integration / E2E | **R019** | Operator path, extension defaults, RC evidence when proven |
| 8 | Multi-active broadcast | **R016** | `sidecars.active[]`, broadcast `RunTurn` (**Could** — open decision) |

```mermaid
flowchart TD
  doc[DocTruth]
  plat[R013_Platform]
  cli[R014_rex_CLI]
  cfg[R015_Config_proto]
  scaffold[R017_agent_scaffold]
  graph[R018_LangGraph]
  e2e[R019_Integration]
  multi[R016_Multi_active]
  doc --> plat
  plat --> cli
  cli --> cfg
  cfg --> scaffold
  scaffold --> graph
  graph --> e2e
  e2e -.-> multi
```

## Next — after v1.0 or in parallel if healthy

| Priority | What / why | Source(s) | Notes |
|----------|------------|-----------|--------|
| **Could** | **MCP** interoperability (design accepted; implementation deferred) | [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md), [ADR 0008](architecture/decisions/0008-dedicated-sidecar-control-plane-api.md) | Formal MCP ADR when scheduled |
| **Could** | Learned / small-model compression; batching/async doc jobs | [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) | Matrix **planned** rows |
| **Could** | Layered prompts (system/project stack) | [CONFIGURATION.md](CONFIGURATION.md#layered-prompts-planned) | **planned** |
| **Could** | Difficulty-based routing cascade (ML escalation) | [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md), [ADR 0004](architecture/decisions/0004-routing-daemon-first-optional-http-gateway.md) | Beyond **RC-09** env hook |
| **Won't (now)** | Direct daemon HTTP/mock without sidecar | [MVP_SPEC.md](MVP_SPEC.md) | CI/harness path only; not product default |

## Later — only if the core path stays healthy

| Priority | What | Source(s) | Notes |
|----------|------|-----------|--------|
| **Could** | L2 **semantic** cache | [CACHING.md](CACHING.md), [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md) | Out of v1.0 |
| **Could** | **Apple MLX** local model path | [ADAPTERS.md](ADAPTERS.md#local-mlx-path-planned) | Post-v1.0 |
| **Could** | Gateway adapters beyond broker HTTP | [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md), [ADR 0004](architecture/decisions/0004-routing-daemon-first-optional-http-gateway.md) | After router story matures; multi-sidecar broadcast → **R016** |
| **Could** | Vendor KV / prompt cache hints | [CACHING.md](CACHING.md#vendor-kv-and-prompt-cache-hints-planned) | Depends on outbound API owning runtime |
| **Won't (now)** | VM/container as **default Mac** sidecar envelope | [AGENT_RUNTIME_ENVIRONMENT.md](AGENT_RUNTIME_ENVIRONMENT.md) | Process + broker instead |

## Engineering backlog (refactor / contract IDs)

| ID | Theme | Priority |
|----|-------|----------|
| R004 | CLI / extension NDJSON seam hardening | Done |
| R005 | Cross-boundary NDJSON conformance tests | Done |
| R007 | Policy engine / cache seams | Done |
| R008 | Centralized agent approvals | Done |
| R009 | Extension contract tests (approval-id, probe recovery) | Done |
| R010 | Broker `fs.write` | Done |
| R011 | Broker `exec.shell` allowlist | Done |
| **R012** | **AccessPolicy broker centralization** (RC-05) | **Done** |
| **R013** | Platform enablers (`BrokerListDir`, `RunTurn.model`, stream passthrough) | Done |
| **R014** | Unified `rex` CLI (replace `rex-cli` / `rex-daemon`) | Done |
| **R015** | JSON config + `rex proto install` + `proto.gen_root` | Should |
| **R016** | Multi-active sidecar broadcast | Could |
| **R017** | `rex-agent` scaffold (gRPC + broker client) | Should |
| **R018** | LangGraph agent core (ReAct, broker tools) | Should |
| **R019** | Integration / E2E (operator path, extension defaults) | Should |

## Parked in design docs

| Topic | When to pull in | Source |
|--------|-----------------|--------|
| **Remote** networking, **TLS**, **production auth** | Operator story + threat model ready | [MVP_SPEC.md](MVP_SPEC.md), [ARCHITECTURE.md](ARCHITECTURE.md) |
| **Wasm** in-process plugins | Sidecar path mature enough to compare | [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md) |
| **JSON config** via **R015** (`$REX_HOME/config.json`, `rex config`, `proto.gen_root`) | R013–R014 landed or in parallel | [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md), [CONFIGURATION.md](CONFIGURATION.md) |
| **Node gRPC `StreamInference`** in extension | New ADR supersedes hybrid policy | [ADR 0007](architecture/decisions/0007-editor-extension-hybrid-transport-cli-and-grpc.md) |
| **Large** multi-plugin orchestration | Single-plugin supervision stable | [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md) |
| **Long-term / project memory** | Economics path clear | [LONG_TERM_MEMORY.md](LONG_TERM_MEMORY.md) |
| **Agent knowledge** (curated docs for AI, remote/MCP) | Operator pain or bundle design accepted | [AGENT_KNOWLEDGE.md](AGENT_KNOWLEDGE.md) |
| **Observability suite + economics validation** | Design documented; implementation after OTLP/API PRs | [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md), [ADR 0010](architecture/decisions/0010-daemon-exports-observability-via-otel-and-sidecar-api.md) |
| **VM/container sidecar envelope** (server/fleet) | Linux deployment needs stronger isolation | [AGENT_RUNTIME_ENVIRONMENT.md](AGENT_RUNTIME_ENVIRONMENT.md) |

**CI:** [CI.md](CI.md) — mock / self-contained default; live LLM not required on PRs.

## How to refresh this file

1. Update **[V1_0.md](V1_0.md)** **RC-*** status when a gap closes; mirror the compact table above.
2. Skim [MVP_SPEC.md](MVP_SPEC.md) when **scope** changes; [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md), [EXTENSION_ROADMAP.md](EXTENSION_ROADMAP.md) for feature phasing.
3. **New product or feature ideas:** follow [DOCUMENTATION.md — Roadmap and new features](DOCUMENTATION.md#roadmap-and-new-features) (hub first, then row with **Source(s)** link).
4. Re-check [PRIORITIZATION.md](PRIORITIZATION.md) when moving rows.

### Prioritization audit (2026-05-23)

Roadmap rows checked against [PRIORITIZATION.md](PRIORITIZATION.md): MoSCoW labels, hub links for **Could** / **Won't (now)**, economics-matrix coherence, and **RC-S*** mirrors. Re-run when priorities shift materially.

## Related

- [V1_0.md](V1_0.md) — release criteria (canonical **done**)
- [MVP_SPEC.md](MVP_SPEC.md) — Phase 1 architecture
- [docs/README.md](README.md) — documentation index
- [PRIORITIZATION.md](PRIORITIZATION.md) — bucketing and scoring
