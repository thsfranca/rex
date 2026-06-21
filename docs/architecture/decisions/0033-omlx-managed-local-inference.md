# ADR 0033: oMLX managed local inference

- **Date:** 2026-06-17
- **Status:** Accepted (implemented)

## Context

Rex is a **Mac-first** local AI runtime ([PURPOSE_AND_PRINCIPLES.md](../../PURPOSE_AND_PRINCIPLES.md)) whose product path is a **development agent** with **long-context** tool loops ([AGENT_GRAPH_ARCHITECTURE.md](../../AGENT_GRAPH_ARCHITECTURE.md)). Inference is **broker-only** from the sidecar through `http_openai_compat` ([ADR 0018](0018-gateway-first-multi-provider-inference.md), [ADR 0002](0002-inference-adapter-contract.md)).

Today:

- **Cloud multi-provider** uses an opt-in **LiteLLM gateway** ([ADR 0019](0019-inference-gateway-opt-in-litellm.md)).
- **Local Ollama** is the documented direct path with Ollama-specific `/api/show` probing for `native_tools: auto` ([NATIVE_TOOL_CALLING.md](../../NATIVE_TOOL_CALLING.md)).
- A vague **in-daemon MLX** adapter is planned ([ADAPTERS.md](../../ADAPTERS.md#in-daemon-mlx-path-deferred)) — different from an external HTTP server.

**[oMLX](https://github.com/jundot/omlx)** is an Apple Silicon inference server (OpenAI-compat HTTP plus optional Anthropic API upstream) with tiered KV caching and continuous batching — a strong fit for long-prefix development-agent workloads. Rex has no design for it.

Operators need a **first-class Mac local MLX path** without multiplying HTTP adapters or breaking broker invariants. Rex must stay **easy to extend**: one broker API (OpenAI Chat Completions), with managed oMLX as a **lifecycle helper** that injects `inference.openai_compat.base_url` — the same pattern as the LiteLLM gateway ([ADR 0019](0019-inference-gateway-opt-in-litellm.md)).

## Decision

1. **Wire:** Use **OpenAI Chat Completions only** for oMLX integration — existing `http_openai_compat` adapter ([ADR 0018](0018-gateway-first-multi-provider-inference.md)). **No** dedicated `omlx` `InferenceRuntime` id. Rex never calls oMLX `/v1/messages` or any Anthropic-shaped Rex runtime for oMLX.
2. **Placement:** **Daemon-supervised oMLX child process** when `inference.omlx.mode: managed` — same invariant as LiteLLM gateway ([ADR 0019](0019-inference-gateway-opt-in-litellm.md)): **not** a sidecar plugin, **not** embedded in `rex-daemon`.
3. **Mac local primary:** Document **`managed`** as the **primary Mac operator profile** for local MLX inference. Modes: `managed` \| `external` \| `disabled`.
4. **Effective URL injection:** When `inference.omlx.mode: managed`, daemon injects `inference.openai_compat.base_url` → `http://127.0.0.1:{omlx.port}/v1` (default port **8000**). Canonical resolver table: [OMLX_INFERENCE.md](../../OMLX_INFERENCE.md#effective-openai_compatbase_url).
5. **Mutual exclusion:** Config validation **rejects** `inference.omlx.mode: managed` and `inference.gateway.mode: managed` together. At most one managed URL injector is active — avoids competing precedence tiers.
6. **oMLX multi-API is upstream:** oMLX may expose Anthropic Messages, embeddings, rerank, and LM Studio–style model dirs — **operator/oMLX configuration only**, not Rex adapter surfaces. Rex **OpenAI Chat Completions only** toward oMLX.
7. **In-daemon MLX FFI on Mac:** **Deferred** — oMLX HTTP server is the preferred Mac local path; in-daemon MLX remains a **Could** alternative for non-HTTP MLX ([ADAPTERS.md](../../ADAPTERS.md#in-daemon-mlx-path-deferred)).
8. **Reject:** `omlx` sidecar; MLX embed in `rex-daemon`; mandatory oMLX on every install; routing agent tools through gateway **and** oMLX (prefer direct managed oMLX for long-context agent); `inference.provider_profile` as a broker API selector for oMLX.

## Amendment (2026-06-17)

Supersedes original decision bullets **#4–#5** (4-tier precedence where oMLX outranked gateway; mandatory provider-profile registry for oMLX):

- **Precedence:** Replaced by mutual exclusion plus unified `openai_compat.base_url` resolver — see hub.
- **Provider profiles:** Not part of the oMLX broker contract. [fixtures/guidelines/inference_provider_profiles.yaml](../../../fixtures/guidelines/inference_provider_profiles.yaml) remains an optional operator cheat sheet only; no Rust profile registry in oMLX implementation slices.

## Alternatives considered

| Approach | Outcome |
|----------|---------|
| Dedicated `omlx` `InferenceRuntime` | **Rejected** — identical OpenAI wire; duplicates broker dispatch ([ADR 0018](0018-gateway-first-multi-provider-inference.md)) |
| oMLX as sidecar | **Rejected** — [ADR 0017](0017-single-active-sidecar-phase-1.md), wrong protocol |
| Embed MLX in `rex-daemon` | **Rejected** — process/language boundary; same as gateway rejection |
| External-only (operator runs menu bar app) | **Kept** as `omlx.mode: external` |
| oMLX Anthropic wire via Rex | **Rejected** — Rex OpenAI-only toward oMLX; Anthropic is oMLX-internal |
| LiteLLM-only local MLX | **Rejected** as primary — extra hop; poor fit for long-context agent tool loops |
| Provider profile registry as broker selector | **Rejected** — parallel inference surface; `openai_compat` is the single broker config |
| Managed oMLX + OpenAI-compat injection (chosen) | **Accepted** |

## Consequences

- **Positive:** Mac operators get a documented long-context local path aligned with Rex prefix immutability and native tools.
- **Positive:** One HTTP adapter (`http_openai_compat`); managed oMLX mirrors managed gateway — lifecycle config injects `openai_compat.base_url` only.
- **Positive:** Clear separation: **oMLX** = Mac local MLX lifecycle; **gateway** = multi-provider cloud lifecycle.
- **Negative:** Second supervised child process pattern (parallel to gateway) — implementation must share supervisor seams carefully.
- **Negative:** Python/oMLX dependency when `managed` is enabled on Mac.
- **Risks / follow-up:** Live E2E requires operator oMLX + MLX model install; not in PR CI (**RC-10**). In-daemon MLX remains deferred — [ADAPTERS.md](../../ADAPTERS.md#in-daemon-mlx-path-deferred).

## Related

- [OMLX_INFERENCE.md](../../OMLX_INFERENCE.md) — feature hub
- [INFERENCE_GATEWAY.md](../../INFERENCE_GATEWAY.md) — cloud gateway (orthogonal managed child)
- [NATIVE_TOOL_CALLING.md](../../NATIVE_TOOL_CALLING.md), [ADR 0023](0023-hybrid-agent-serialization-boundaries.md)
- [ADR 0002](0002-inference-adapter-contract.md), [ADR 0018](0018-gateway-first-multi-provider-inference.md), [ADR 0019](0019-inference-gateway-opt-in-litellm.md)
