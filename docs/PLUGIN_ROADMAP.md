# Plugin and extensibility roadmap

REX keeps **routing/caching/pipeline policy in `rex-daemon`**, runs the **development agent in a supervised sidecar process**, and uses **in-process inference adapters only as broker mechanisms** (HTTP OpenAI-compat today) — **not** as a substitute for the sidecar agent. See [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md), [MVP_SPEC.md](MVP_SPEC.md), [AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md), ADRs [0001](architecture/decisions/0001-daemon-owns-agent-orchestration-and-economics.md), [0005](architecture/decisions/0005-rex-owns-sidecar-environment-not-agent-implementations.md), [0008](architecture/decisions/0008-dedicated-sidecar-control-plane-api.md).

**Prioritization (2026-06-09):** Streaming/agent **RC-01–RC-10** **Met**; **v1.0 not Met** — observability Must **RC-LF1** blocks the tag. **Next** work is LangFuse discovery (**LF-D01**) — not new broker surfaces. Global queue: [PRIORITIZATION.md](PRIORITIZATION.md#current-focus-queue-audit-2026-06-09).

## Current purpose

- Ship reliable **`rex.v1`** streaming for extensions and CLI.
- **MVP product path:** daemon-supervised **sidecar agent** + **brokered** HTTP inference + **one brokered tool**.
- Concentrate **cost/performance levers** ([CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md)) in the daemon boundary.
- **Harness only:** direct in-process **mock** / HTTP without sidecar for CI and migration.

## Phase 1 platform shape (see [MVP_SPEC.md](MVP_SPEC.md))

Completion status: **[V1_0.md](V1_0.md)** **RC-*** only—not this table.

| Slice | Outcome | RC-* (status in v1 hub) |
|-------|---------|-------------------------|
| **Sidecar lifecycle** | Daemon supervises **0 or 1** process; health, timeout, restart | RC-03 Met |
| **`rex.sidecar.v1`** | Control-plane API on UDS; verbs in [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md) | RC-03 Met |
| **Reference sidecar** | Minimal agent binary (any stack per ADR 0005) | RC-03 Met — **`rex-sidecar-stub`** harness/CI default; **`rex-agent`** shipped — [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md) |
| **`StreamInference` routing** | Assistant completions via sidecar turn, not direct daemon HTTP | RC-03 Met |
| **Brokered HTTP** | `BrokerInference` → `http_openai_compat` when sidecar requests inference | RC-04 Met |
| **Brokered `fs.read`** | Workspace read via broker + centralized access policy | RC-04 Met; RC-05 Met |
| **Brokered `fs.write`** | Bounded workspace write via broker | Shipped (R010) |
| **Brokered `exec.shell`** | Allowlisted programs via broker | Shipped (R011) |

## Brokered HTTP (daemon mechanism)

- JSON: **`inference.openai_compat`** in `$REX_ROOT/config.json` — [CONFIGURATION.md](CONFIGURATION.md). Keys name the **OpenAI-compat protocol**, not OpenAI-the-vendor — [ADAPTERS.md](ADAPTERS.md#terminology-protocol-vs-vendor). Legacy **`REX_OPENAI_COMPAT_*`** env vars are ignored with warning.
- **Multi-provider (default API):** LiteLLM gateway — opt-in managed or external URL — [INFERENCE_GATEWAY.md](INFERENCE_GATEWAY.md), [ADR 0019](architecture/decisions/0019-inference-gateway-opt-in-litellm.md).
- **Not** the product agent: the sidecar requests inference; the daemon executes the HTTP adapter.
- **Legacy:** `REX_INFERENCE_RUNTIME=cursor-cli` subprocess — non-MVP.
- CI: **`mock`** or stub sidecar — [CI.md](CI.md).
- **Done:** native broker tool calling (**R038**) — [NATIVE_TOOL_CALLING.md](NATIVE_TOOL_CALLING.md).
- **Next (Must):** LangFuse Cloud observability (**RC-LF1**, **LF-D01**) — [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md).
- **Next (Should):** live broker validation (**R039–R040**, **RC-S6**) — [ECONOMICS_VALIDATION.md](ECONOMICS_VALIDATION.md).
- **Could:** gateway-path smoke (**R041**); MCP client (**R033**, rank 17).

## Daemon-first principle

| Principle | Meaning |
|-----------|---------|
| Policy in-core | Routing, caches, budgets, terminals owned by **`rex-daemon`**. |
| Agent in sidecar | Reasoning loop and tool **requests** live in the guest process. |
| Broker executes | Host FS, shell, and HTTP inference run **only** through daemon policy. |

## Placement decision gate

| Question | Prefer **in-daemon** | Prefer **sidecar** |
|---|---|---|
| Agent reasoning / tool loop | No | **Yes** |
| Stream contract for clients | **Yes** | No (guest uses `rex.sidecar.v1`) |
| HTTP call to LLM API | **Yes** (broker) | Intent only |
| Another language ML stack | No | Yes |

### Feature placement

| Area | Placement | Notes |
|---|---|---|
| L1/policy caches | **Daemon** | [CACHING.md](CACHING.md) |
| Context pipeline (pre-sidecar) | **Daemon** | Before delegating to sidecar |
| Agent turn + tools | **Sidecar** | MVP |
| Hybrid routing cascade | **Daemon** router (planned) | [ADR 0004](architecture/decisions/0004-routing-daemon-first-optional-http-gateway.md) |

## Inference adapter phases (broker layer)

| Phase | Outcome |
|---|---|
| 1 — Adapter seam | `InferenceRuntime`, `AdapterCapabilities` |
| 2 — HTTP OpenAI-compat | Broker backend for sidecar — **implemented** |
| 3 — L1 cache | **`ask`** only — [ADR 0003](architecture/decisions/0003-layered-cache-agent-mode-policy.md) |
| 4 — Proto / CLI knobs | **`model`** / **`mode`** on wire |
| 4b — Inference Gateway | **Done** — managed LiteLLM supervisor + `rex gateway` — [INFERENCE_GATEWAY.md](INFERENCE_GATEWAY.md), [ADR 0019](architecture/decisions/0019-inference-gateway-opt-in-litellm.md) |
| 5+ | L2 semantic cache | **Could** — [ROADMAP.md](ROADMAP.md) Later, [CACHING.md](CACHING.md) |
| 5+ | Difficulty-based routing cascade | **Could** — [ROADMAP.md](ROADMAP.md) Later, [ADR 0004](architecture/decisions/0004-routing-daemon-first-optional-http-gateway.md) |
| 5+ | LangFuse Cloud observability | **Should** — **active** — [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md), **LF-D01–LF-F01** |
| 5+ | `auto` mode, sidecar-only routing | Backlog — Later |

**Status:** See [V1_0.md](V1_0.md) **RC-*** (canonical). Streaming/agent **RC-01–RC-10** **Met**; observability **RC-LF1** open (v1.0 **not Met**). Broker **`fs.write`**, **`exec.shell`**, and **`BrokerInference`** are shipped ([AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md)).

## Later optional tracks

- Multi-plugin fleets, Wasm, VM envelope (server/fleet).
- Observation export, exotic ML codecs in sidecar.
- Full MCP catalog in guest.

## What stays built-in

- Socket + stream lifecycle for `rex.v1`.
- Sidecar supervisor + broker.
- `InferenceRuntime` as **broker implementation** ([ADR 0002](architecture/decisions/0002-inference-adapter-contract.md)).

## Success metrics

- Terminal stream invariant preserved for clients.
- Sidecar outage: clear client error on product path; daemon stays up.
- At least one successful brokered `fs.read` in MVP acceptance.

## Related

- [MVP_SPEC.md](MVP_SPEC.md) · [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md) · [ARCHITECTURE.md](ARCHITECTURE.md) · [ADAPTERS.md](ADAPTERS.md) · [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md) · [AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md) · [POLICY_ENGINE.md](POLICY_ENGINE.md)
