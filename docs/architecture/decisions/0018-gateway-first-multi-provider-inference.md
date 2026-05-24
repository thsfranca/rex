# ADR 0018: Gateway-first multi-provider inference

- **Date:** 2026-05-24
- **Status:** Accepted (design)

## Context

Operators need Rex to reach **Anthropic**, **OpenAI**, and **local** models without multiplying daemon adapters or breaking **broker-only** sidecar inference ([ADR 0001](0001-daemon-owns-agent-orchestration-and-economics.md), [MVP_SPEC.md](../../MVP_SPEC.md)).

Today the product path is **sidecar → `BrokerInference` → `http_openai_compat`**. That adapter speaks **OpenAI Chat Completions** only. [ADR 0004](0004-routing-daemon-first-optional-http-gateway.md) allows an optional external OpenAI-compat gateway but rejects **mandating** one before the agent path is proven.

Teams often run **LiteLLM** (or similar) as a multi-provider OpenAI-compat proxy. Anthropic’s **native Messages API** is a different wire contract.

## Decision

1. **Primary (documented operator path):** Point `inference.openai_compat` / `REX_OPENAI_COMPAT_*` at a **LiteLLM** (or equivalent) OpenAI-compat gateway. Provider choice and API keys live on the **gateway**; Rex sends `model` and prompt on the compat wire.
2. **Secondary A:** Same `http_openai_compat` adapter with **direct** OpenAI-compat URLs (OpenAI API, Ollama, LM Studio).
3. **Secondary B (planned implementation):** Native **`anthropic`** `InferenceRuntime` for the **Messages API** when gateway hop or compat translation is insufficient.
4. **Naming:** Keep `openai_compat` identifiers as **protocol** names in docs and config until a separate migration ADR adds neutral aliases (`http_chat_completions`, etc.).
5. **Adapter granularity:** One runtime per **HTTP contract**, not per vendor brand ([ADAPTERS.md](../../ADAPTERS.md)).

## Alternatives considered

| Approach | Outcome |
|----------|---------|
| **One adapter per vendor** (`openai`, `anthropic`, `litellm`, …) | **Rejected** for identical wire — duplicates HTTP/SSE, broker dispatch, and CI; LiteLLM is not a vendor protocol |
| **One adapter per `base_url`** | **Rejected** — unbounded `RuntimeKind` values; poor fit for daemon policy |
| **Mandatory LiteLLM in every install** | **Rejected** — contradicts local-first default and ADR 0004 |
| **Sidecar-held Anthropic keys** | **Rejected** — violates broker invariant ([AGENT_DELIVERY_ROADMAP.md](../../AGENT_DELIVERY_ROADMAP.md)) |
| **Protocol adapter + optional native fork (chosen)** | **Accepted** — matches shipped `http_openai_compat` and planned `anthropic` |

## Consequences

- **Positive:** Anthropic and OpenAI reachable **without** `rex.v1` or sidecar changes on the primary path; single SSE parser and capability matrix for all OpenAI-shaped backends.
- **Positive:** Centralized keys and metering on the gateway when operators want them.
- **Negative:** Extra network hop and operational dependency when using LiteLLM.
- **Negative:** Vendor-specific features (Anthropic prompt cache, tool schemas) may require native adapter or gateway passthrough — not assumed on compat path.
- **Follow-up:** Implement `anthropic` runtime + broker dispatch; broker `provider_*` error codes in [ERROR_HANDLING.md](../../ERROR_HANDLING.md) when that hub exists; optional neutral rename of `openai_compat` config with aliases.

## Related

- [ADAPTERS.md](../../ADAPTERS.md) — operator profiles and error intent
- [CONFIGURATION.md](../../CONFIGURATION.md) — LiteLLM operator profile
- [ADR 0002](0002-inference-adapter-contract.md) — `InferenceRuntime` seam
- [ADR 0004](0004-routing-daemon-first-optional-http-gateway.md) — optional gateway stance
