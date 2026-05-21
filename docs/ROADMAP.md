# Roadmap

**Purpose:** track progress until all **Must** release criteria in **[V1_0.md](V1_0.md)** are **Met**. [PURPOSE_AND_PRINCIPLES.md](PURPOSE_AND_PRINCIPLES.md) states intent; this file is the short work queue. [PRIORITIZATION.md](PRIORITIZATION.md) describes MoSCoW bucketing and light R-ICE scoring.

**Version:** workspace remains **`0.1.0`** (SemVer unstable API) until v1.0 criteria are Met—then tag **`1.0.0`** per [V1_0.md](V1_0.md).

**Specs:** [MVP_SPEC.md](MVP_SPEC.md) (Phase 1 baseline); [EXTENSION.md](EXTENSION.md) (NDJSON contract); [EXTENSION_ROADMAP.md](EXTENSION_ROADMAP.md) (extension phasing).

## Milestone ladder

| Stage | Meaning |
|-------|---------|
| **Phase 1 baseline** | Sidecar + broker + extension loop shipped ([MVP_SPEC.md](MVP_SPEC.md)) |
| **v1.0** | All Must **RC-*** in [V1_0.md](V1_0.md) **Met** |
| **Post-v1.0** | Next / Later / Parked below |

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

## Baseline — Phase 1 (implemented; feeds v1.0)

| What | RC-* | Source | Status |
|------|------|--------|--------|
| UDS + gRPC + streaming under bad paths | RC-01, RC-07 | [MVP_SPEC.md](MVP_SPEC.md), [ARCHITECTURE.md](ARCHITECTURE.md) | Implemented |
| `rex-cli` NDJSON line-safe; one terminal event | RC-01, RC-07 | [EXTENSION.md](EXTENSION.md), conformance fixtures | Implemented |
| Sidecar supervision (`REX_SIDECAR_*`) | RC-03 | [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md) | Implemented |
| `rex.sidecar.v1` + `rex-sidecar-stub` | RC-03 | [ADR 0008](architecture/decisions/0008-dedicated-sidecar-control-plane-api.md) | Implemented |
| `StreamInference` via sidecar (product path) | RC-03 | [MVP_SPEC.md](MVP_SPEC.md) | Implemented |
| Brokered HTTP inference | RC-04 | [ADAPTERS.md](ADAPTERS.md) | Implemented |
| Brokered `fs.read` | RC-04 (partial vs RC-05) | [AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md) | Implemented |
| Extension modes, approvals, apply, cancel | RC-02, RC-06 | [EXTENSION.md](EXTENSION.md), [EXTENSION_LOCAL_E2E.md](EXTENSION_LOCAL_E2E.md) | Implemented |
| Extension → CLI `--mode` / `--model` | RC-02 | [EXTENSION.md](EXTENSION.md) | Implemented |
| L1 cache (`ask`), policy engine, approvals seam | RC-06, RC-09 | [POLICY_ENGINE.md](POLICY_ENGINE.md), [CACHING.md](CACHING.md) | Implemented |
| Default PR CI (mock / stub; no live LLM) | RC-10 | [CI.md](CI.md) | Implemented |

**Scope note:** Phase 1 acceptance evidence: [MVP_SPEC.md](MVP_SPEC.md) success-criteria table. v1.0 adds **RC-05**, **RC-08**, full **RC-09** observability, and doc/script coherence (**RC-02**).

## Now — close v1.0 gaps

| Priority | What / why | RC-* | “Done enough” | Where |
|----------|------------|------|---------------|-------|
| **Must** | **Centralized AccessPolicy broker** — all sidecar tool paths through one evaluation pipeline; structured deny | RC-05 | Tests; [POLICY_ENGINE.md](POLICY_ENGINE.md) pipeline steps 1–3 | daemon |
| **Must** | **Routing observability** — `route=` plus **decision id** in logs (beyond env-only hook) | RC-09 | Documented policy + grep-able logs | daemon, docs |
| **Must** | **Actionable operator failures** (daemon, sidecar, HTTP backend, PATH) | RC-08 | Extension + CLI hints; tests | extension, rex-cli |
| **Must** | **Doc/script coherence** with Phase 1 shipped state | RC-02 | Hubs match code; `verify_mvp_local.sh` referenced consistently | docs |
| **Should** | Stream/log polish; long-session extension stress | RC-07, RC-S2 | No silent hang; cancel-to-idle | daemon, extension |
| **Should** | Extension **`rex.modelId`** on every complete | RC-S1 | Setting passes `--model` | extension |

## Next — after v1.0 or in parallel if healthy

| Priority | What / why | Source(s) | Notes |
|----------|------------|-----------|--------|
| **Could** | **MCP** interoperability (design accepted; implementation deferred) | [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md), [ADR 0008](architecture/decisions/0008-dedicated-sidecar-control-plane-api.md) | Formal MCP ADR when scheduled |
| **Could** | Learned / small-model compression; batching/async doc jobs | [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) | Matrix **planned** rows |
| **Could** | Layered prompts (system/project stack) | [CONFIGURATION.md](CONFIGURATION.md) | **planned** |
| **Could** | Adaptive retrieval + extractive compression (if not already on your branch) | [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) | Evidence-informed defaults in hub |
| **Could** | Difficulty-based routing cascade (ML escalation) | [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md), [ADR 0004](architecture/decisions/0004-routing-daemon-first-optional-http-gateway.md) | Env hook is a v1.0 **RC-09** target |
| **Harness only** | Direct daemon HTTP/mock without sidecar | [MVP_SPEC.md](MVP_SPEC.md) | CI only |

## Later — only if the core path stays healthy

| Priority | What | Source(s) | Notes |
|----------|------|-----------|--------|
| **Could** | L2 **semantic** cache | [CACHING.md](CACHING.md), [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md) | Out of v1.0 |
| **Could** | **Apple MLX** local model path | [ARCHITECTURE.md](ARCHITECTURE.md), [MVP_SPEC.md](MVP_SPEC.md) | Post-v1.0 |
| **Later** | More sidecars or gateway adapters | [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md), [ADR 0004](architecture/decisions/0004-routing-daemon-first-optional-http-gateway.md) | After router story matures |
| **Won't (now)** | VM/container as **default Mac** sidecar envelope | [AGENT_RUNTIME_ENVIRONMENT.md](AGENT_RUNTIME_ENVIRONMENT.md) | Process + broker instead |

## Engineering backlog (refactor / contract IDs)

Migrated from superseded **`REFACTOR_PROPOSALS`** list — IDs kept for continuity.

| ID | Theme | Priority |
|----|-------|----------|
| R004 | CLI / extension NDJSON seam hardening | Done — piped NDJSON line flush in `rex-cli`; contract in [EXTENSION.md](EXTENSION.md), [MVP_SPEC.md](MVP_SPEC.md) |
| R005 | Cross-boundary contract conformance tests | Done — shared [fixtures/ndjson_contract/](../fixtures/ndjson_contract/README.md), `crates/rex-cli/tests/ndjson_contract_conformance.rs`, extension `ndjson_contract_fixture.test.ts` |
| R007 | Mode orchestrator unified policy boundary | Done — [POLICY_ENGINE.md](POLICY_ENGINE.md); `cache_decision=` per [CACHING.md](CACHING.md) |
| R008 | Agent execution approvals / checkpoints centralized | Done — [ADR 0009](architecture/decisions/0009-centralized-agent-approvals-and-checkpoints.md); `REX_AGENT_APPROVALS`; extension `--approval-id` wired |
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
2. Skim [MVP_SPEC.md](MVP_SPEC.md), [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md), [EXTENSION_ROADMAP.md](EXTENSION_ROADMAP.md) when changing direction.
3. New ideas: design doc first, then a row with **RC-*** link where applicable.
4. Re-check [PRIORITIZATION.md](PRIORITIZATION.md) when moving rows.
5. Do not commit per-PR handoff files; use gitignored `TEMP_*` or GitHub PR UI.

## Related

- [V1_0.md](V1_0.md) — release criteria (canonical)
- [docs/README.md](README.md) — documentation index
- [PRIORITIZATION.md](PRIORITIZATION.md) — bucketing and scoring
