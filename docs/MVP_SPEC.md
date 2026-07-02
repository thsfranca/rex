# Phase 1 product architecture

**Scope and shape** for the first REX product path (daemon-supervised sidecar, brokered HTTP, CLI thin client). **Done** is defined only in **[V1_0.md](V1_0.md)** (`RC-*` release criteria)—not in this file.

## Product goals

- Deliver a **basic development agent** via the **`rex` CLI** whose **reasoning and runtime live in a daemon-supervised sidecar** — not in the client and not as “daemon calls the model directly.”
- Keep the CLI a **thin client**: modes, approvals, streaming via **`rex complete`** NDJSON ([NDJSON_STREAM.md](NDJSON_STREAM.md), [ADR 0007](architecture/decisions/0007-editor-extension-hybrid-transport-cli-and-grpc.md)).
- **`rex-daemon`** supervises the sidecar, **brokers** inference (OpenAI-compatible HTTP) and **at least one host tool** (`fs.read` recommended), and remains **stream- and policy-authoritative** for `rex.v1`.
- **`StreamInference`** for assistant work is **fulfilled through the sidecar**; the daemon maps chunks to the existing NDJSON contract.
- Make daemon economics **measurable and operable** via **LangFuse Cloud** (daemon OTLP export + Cloud UI) — [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md), [LANGFUSE_DISCOVERY_ROADMAP.md](LANGFUSE_DISCOVERY_ROADMAP.md). Legacy Rex store/Grafana code cancelled (**LF-R01**).
- Keep **dogfooding** `rex` from the terminal as the success narrative.

## Stub vs product agent

| | **Shipped today** | **Operator checklist (not “planned product”)** |
|---|---|---|
| Sidecar binary | **`rex-sidecar-stub`** — harness/CI default; `__rex_*` directives | **`rex-agent`** — LangGraph ReAct (**R017–R018** Done); default via `rex config init` and install scripts |
| Product sidecar | **`rex-agent`** shipped under `sidecars/rex-agent/` | Live-model proof via [CLI_OPERATOR_UX.md](CLI_OPERATOR_UX.md) |
| CLI | Unified **`rex`** binary (**R014**); **`rex-cli`/`rex-daemon` shims** | — |
| Config | JSON config + `rex config` (**R015**); `rex config init` writes **rex-agent** + mock web search | Edit **`inference.openai_compat`** for live backend |
| Daemon broker policy | Mode × capability matrix; protected paths (**R020** Done) | — |
| Turn correlation | `turn_id` / `context_revision` on RunTurn (**R021** Done) | — |
| Workspace binding | Fail-closed daemon; CLI/config supplies root (**R022** Done) | — |
| v1.0 **RC-*** | **Met** (stub + product paths) | Live HTTP backend for terminal dogfood — [CLI_OPERATOR_UX.md](CLI_OPERATOR_UX.md); plan/agent **tool loop** — **R038** **Done** — [NATIVE_TOOL_CALLING.md](NATIVE_TOOL_CALLING.md) |
| Observability | **Not met** — **RC-LF1** LangFuse Cloud export (**LF-F01**); discovery **LF-D01** | Live smoke (**R039–R040**, **RC-S6** **Met**) — [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md), [ECONOMICS_VALIDATION.md](ECONOMICS_VALIDATION.md) |

## Architecture

```mermaid
flowchart LR
 CLI[rex_cli]
 Daemon[rex_daemon]
 Sidecar[Agent_sidecar]
 LLM[HTTP_LLM_backend]
 Host[Workspace_host]
 LangFuse[LangFuse_Cloud]
 CLI -->|rex_v1| Daemon
 Daemon -->|supervise| Sidecar
 Sidecar -->|rex_sidecar_v1| Daemon
 Daemon -->|broker_inference| LLM
 Daemon -->|broker_tool| Host
 Daemon -->|chunks| CLI
 Daemon -->|otlp_when_obs_enabled| LangFuse
```

Hub detail: [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md), [AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md), [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md), [ADR 0008](architecture/decisions/0008-dedicated-sidecar-control-plane-api.md).

## v1.0 closure (observability Must row)

**v1.0 not Met** until **RC-LF1** closes in [V1_0.md](V1_0.md): LangFuse Cloud receives Rex economics export (**LF-F01**). Opt-in live validation (**RC-S6** **Met**).

After v1.0, converge **routing, compaction, caches, metering, and richer tool/MCP loops** in **`rex-daemon`** and the sidecar envelope ([ADR 0001](architecture/decisions/0001-daemon-owns-agent-orchestration-and-economics.md)). Durable memory and multi-plugin fleets stay on the roadmap ([LONG_TERM_MEMORY.md](LONG_TERM_MEMORY.md), [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md), [ROADMAP.md](ROADMAP.md) **Later**).

## In scope (Phase 1 shape)

| Item | Definition |
|---|---|
| Daemon | `/tmp/rex.sock`; `rex.v1`; policy, broker, sidecar supervisor. |
| CLI | Unified **`rex`**; NDJSON; `--mode` / `--model` on `complete` (shim: `rex-cli`). |
| **Sidecar agent** | One supervised process; agent stack pluggable per [ADR 0005](architecture/decisions/0005-rex-owns-sidecar-environment-not-agent-implementations.md). |
| **`rex.sidecar.v1`** | Control plane distinct from `rex.v1` — verbs in [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md). |
| **Brokered inference** | Daemon runs HTTP OpenAI-compat adapter on sidecar request ([CONFIGURATION.md](CONFIGURATION.md), [ADAPTERS.md](ADAPTERS.md)). |
| **Brokered tool** | At least **`fs.read`** (or bounded **`exec.shell`** if chosen at implementation) — [AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md). |
| CLI consumer | Modes, approvals, cancel, status — [NDJSON_STREAM.md](NDJSON_STREAM.md). |
| Policy seams | L1 (**`ask`** only), `PolicyEngine`, `ApprovalGate`; context pipeline. |
| **Observability JSON** | `observability.enabled`, `observability.otlp` (LangFuse Cloud endpoint when **LF-F01** lands) — [CONFIGURATION.md](CONFIGURATION.md#observability). |
| **LangFuse Cloud** | Primary observability UI and persistence — [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md). Operator LangFuse account when observability enabled. |
| **Economics validation** | Opt-in live Ollama smoke + run manifests — design [ECONOMICS_VALIDATION.md](ECONOMICS_VALIDATION.md); implementation **R039–R042** (**RC-S6**). |

## Observability (Phase 1 shape)

Canonical hub: [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md). **Done** status for **RC-LF1** and **RC-S6** lives in [V1_0.md](V1_0.md)—not here.

| Phase | Deliverable | Status |
|-------|-------------|--------|
| **0** | Stdout economics grep; observability off in JSON | **shipped** |
| **1** | LangFuse discovery (**LF-D01–LF-D10**) | **active** |
| **2** | Daemon OTLP → LangFuse Cloud (**LF-F01**, **RC-LF1**) | **planned** |
| **3** | LiteLLM / sidecar / validation features (**LF-F02–LF-F07**) | **planned** |

Rex-owned store, read API, Grafana suite, and CHCE (**R043–R054**) are **cancelled** — [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md) (historical).

## Out of scope (Phase 1 shape)

- Multi-plugin fleets, Wasm, VM-default envelope.
- Full MCP catalog in sidecar.
- Node gRPC streaming clients.
- **Product** path that treats in-process HTTP/mock as the agent (harness/CI only).
- Apple MLX, remote TLS listener, on-disk `rex config`, durable LTM store.
- Self-hosted LangFuse on Mac as default (Cloud recommended — [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md)).
- Rex-owned observability store and bundled Grafana (cancelled).
- Prompt or file body storage in observability export (metadata-only default).

## Protocol requirements (`rex.v1`)

| RPC | Type | Requirement |
|---|---|---|
| `GetSystemStatus` | Unary | Version, uptime, active model id (broker backend when configured). |
| `StreamInference` | Server streaming | Chunks + terminal `done` or mapped error → NDJSON `error`. |

Assistant modes are **fulfilled through the sidecar path** on the product path; see [V1_0.md](V1_0.md) **RC-03**.

## Sidecar control plane (minimum)

Documented in [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md). Illustrative verbs:

| Verb | Purpose |
|------|---------|
| `Health` / `GetCapabilities` | Supervision and feature flags |
| `RunTurn` | One agent turn; stream text deltas to daemon |
| Brokered inference | Sidecar requests completion; daemon invokes HTTP adapter |
| Brokered tool | At least **`fs.read`** recommended |

## Brokered HTTP (not “daemon = agent”)

- JSON: `inference.openai_compat` in `$REX_ROOT/config.json` — [CONFIGURATION.md](CONFIGURATION.md) (legacy `REX_OPENAI_COMPAT_*` env ignored with warning).
- Daemon **`http_openai_compat`** module is the **broker implementation** when the sidecar (or harness) requests inference.
- Operator profiles: Ollama, LM Studio, OpenAI API — [ADAPTERS.md](ADAPTERS.md).

## CLI expectations

| Command shape | Expected behavior |
|---|---|
| `rex status` | Status from `GetSystemStatus`. |
| `rex complete "<prompt>" --format ndjson --mode <ask\|plan\|agent>` | Forwards to daemon; product path uses sidecar per **RC-03**. |

## NDJSON consumer contract

[NDJSON_STREAM.md](NDJSON_STREAM.md). CLI clients **depend on** a healthy sidecar-backed assistant; they do not embed the agent runtime.

## Degraded / harness paths

| Path | Use |
|------|-----|
| `REX_INFERENCE_RUNTIME=mock` | CI, `uds_e2e` |
| Direct in-process HTTP without sidecar | Migration and tests only — **not** product acceptance (**RC-03**) |

When sidecar is required but absent, clients must get a **clear error**, not silent fallback that looks like success (**RC-08**).

## Operator verification (supports RC-02 / RC-03)

Use when validating the local path; release-criteria status is tracked in **[V1_0.md](V1_0.md)**.

**Preflight:** [`scripts/verify_mvp_local.sh`](../scripts/verify_mvp_local.sh) — build, Rust/sidecar CI gates, and **product-path smoke** ([CI.md](CI.md)).

### Automated evidence (CI / local preflight)

Covered by `cargo test -p rex-daemon mvp_product_path` (also run from `verify_mvp_local.sh`):

- [x] Build workspace (via preflight script).
- [x] Sidecar health under daemon supervision (stub spawn + health).
- [x] `StreamInference` **agent** mode uses sidecar **`BrokerInference`** → daemon HTTP (loopback fixture in CI; live JSON `inference.openai_compat` for operator dogfood).
- [x] Brokered **`fs.read`** via prompt `__rex_read:<file>` under `REX_WORKSPACE_ROOT`.
- [x] Required sidecar missing → clear **sidecar** error at daemon startup (no silent success).

### Operator-only (live HTTP backend)

Required for terminal dogfood after preflight passes. Use a running OpenAI-compatible server (Ollama, LM Studio, etc.) — see [CLI_OPERATOR_UX.md](CLI_OPERATOR_UX.md):

- [ ] `rex config init` then edit JSON (`inference.openai_compat`, `sidecars`) — [CLI_OPERATOR_UX.md](CLI_OPERATOR_UX.md).
- [ ] Run **`rex complete "…" --format ndjson --mode agent`**; confirm real model text in stream.
- [ ] Cancel mid-stream; confirm terminal `error` with actionable code.
- [ ] Stop daemon; confirm sockets cleaned up.

### Observability (supports **RC-LF1**; optional until LangFuse export enabled)

When `observability.enabled: true` and LangFuse OTLP configured (**LF-F01**) — [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md):

- [ ] LangFuse Cloud project created; API keys in env (not committed JSON).
- [ ] Complete one agent turn; confirm economics metadata appears in LangFuse trace UI.
- [ ] Stdout still emits `stream.terminal` and `route=` when Cloud unreachable (phase 0 fallback).

### Additional hooks

`sidecar_roundtrip.rs`, supervisor in `rex-daemon`, `broker.rs` unit tests, NDJSON conformance fixtures, CLI contract tests.

## Related

- [V1_0.md](V1_0.md) — **done** definition (**RC-***, **RC-S***)
- [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md) — product agent program (partial — shipped)
- [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md) — LangFuse observability hub
- [LANGFUSE_DISCOVERY_ROADMAP.md](LANGFUSE_DISCOVERY_ROADMAP.md) — discovery queue
- [ECONOMICS_VALIDATION.md](ECONOMICS_VALIDATION.md) — live validation harness
- [ROADMAP.md](ROADMAP.md) — work queue
- [ARCHITECTURE.md](ARCHITECTURE.md) — system architecture
