# ADR 0019: Inference Gateway — opt-in LiteLLM, daemon-controlled

- **Date:** 2026-05-24
- **Status:** Accepted (design)
- **Supersedes (partial):** [0018](0018-gateway-first-multi-provider-inference.md) — default API and built-in gateway capability wording only

## Context

[ADR 0018](0018-gateway-first-multi-provider-inference.md) established LiteLLM as the **recommended** multi-provider path via `http_openai_compat`, with provider keys on the gateway host. Operators still run LiteLLM manually, wire `base_url`, and may confuse the **agent sidecar** slot with hosting LiteLLM.

Rex needs:

- **LiteLLM as the default inference API** story (OpenAI-compat toward a gateway).
- **Opt-in built-in** LiteLLM server Rex **controls** (spawn, health, stop) when `inference.gateway.mode: managed`.
- **Unified local + cloud model list** including Ollama models discovered through LiteLLM.
- **Optional native Anthropic** Messages runtime later — secondary to the gateway path.

Sidecars cannot host LiteLLM: wrong protocol (`rex.sidecar.v1` vs HTTP), wrong semantics (`RunTurn`), [ADR 0017](0017-single-active-sidecar-phase-1.md) single slot, and broker rules against sidecar-held cloud keys.

## Decision

1. **Default API:** Document and configure multi-provider HTTP inference toward **LiteLLM-shaped** OpenAI-compat; keep a single daemon adapter: `http_openai_compat` ([ADR 0002](0002-inference-adapter-contract.md)).
2. **Inference Gateway feature:** Add daemon-supervised **gateway child process** with config `inference.gateway.*` — **not** a sidecar plugin. Canonical hub: [INFERENCE_GATEWAY.md](../../INFERENCE_GATEWAY.md).
3. **Opt-in:** `inference.gateway.mode: disabled | external | managed`. Default remains `disabled`; CI stays `mock` without spawning Python.
4. **Rex control when `managed`:** Daemon spawns/stops LiteLLM, health-gates readiness, injects loopback `openai_compat.base_url` unless override allowed.
5. **Ollama discovery:** Default gateway template uses LiteLLM `ollama/*` + `check_provider_endpoint: true` so `GET /v1/models` lists local Ollama models; gateway startup does **not** fail if Ollama is offline.
6. **Reject:** Embedding LiteLLM in the `rex-daemon` process; hosting gateway as `rex.sidecar.v1`; mandatory LiteLLM on every install; dedicated `litellm` runtime id.
7. **Optional Anthropic:** Planned `anthropic` `InferenceRuntime` remains **secondary** for operators who bypass the gateway ([ADR 0018](0018-gateway-first-multi-provider-inference.md) secondary B).

## Alternatives considered

| Approach | Outcome |
|----------|---------|
| LiteLLM as sidecar | **Rejected** — slot, protocol, semantics |
| Embed in `rex-daemon` | **Rejected** — process/language boundary |
| External-only gateway (status quo) | **Kept** as `gateway.mode: external` |
| Rex polls Ollama and writes config | **Deferred** — prefer LiteLLM discovery |
| Auto-start managed on every daemon | **Rejected** — opt-in only |

## Consequences

- **Positive:** Lower toil for operators who opt in; clear separation from agent sidecar; one model catalog for cloud + Ollama when managed.
- **Positive:** ADR 0018 broker and adapter rules unchanged.
- **Negative:** Python/LiteLLM dependency when `managed` is enabled.
- **Negative:** Extra loopback hop vs direct Ollama URL.
- **Follow-up:** Implement gateway supervisor, config schema, templates under `$REX_ROOT/gateway/`, `rex doctor` readiness (implementation PRs).

## Related

- [INFERENCE_GATEWAY.md](../../INFERENCE_GATEWAY.md)
- [ADAPTERS.md](../../ADAPTERS.md)
- [CONFIGURATION.md](../../CONFIGURATION.md)
- [ADR 0004](0004-routing-daemon-first-optional-http-gateway.md)
- [ADR 0017](0017-single-active-sidecar-phase-1.md)
- [ADR 0018](0018-gateway-first-multi-provider-inference.md)
