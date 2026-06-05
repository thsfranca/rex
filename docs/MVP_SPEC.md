# Phase 1 product architecture

**Scope and shape** for the first REX product path (daemon-supervised sidecar, brokered HTTP, thin extension). **Done** is defined only in **[V1_0.md](V1_0.md)** (`RC-*` release criteria)—not in this file.

## Product goals

- Deliver a **basic development agent** in the VS Code/Cursor extension whose **reasoning and runtime live in a daemon-supervised sidecar** — not in the extension and not as “daemon calls the model directly.”
- Keep the extension a **thin client**: modes, approvals, apply/insert, streaming via **`rex complete`** NDJSON ([EXTENSION.md](EXTENSION.md), [ADR 0007](architecture/decisions/0007-editor-extension-hybrid-transport-cli-and-grpc.md)).
- **`rex-daemon`** supervises the sidecar, **brokers** inference (OpenAI-compatible HTTP) and **at least one host tool** (`fs.read` recommended), and remains **stream- and policy-authoritative** for `rex.v1`.
- **`StreamInference`** for assistant work is **fulfilled through the sidecar**; the daemon maps chunks to the existing NDJSON contract.
- Keep **dogfooding** `rex` from the IDE as the success narrative.

## Stub vs product agent

| | **Shipped today** | **Operator checklist (not “planned product”)** |
|---|---|---|
| Sidecar binary | **`rex-sidecar-stub`** — harness/CI default; `__rex_*` directives | **`rex-agent`** — LangGraph ReAct (**R017–R018** Done); enable via JSON or extension overlay |
| Product sidecar | **`rex-agent`** shipped under `sidecars/rex-agent/` | Live-model proof via [EXTENSION_LOCAL_E2E.md](EXTENSION_LOCAL_E2E.md) §8 |
| CLI | Unified **`rex`** binary (**R014**); **`rex-cli`/`rex-daemon` shims** | — |
| Config | JSON config + `rex config` (**R015**); `rex config init` defaults **stub** | Product path: set `sidecars.active` to **`rex-agent`** or use **`rex.productAgentConfig`** |
| Daemon broker policy | Mode × capability matrix; protected paths (**R020** Done) | — |
| Turn correlation | `turn_id` / `context_revision` on RunTurn (**R021** Done) | — |
| Workspace binding | Fail-closed daemon; extension supplies root (**R022**, **R019** Done) | — |
| Extension defaults | **`rex.productAgentConfig`** default **true** merges **`rex-agent`** + **`agent.approvals_enabled`** on auto-start | Manual JSON still supported |
| v1.0 **RC-*** | **Met** (stub + product paths) | Live HTTP backend for IDE dogfood — [EXTENSION_LOCAL_E2E.md](EXTENSION_LOCAL_E2E.md); reliable plan/agent **tool loop** requires **R038** — [NATIVE_TOOL_CALLING.md](NATIVE_TOOL_CALLING.md) |

## Architecture

```mermaid
flowchart LR
  IDE[Extension]
  CLI[rex_cli]
  Daemon[rex_daemon]
  Sidecar[Agent_sidecar]
  LLM[HTTP_LLM_backend]
  Host[Workspace_host]
  IDE --> CLI
  CLI -->|rex_v1| Daemon
  Daemon -->|supervise| Sidecar
  Sidecar -->|rex_sidecar_v1| Daemon
  Daemon -->|broker_inference| LLM
  Daemon -->|broker_tool| Host
  Daemon -->|chunks| CLI
```

Hub detail: [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md), [AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md), [ADR 0008](architecture/decisions/0008-dedicated-sidecar-control-plane-api.md).

## After v1.0

Converge **routing, compaction, caches, metering, and richer tool/MCP loops** in **`rex-daemon`** and the sidecar envelope ([ADR 0001](architecture/decisions/0001-daemon-owns-agent-orchestration-and-economics.md)). Durable memory and multi-plugin fleets stay on the roadmap ([LONG_TERM_MEMORY.md](LONG_TERM_MEMORY.md), [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md), [ROADMAP.md](ROADMAP.md) **Next/Later**).

## In scope (Phase 1 shape)

| Item | Definition |
|---|---|
| Daemon | `/tmp/rex.sock`; `rex.v1`; policy, broker, sidecar supervisor. |
| CLI | Unified **`rex`**; NDJSON; `--mode` / `--model` on `complete` (shim: `rex-cli`). |
| **Sidecar agent** | One supervised process; agent stack pluggable per [ADR 0005](architecture/decisions/0005-rex-owns-sidecar-environment-not-agent-implementations.md). |
| **`rex.sidecar.v1`** | Control plane distinct from `rex.v1` — verbs in [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md). |
| **Brokered inference** | Daemon runs HTTP OpenAI-compat adapter on sidecar request ([CONFIGURATION.md](CONFIGURATION.md), [ADAPTERS.md](ADAPTERS.md)). |
| **Brokered tool** | At least **`fs.read`** (or bounded **`exec.shell`** if chosen at implementation) — [AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md). |
| Extension | Modes, approvals, apply/insert, cancel, status — [EXTENSION.md](EXTENSION.md). |
| Policy seams | L1 (**`ask`** only), `PolicyEngine`, `ApprovalGate`; context pipeline. |

## Out of scope (Phase 1 shape)

- Multi-plugin fleets, Wasm, VM-default envelope.
- Full MCP catalog in sidecar.
- Extension Node `StreamInference`.
- **Product** path that treats in-process HTTP/mock as the agent (harness/CI only).
- Apple MLX, remote TLS listener, on-disk `rex config`, durable LTM store.

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

## Extension consumer contract

[EXTENSION.md](EXTENSION.md). The extension **depends on** a healthy sidecar-backed assistant; it does not embed the agent runtime.

## Degraded / harness paths

| Path | Use |
|------|-----|
| `REX_INFERENCE_RUNTIME=mock` | CI, `uds_e2e` |
| Direct in-process HTTP without sidecar | Migration and tests only — **not** product acceptance (**RC-03**) |

When sidecar is required but absent, clients must get a **clear error**, not silent fallback that looks like success (**RC-08**).

## Operator verification (supports RC-02 / RC-03)

Use when validating the local path; release-criteria status is tracked in **[V1_0.md](V1_0.md)**.

**Preflight:** [`scripts/verify_mvp_local.sh`](../scripts/verify_mvp_local.sh) — build, Rust/extension CI gates, and **product-path smoke** ([CI.md](CI.md)).

### Automated evidence (CI / local preflight)

Covered by `cargo test -p rex-daemon mvp_product_path` (also run from `verify_mvp_local.sh`):

- [x] Build workspace (via preflight script).
- [x] Sidecar health under daemon supervision (stub spawn + health).
- [x] `StreamInference` **agent** mode uses sidecar **`BrokerInference`** → daemon HTTP (loopback fixture in CI; live JSON `inference.openai_compat` for operator dogfood).
- [x] Brokered **`fs.read`** via prompt `__rex_read:<file>` under `REX_WORKSPACE_ROOT`.
- [x] Required sidecar missing → clear **sidecar** error at daemon startup (no silent success).

### Operator-only (live HTTP backend)

Required for IDE dogfood after preflight passes. Use a running OpenAI-compatible server (Ollama, LM Studio, etc.) — see [EXTENSION_LOCAL_E2E.md](EXTENSION_LOCAL_E2E.md):

- [ ] `rex config init` then edit JSON (`inference.openai_compat`, `sidecars`) — [EXTENSION_LOCAL_E2E.md](EXTENSION_LOCAL_E2E.md) §3.
- [ ] Start **`rex daemon`**; confirm sidecar health in logs.
- [ ] Extension: **agent** mode send (real model text), cancel, apply with approval.
- [ ] Stop daemon; confirm sockets cleaned up.

### Additional hooks

`sidecar_roundtrip.rs`, supervisor in `rex-daemon`, `broker.rs` unit tests, NDJSON conformance fixtures, extension contract tests.

## Related

- [V1_0.md](V1_0.md) — **done** definition (**RC-***)
- [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md) — product agent program (partial — shipped)
- [ROADMAP.md](ROADMAP.md) — work queue
- [ARCHITECTURE.md](ARCHITECTURE.md) — system architecture
