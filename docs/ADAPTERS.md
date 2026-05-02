# Inference adapters

Pluggable backends behind **`InferenceRuntime`** (`crates/rex-daemon/src/adapters.rs`). The daemon remains **transport- and stream-authoritative**. Adapters emit **chunks** ending in a single **`done`** semantics or map failures to observable terminal errors (`MVP_SPEC.md`, NDJSON facade).

Product stance: **`rex-daemon` owns agent/economics policy** over time ([ADR 0001](architecture/decisions/0001-daemon-owns-agent-orchestration-and-economics.md)); adapters produce **completion streams** subject to **`AdapterCapabilities`**. An external CLI may **loop internally**; REX still presents **one contract** to clients and controls **caching, pipeline stages, and adapter selection**.

## Purpose

- Stay vendor-agnostic (mock, MLX future, HTTP future, **optional** Cursor subprocess).
- Avoid applying pipeline stages that **break** or **duplicate** a backend’s expectations.
- Trace optimization levers: [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) matrix · [CACHING.md](CACHING.md).

## `InferenceRequest` (design contract)

| Field | Purpose |
|---|---|
| `prompt` | User-visible task after optional pipeline rewriting. |
| `mode` | `ask`, `plan`, `agent` (+ future) driving cacheability (`CACHING.md`). |
| `model_hint` | Optional id / **`auto`** when supported. |
| `trace_id` | Correlation across daemon + CLI + extension. |
| `metadata` | Opaque adapter hints — keep `rex.v1` vendor-neutral. |

**Invariant:** exactly **one terminal client-visible outcome** per `StreamInference` attempt.

## Streaming response shape

Chunks carry incremental `text`, monotonic `index`, terminating `done` chunk **or** gRPC/internal error surfaced as terminal **`error`** on NDJSON CLI path.

## `AdapterCapabilities`

| Capability | Meaning |
|---|---|
| `wants_context_injection` | If **false**, skip heavy lexical injection (Cursor subprocess profile historically **false** for `[context]`). |
| `cacheable_modes` | Subset permitted for response cache (**never unchecked `agent` writes**). |
| `max_prompt_tokens` / `max_context_tokens` | Optional pipeline clamp; `0` → daemon defaults. |
| `default_timeout` | Subprocess watchdog budget. |
| `supported_modes` | Adapter refuses unsupported combos early + clearly. |

Pipeline consultation: **`ContextPipeline`** + future router — [`CONTEXT_EFFICIENCY.md`](CONTEXT_EFFICIENCY.md).

## Cursor CLI subprocess profile (**optional backend**)

Spawns **`cursor-agent`** (or templated **`REX_CURSOR_CLI_COMMAND`**) non-interactively with typed stdout (project-specific flags evolve — consult current Cursor CLI docs).

| Aspect | Policy |
|---|---|
| Product role | **Account-bound frontier access** helper — **not** the definition of the REX agent story. |
| Context stacking | Typically **skip** injecting an extra heavy `[context]` lexical blob on top — adapter may leverage its own tooling. |
| Invocations | **Stateless boundary** preferred; pass fresh prompt each **`StreamInference`** unless explicit session bridging appears later. |
| Safety | Mandatory **timeouts / kill**; stderr surfaced bounded — [CONFIGURATION.md](CONFIGURATION.md). |

### Local verification

- CI / default automation: **`mock`** only; UDS harness runs **`cursor-cli`** path via **`printf` stub** (`uds_e2e.rs`).
- Real binary: configure env vars, **`rex-cli complete "hello" --format ndjson`**, watch terminal + daemon logs.

## In-process adapter → optional gRPC drop-in

Same capability + streaming contract implemented out-of-process per [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md) lifecycle — **clients unchanged**.

## New adapter checklist

1. Declare capabilities + safe cache modes (**`agent` caution** — [ADR 0003](architecture/decisions/0003-layered-cache-agent-mode-policy.md)).
2. Guarantee single terminal semantics (no duplicate `done`).
3. Subprocess adapters: cwd workspace, SIGKILL/timeout hygiene.
4. Key caches + metrics via **adapter id** + resolved **model** id.
5. Document profile subsection here.
6. Update `PLUGIN_ROADMAP.md` if lifecycle / phase shifts.

## Related

- [ARCHITECTURE.md](ARCHITECTURE.md)
- [CACHING.md](CACHING.md)
- [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md)
