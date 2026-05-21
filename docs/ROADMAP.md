# Roadmap

**Purpose:** track progress until all **Must** release criteria in **[V1_0.md](V1_0.md)** are **Met**. That file is the **only** “done” definition. [PURPOSE_AND_PRINCIPLES.md](PURPOSE_AND_PRINCIPLES.md) states intent; [MVP_SPEC.md](MVP_SPEC.md) is Phase 1 **architecture and scope** (no separate completion status). [PRIORITIZATION.md](PRIORITIZATION.md) describes MoSCoW bucketing and light R-ICE scoring.

**Version:** workspace remains **`0.1.0`** (SemVer unstable API) until v1.0 criteria are Met—then tag **`1.0.0`** per [V1_0.md](V1_0.md).

## Release criteria status

Canonical definitions and evidence: **[V1_0.md](V1_0.md)**. Update status there first, then this mirror.

| ID | Status |
|----|--------|
| RC-01 | Partial |
| RC-02 | Partial |
| RC-03 | Met |
| RC-04 | Met |
| RC-05 | Not met |
| RC-06 | Met |
| RC-07 | Partial |
| RC-08 | Partial |
| RC-09 | Partial |
| RC-10 | Met |

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

## Now — close v1.0 gaps

| Priority | What / why | RC-* | “Done enough” | Where |
|----------|------------|------|---------------|-------|
| **Must** | **Centralized AccessPolicy broker** — all sidecar tool paths through one evaluation pipeline; structured deny | RC-05 | Tests; [POLICY_ENGINE.md](POLICY_ENGINE.md) pipeline steps 1–3 | daemon |
| **Must** | **Routing observability** — `route=` plus **decision id** in logs (beyond env-only hook) | RC-09 | Documented policy + grep-able logs | daemon, docs |
| **Must** | **Actionable operator failures** (daemon, sidecar, HTTP backend, PATH) | RC-08 | Extension + CLI hints; tests | extension, rex-cli |
| **Must** | **Doc/script coherence** for operator path and hubs | RC-02 | [MVP_SPEC.md](MVP_SPEC.md), [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md), [CONFIGURATION.md](CONFIGURATION.md) match code; `verify_mvp_local.sh` | docs |
| **Should** | Stream/log polish; long-session extension stress | RC-07, RC-S2 | No silent hang; cancel-to-idle | daemon, extension |
| **Should** | Extension **`rex.modelId`** on every complete | RC-S1 | Setting passes `--model` | extension |

## Next — after v1.0 or in parallel if healthy

| Priority | What / why | Source(s) | Notes |
|----------|------------|-----------|--------|
| **Could** | **MCP** interoperability (design accepted; implementation deferred) | [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md), [ADR 0008](architecture/decisions/0008-dedicated-sidecar-control-plane-api.md) | Formal MCP ADR when scheduled |
| **Could** | Learned / small-model compression; batching/async doc jobs | [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) | Matrix **planned** rows |
| **Could** | Layered prompts (system/project stack) | [CONFIGURATION.md](CONFIGURATION.md) | **planned** |
| **Could** | Adaptive retrieval + extractive compression | [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) | Evidence-informed defaults |
| **Could** | Difficulty-based routing cascade (ML escalation) | [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md), [ADR 0004](architecture/decisions/0004-routing-daemon-first-optional-http-gateway.md) | Beyond **RC-09** env hook |
| **Harness only** | Direct daemon HTTP/mock without sidecar | [MVP_SPEC.md](MVP_SPEC.md) | CI only |

## Later — only if the core path stays healthy

| Priority | What | Source(s) | Notes |
|----------|------|-----------|--------|
| **Could** | L2 **semantic** cache | [CACHING.md](CACHING.md), [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md) | Out of v1.0 |
| **Could** | **Apple MLX** local model path | [ARCHITECTURE.md](ARCHITECTURE.md), [MVP_SPEC.md](MVP_SPEC.md) | Post-v1.0 |
| **Later** | More sidecars or gateway adapters | [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md), [ADR 0004](architecture/decisions/0004-routing-daemon-first-optional-http-gateway.md) | After router story matures |
| **Won't (now)** | VM/container as **default Mac** sidecar envelope | [AGENT_RUNTIME_ENVIRONMENT.md](AGENT_RUNTIME_ENVIRONMENT.md) | Process + broker instead |

## Engineering backlog (refactor / contract IDs)

| ID | Theme | Priority |
|----|-------|----------|
| R004 | CLI / extension NDJSON seam hardening | Done |
| R005 | Cross-boundary NDJSON conformance tests | Done |
| R007 | Policy engine / cache seams | Done |
| R008 | Centralized agent approvals | Done |
| **R012** | **AccessPolicy broker centralization** (RC-05) | **Open** |

## Parked in design docs

| Topic | When to pull in | Source |
|--------|-----------------|--------|
| **Remote** networking, **TLS**, **production auth** | Operator story + threat model ready | [MVP_SPEC.md](MVP_SPEC.md), [ARCHITECTURE.md](ARCHITECTURE.md) |
| **Wasm** in-process plugins | Sidecar path mature enough to compare | [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md) |
| **On-disk** config, **`rex config`**, `.rex.toml` | Precedence specified | [CONFIGURATION.md](CONFIGURATION.md) |
| **Node gRPC `StreamInference`** in extension | New ADR supersedes hybrid policy | [ADR 0007](architecture/decisions/0007-editor-extension-hybrid-transport-cli-and-grpc.md) |
| **Large** multi-plugin orchestration | Single-plugin supervision stable | [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md) |
| **Long-term / project memory** | Economics path clear | [LONG_TERM_MEMORY.md](LONG_TERM_MEMORY.md) |
| **VM/container sidecar envelope** (server/fleet) | Linux deployment needs stronger isolation | [AGENT_RUNTIME_ENVIRONMENT.md](AGENT_RUNTIME_ENVIRONMENT.md) |

**CI:** [CI.md](CI.md) — mock / self-contained default; live LLM not required on PRs.

## How to refresh this file

1. Update **[V1_0.md](V1_0.md)** **RC-*** status when a gap closes; mirror the compact table above.
2. Skim [MVP_SPEC.md](MVP_SPEC.md) when **scope** changes; [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md), [EXTENSION_ROADMAP.md](EXTENSION_ROADMAP.md) for feature phasing.
3. New ideas: design doc first, then a row with **RC-*** link where applicable.
4. Re-check [PRIORITIZATION.md](PRIORITIZATION.md) when moving rows.

## Related

- [V1_0.md](V1_0.md) — release criteria (canonical **done**)
- [MVP_SPEC.md](MVP_SPEC.md) — Phase 1 architecture
- [docs/README.md](README.md) — documentation index
- [PRIORITIZATION.md](PRIORITIZATION.md) — bucketing and scoring
