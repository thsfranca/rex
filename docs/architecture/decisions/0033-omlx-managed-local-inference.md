# ADR 0033: oMLX managed local inference

- **Date:** 2026-06-17
- **Status:** Accepted (design)

## Context

Rex is a **Mac-first** local AI runtime ([PURPOSE_AND_PRINCIPLES.md](../../PURPOSE_AND_PRINCIPLES.md)) whose product path is a **development agent** with **long-context** tool loops ([AGENT_GRAPH_ARCHITECTURE.md](../../AGENT_GRAPH_ARCHITECTURE.md)). Inference is **broker-only** from the sidecar through `http_openai_compat` ([ADR 0018](0018-gateway-first-multi-provider-inference.md), [ADR 0002](0002-inference-adapter-contract.md)).

Today:

- **Cloud multi-provider** uses an opt-in **LiteLLM gateway** ([ADR 0019](0019-inference-gateway-opt-in-litellm.md)).
- **Local Ollama** is the documented direct path with Ollama-specific `/api/show` probing for `native_tools: auto` ([NATIVE_TOOL_CALLING.md](../../NATIVE_TOOL_CALLING.md)).
- A vague **in-daemon MLX** adapter is planned ([ADAPTERS.md](../../ADAPTERS.md#in-daemon-mlx-path-deferred)) — different from an external HTTP server.

**[oMLX](https://github.com/jundot/omlx)** is an Apple Silicon inference server (OpenAI-compat HTTP + optional Anthropic API) with tiered KV caching and continuous batching — a strong fit for long-prefix development-agent workloads. Rex has no design for it.

Operators need a **first-class Mac local MLX path** without multiplying HTTP adapters or breaking broker invariants. Rex must stay **easy to extend**: one popular API (OpenAI Chat Completions), declarative **provider profiles** instead of per-vendor runtime enums.

## Decision

1. **Wire:** Use **OpenAI Chat Completions only** for oMLX integration — existing `http_openai_compat` adapter ([ADR 0018](0018-gateway-first-multi-provider-inference.md)). **No** dedicated `omlx` `InferenceRuntime` id.
2. **Placement:** **Daemon-supervised oMLX child process** when `inference.omlx.mode: managed` — same invariant as LiteLLM gateway ([ADR 0019](0019-inference-gateway-opt-in-litellm.md)): **not** a sidecar plugin, **not** embedded in `rex-daemon`.
3. **Mac local primary:** Document **`managed`** as the **primary Mac operator profile** for local MLX inference. Modes: `managed` \| `external` \| `disabled`.
4. **Effective URL precedence:** (1) explicit `openai_compat.base_url` override → (2) oMLX managed → (3) gateway managed → (4) configured base URL or broker error. Canonical table: [OMLX_INFERENCE.md](../../OMLX_INFERENCE.md#effective-base_url-precedence).
5. **Provider profiles:** Introduce declarative **inference provider profiles** ([fixtures/guidelines/inference_provider_profiles.yaml](../../../fixtures/guidelines/inference_provider_profiles.yaml)) so `omlx`, `ollama`, and future OpenAI-compat servers share one adapter with preset defaults (base URL, native tools, probes) — replacing ad-hoc `is_ollama_like_base_url` as the long-term pattern.
6. **oMLX Anthropic `/v1/messages`:** **Out of scope** — requires planned `anthropic` runtime; separate track.
7. **In-daemon MLX FFI on Mac:** **Deferred** — oMLX HTTP server is the preferred Mac local path; in-daemon MLX remains a **Could** alternative for non-HTTP MLX ([ADAPTERS.md](../../ADAPTERS.md#in-daemon-mlx-path-deferred)).
8. **Reject:** `omlx` sidecar; MLX embed in `rex-daemon`; mandatory oMLX on every install; routing agent tools through gateway **and** oMLX (prefer direct managed oMLX for long-context agent).

## Alternatives considered

| Approach | Outcome |
|----------|---------|
| Dedicated `omlx` `InferenceRuntime` | **Rejected** — identical OpenAI wire; duplicates broker dispatch ([ADR 0018](0018-gateway-first-multi-provider-inference.md)) |
| oMLX as sidecar | **Rejected** — [ADR 0017](0017-single-active-sidecar-phase-1.md), wrong protocol |
| Embed MLX in `rex-daemon` | **Rejected** — process/language boundary; same as gateway rejection |
| External-only (operator runs menu bar app) | **Kept** as `omlx.mode: external` |
| oMLX Anthropic wire via Rex | **Deferred** — needs `anthropic` runtime |
| LiteLLM-only local MLX | **Rejected** as primary — extra hop; poor fit for long-context agent tool loops |
| Managed oMLX primary + OpenAI-compat (chosen) | **Accepted** |

## Consequences

- **Positive:** Mac operators get a documented long-context local path aligned with Rex prefix immutability and native tools.
- **Positive:** One HTTP adapter + profiles scales to LM Studio, Jan, and other OpenAI-compat servers without new `RuntimeKind` values.
- **Positive:** Clear separation: **oMLX** = Mac local MLX; **gateway** = multi-provider cloud.
- **Negative:** Second supervised child process pattern (parallel to gateway) — implementation must share supervisor seams carefully.
- **Negative:** Python/oMLX dependency when `managed` is enabled on Mac.
- **Risks / follow-up:** Implement config schema, omlx supervisor, `rex omlx init|doctor`, provider profile registry, opt-in live E2E — see [OMLX_INFERENCE.md](../../OMLX_INFERENCE.md#implementation-slices-planning-only).

## Related

- [OMLX_INFERENCE.md](../../OMLX_INFERENCE.md) — feature hub
- [INFERENCE_GATEWAY.md](../../INFERENCE_GATEWAY.md) — cloud gateway (orthogonal)
- [NATIVE_TOOL_CALLING.md](../../NATIVE_TOOL_CALLING.md), [ADR 0023](0023-hybrid-agent-serialization-boundaries.md)
- [ADR 0002](0002-inference-adapter-contract.md), [ADR 0018](0018-gateway-first-multi-provider-inference.md), [ADR 0019](0019-inference-gateway-opt-in-litellm.md)
