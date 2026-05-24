# Inference adapters

Pluggable backends behind **`InferenceRuntime`** (daemon `adapters` + `http_openai_compat` modules). The daemon remains **transport- and stream-authoritative**. Adapters emit **chunks** ending in a single **`done`** semantics or map failures to observable terminal errors ([MVP_SPEC.md](MVP_SPEC.md), NDJSON facade).

Product stance: **`rex-daemon` owns economics, policy, and brokering** ([ADR 0001](architecture/decisions/0001-daemon-owns-agent-orchestration-and-economics.md)); the **agent loop** runs in a **sidecar** ([MVP_SPEC.md](MVP_SPEC.md)). Adapters are **broker mechanisms** — they produce completion streams when the daemon fulfills a sidecar inference request (or in harness-only direct paths).

## Purpose

- **MVP broker backend:** OpenAI-compatible **HTTP** chat/completions (`http_openai_compat`).
- **Harness / legacy:** in-process **mock** and optional **Cursor CLI** subprocess — CI and migration; **not** MVP product acceptance without sidecar.
- Trace optimization levers: [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) · [CACHING.md](CACHING.md).

## `InferenceRequest` (design contract)

| Field | Purpose |
|---|---|
| `prompt` | User-visible task after optional pipeline rewriting. |
| `mode` | `ask`, `plan`, `agent` driving cacheability ([CACHING.md](CACHING.md)). |
| `model_hint` | Optional id from client; HTTP runtime uses env default when unset. |
| `trace_id` | Correlation across daemon, CLI, extension. |

**Invariant:** exactly **one terminal client-visible outcome** per `StreamInference` attempt.

## Streaming response shape

Chunks carry incremental `text`, monotonic `index`, terminating `done` chunk **or** gRPC/internal error surfaced as terminal **`error`** on the NDJSON CLI path.

## HTTP OpenAI-compat profile (broker)

| Aspect | Policy |
|---|---|
| Runtime id | `http-openai-compat` (`REX_INFERENCE_RUNTIME`) |
| Endpoint | `POST {base}/chat/completions` with `stream: true` (SSE) |
| Configuration | [CONFIGURATION.md](CONFIGURATION.md) — `REX_OPENAI_COMPAT_*` |
| Context injection | **On** — daemon `ContextPipeline` may shape prompt before HTTP call |
| Cacheable modes | **`ask`** only (same as mock; **`agent`** never cached) |
| Timeouts | `REX_OPENAI_COMPAT_TIMEOUT_SECS` (default 120s) |

### Operator profiles (examples)

| Backend | Typical `REX_OPENAI_COMPAT_BASE_URL` |
|---------|--------------------------------------|
| Ollama (local) | `http://127.0.0.1:11434/v1` |
| LM Studio | `http://127.0.0.1:1234/v1` |
| OpenAI API | `https://api.openai.com/v1` (+ `REX_OPENAI_COMPAT_API_KEY`) |

### Verification

- Local: configure env, start daemon, `rex-cli complete "hello" --format ndjson`.
- Automated: `http_openai_compat` unit test with in-process TCP SSE stub; UDS e2e uses **`mock`** — [CI.md](CI.md).

## Mock profile (test harness)

| Aspect | Policy |
|---|---|
| Runtime id | `mock` |
| Role | Deterministic chunks for CI, UDS e2e, extension fixtures — **not** the MVP product backend |
| Output | `mock: {prompt}` style text via `domain::build_mock_output` |

## Cursor CLI subprocess profile (legacy / non-MVP)

Optional subprocess via `REX_INFERENCE_RUNTIME=cursor-cli`. Not the REX agent product boundary. See [CONFIGURATION.md](CONFIGURATION.md) for `REX_CURSOR_CLI_*`.

CI exercises this path with a **`printf` stub** in `uds_e2e.rs`, not the real `cursor-agent` binary.

## `AdapterCapabilities` (implemented)

Rust struct in `crates/rex-daemon/src/adapters.rs`; passed into `ContextPipeline::prepare`.

| Field | HTTP / mock | Cursor CLI |
|---|---|---|
| `attach_context` | `true` — lexical `[context]` when indexer hits | `false` — no `[context]` suffix |
| `truncate_prompt` | `true` — token budget on user prompt | `false` — full prompt to subprocess |

| Capability (planned) | Meaning |
|---|---|
| `cacheable_modes` | Subset permitted for L1 (**`ask`** only today). |
| `max_prompt_tokens` / `max_context_tokens` | Optional per-adapter clamps beyond pipeline defaults. |
| `default_timeout` | Adapter-specific watchdog. |
| `supported_modes` | Early rejection of unsupported mode strings. |

## In-process adapter → optional gRPC drop-in

Same contract may run out-of-process per [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md) — **`rex.v1` clients unchanged**.

## New adapter checklist

1. Declare capabilities + safe cache modes ([ADR 0003](architecture/decisions/0003-layered-cache-agent-mode-policy.md)).
2. Guarantee single terminal semantics.
3. Document env catalog in [CONFIGURATION.md](CONFIGURATION.md).
4. Add profile subsection here.
5. Update [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md) if placement shifts.

## Local MLX path (planned)

**Status:** `planned` — not shipped. Roadmap: [ROADMAP.md](ROADMAP.md) (**Could**, Later horizon).

### Purpose

Optional **Apple MLX** (or similar local runtime) as an `InferenceRuntime` broker backend for on-device inference, increasing local-first leverage without embedding ML stacks in the daemon core.

### Scope

| In (design stage) | Out (design stage) |
|---|---|
| Adapter profile: capabilities, env catalog, streaming terminal semantics | Replacing sidecar agent loop ([MVP_SPEC.md](MVP_SPEC.md)) |
| Broker-only invocation from daemon (same as HTTP OpenAI-compat) | Default Mac product path (HTTP broker remains primary) |
| CI harness with mock/stub, not live MLX on every PR | |

### Boundaries

- **Mechanism:** new adapter implementing `InferenceRuntime` ([ADR 0002](architecture/decisions/0002-inference-adapter-contract.md)).
- **Policy:** routing, cache, and mode gates stay in daemon ([ADR 0001](architecture/decisions/0001-daemon-owns-agent-orchestration-and-economics.md)).
- Sidecar may host exotic ML codecs per [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md); MLX adapter is **in-daemon broker** unless a future ADR moves it.

### Interfaces (intent)

- Runtime id (for example `mlx`) on `REX_INFERENCE_RUNTIME`.
- Env catalog TBD (model path, device) — documented in [CONFIGURATION.md](CONFIGURATION.md) when scheduled.
- `AdapterCapabilities`: full context pipeline (budget, indexer, compressor) per [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md).

### Cross-links

- [ROADMAP.md](ROADMAP.md) — **Could** row
- [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) — economics matrix row
- [MVP_SPEC.md](MVP_SPEC.md) — explicit v1.0 non-promise

## Related

- [ARCHITECTURE.md](ARCHITECTURE.md)
- [MVP_SPEC.md](MVP_SPEC.md)
- [CACHING.md](CACHING.md)
- [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md)
