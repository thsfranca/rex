# Plugin and extensibility roadmap

REX grows **in-process inference adapters** first, keeps **routing/caching/pipeline policy in `rex-daemon`**, and treats **optional isolated runtimes** (historically “gRPC sidecars”) as **environment** for foreign stacks or fault isolation — not the default home for core economics. See [ADR 0001](architecture/decisions/0001-daemon-owns-agent-orchestration-and-economics.md), [ADR 0005](architecture/decisions/0005-rex-owns-sidecar-environment-not-agent-implementations.md). Concepts and transport options: [AGENT_RUNTIME_ENVIRONMENT.md](AGENT_RUNTIME_ENVIRONMENT.md).

## Current purpose

- Ship reliable **StreamInference** surfaces for extensions and CLI consumers.
- Concentrate **cost/performance levers** ([CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md)) in the daemon boundary.
- Offer **mock** everywhere; **Cursor CLI** as **one optional subprocess adapter** (`REX_INFERENCE_RUNTIME=cursor-cli`).
- Optionally supervise **foreign-runtime** workloads via sidecars **after** the single-process story is boring.

## Optional Cursor CLI adapter (enable)

- Env: **`REX_INFERENCE_RUNTIME=cursor-cli`** on `rex-daemon`.
- Overrides: **`REX_CURSOR_CLI_PATH`** (defaults `cursor-agent`), **`REX_CURSOR_CLI_COMMAND`** (`{prompt}` template), **`REX_CURSOR_CLI_TIMEOUT_SECS`** (default `20`).
- CI: keep **`mock`** default — [DEPENDENCIES.md](DEPENDENCIES.md), [CI.md](CI.md).

## Daemon-first principle

| Principle | Meaning |
|-----------|---------|
| Policy in-core | Routing decisions, caches, budgets, terminal stream semantics owned by **`rex-daemon`**. |
| Adapters dumb-ish | Produce **model-facing** output from an **effective** prompt under **capability** flags — [ADAPTERS.md](ADAPTERS.md). |
| Sidecars opt-in | Use when **process isolation**, **language ecosystem**, or **crash containment** wins over integration cost. |

## Placement decision gate (revised)

| Question | Prefer **in-daemon** | Prefer **sidecar** |
|---|---|---|
| Must every request honor it for economics/safety? | Yes | Rare exception |
| Does it need another language runtime or ML stack you do not want in the Rust binary? | No | Yes |
| Should a fault be contained without taking down the daemon? | No | Yes |
| Is it pure experimentation with uncertain retention? | No (prototype behind flag) | Sometimes |

**Default:** implement in **`rex-daemon`** until isolation evidence appears.

### Feature placement (versus earlier sidecar-first notes)

| Area | Placement | Notes |
|---|---|---|
| L1/L2/policy caches | **Daemon** — today L1 [`l1_cache.rs`](../crates/rex-daemon/src/l1_cache.rs) | L2 future — [CACHING.md](CACHING.md) |
| Request tracing tokens | **Daemon** stdout fields | correlate extension / CLI |
| Context shaping / compaction | **Daemon pipeline** hooks | isolate heavy ML in sidecar **if** ONNX/Python becomes mandatory |
| Hybrid routing cascades | **Daemon** router (planned) | optional HTTP backends still adapters — [ADR 0004](architecture/decisions/0004-routing-daemon-first-optional-http-gateway.md) |

## Cursor CLI adapter phases

| Phase | Outcome |
|---|---|---|
| 1 — Adapter seam | `InferenceRuntime`, metadata, **`AdapterCapabilities`**. Mock default. |
| 2 — Cursor spawn | Bounded subprocess + typed stdout → stream; terminal errors mapped. |
| 3 — L1 cache | **`ask`**-only exact cache; **`agent`** excluded — see [ADR 0003](architecture/decisions/0003-layered-cache-agent-mode-policy.md). |
| 4 — Proto / CLI knobs | **`model`** / **`mode`** fields already in proto; widen adapter use. |
| 5 — Model `auto` | Document semantics per adapter profile. |
| 6 — L2 semantic cache | **`ask`** only; strict guards. |

**Repo status:** Phases **1–3 partial** landed (adapter + L1 **`ask`** + proto fields exist). **`auto`**, semantic L2, sidecar-hosted adapter migration remain backlog.

Hosting the Cursor adapter behind a future **process boundary** duplicates the **`InferenceRuntime`** contract without altering `rex.v1` consumers.

## Optional sidecar platform (defer heavy investment)

Incremental slices if/when justified — **failure isolation** emphasis:

### Phase Sidecar lifecycle baseline

Daemon supervises **0 or 1** plugin process — health probes, timeouts, restart policy.

### Later optional tracks

Observation export, auxiliary cache/process for exotic ML codecs, experimentation-only context rankers — only if duplicates logic **not worth** compiling into core.

Defer: Wasm plugins, unmanaged multi-plugin sprawl absent operator demand.

## What stays built-in

- Socket + stream lifecycle correctness.
- `InferenceRuntime` registration + **`AdapterCapabilities`** gating ([ADR 0002](architecture/decisions/0002-inference-adapter-contract.md)).
- Scheduling/cancellation scaffolding.

## Success metrics

- Terminal stream invariant preserved (`done` XOR `error` equivalence at client).
- Repeated safe prompts measurable via **`l1_cache=hit`**.
- Sidecar outages (when used) downgrade gracefully without crashing daemon.

## Related

- [ARCHITECTURE.md](ARCHITECTURE.md) · [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) · [ADAPTERS.md](ADAPTERS.md) · [AGENT_RUNTIME_ENVIRONMENT.md](AGENT_RUNTIME_ENVIRONMENT.md) · [ADR 0005](architecture/decisions/0005-rex-owns-sidecar-environment-not-agent-implementations.md)
