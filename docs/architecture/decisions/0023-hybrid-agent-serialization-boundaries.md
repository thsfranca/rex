# ADR 0023: Hybrid agent serialization at boundaries

- **Date:** 2026-06-04
- **Status:** Accepted

## Context

Rex assembles agent context across **three boundaries** (extension user text → daemon `effective_prompt` → sidecar `messages_to_prompt()` per LLM step). In a 5–12 step ReAct loop, **per-step re-transmission** of a static prefix dominates token spend when provider prefix caching is absent (`O(N²)` compounding).

**R018** ships one-line JSON `{"type":"tool"|"final",...}` inside a single broker `user` message. Research on compact notations (TOON, TRON), YAML-for-generation, CBOR, and natural-language tool pipelines does **not** justify replacing Rex’s interim protocol before gateway-normalized native tools land (**R033**).

Daemon retains economics authority ([ADR 0001](0001-daemon-owns-agent-orchestration-and-economics.md)); sidecar remains broker-only ([ADR 0008](0008-dedicated-sidecar-control-plane-api.md)).

## Decision

Adopt a **hybrid serialization policy** per boundary:

| Boundary | Format | Phase | Milestone |
|----------|--------|-------|-----------|
| **Generative outputs** (LLM → tools) | Provider-native function calling + **structured outputs** (`strict: true`) | Phase 2 | **R033** |
| **Interim generative** (until R033) | One-line JSON `{"type":"tool"\|"final",...}` in text | Phase 1 | Current (**R018**) |
| **Input tool schemas** (static prefix) | **TRON**-class batching for repeated tool shapes | Phase 1 optional | **R036** (Could) |
| **Tool result payloads** (daemon → sidecar → prompt) | **Raw delimited text** (`<<TOOL_RESULT:tool>>` … `<<END>>`); line-safe truncation at `max_tool_result_bytes` | Phase 1 | **R034** (Should) |
| **Write payloads** (model → sidecar) | **Line-oriented delimiters** for multi-line code; metadata outside JSON string | Phase 1 | Extends **R030** |
| **Rejected on LLM wire** | TOON/YAML for generation; CBOR/MessagePack; natural-language tool multi-stage pipelines | — | Anti-patterns in [AGENT_GRAPH_ARCHITECTURE.md](../../AGENT_GRAPH_ARCHITECTURE.md) |

**Phase 1 trigger:** keep JSON-in-text + raw delimited tool results until LiteLLM/gateway normalization for cross-provider `strict: true` is documented in [INFERENCE_GATEWAY.md](../../INFERENCE_GATEWAY.md); then **R033** deprecates in-text tool JSON.

**Prefix economics:** static `[system]` + `daemon_context` bytes must be **immutable** across tool steps within a turn (SHA-256 contract — **R027** / **R032**); see [CACHING.md](../../CACHING.md) and [AGENT_GRAPH_ARCHITECTURE.md](../../AGENT_GRAPH_ARCHITECTURE.md).

## Rejected options

| Option | Why rejected for Rex |
|--------|----------------------|
| **TOON** / tabular compact notation for **generation** | Optimized for static schema display; poor fit for variable tool args and provider-native strict JSON; migration cost across extension, daemon, sidecar, NDJSON stream |
| **YAML for generation** | Higher parse failure rate in LLM output; breaks one-line NDJSON streaming contract; no broker path today |
| **CBOR / MessagePack on LLM wire** | Opaque to providers; no tokenization benefit on HTTP text APIs; breaks grep/debug and extension passthrough |
| **Natural Language Tools (NLT)** | Multi-stage selection pipeline adds latency and non-determinism; conflicts with daemon policy broker and single-tool-per-step contract |

TRON-style **static schema compression** in the daemon-assembled prefix remains **optional** (**R036**, Could) — input-only, not generative wire format.

## Consequences

- **Positive:** Single accepted table for format debates; Phase 1 work (prefix lock, raw results, microcompaction) can proceed without re-litigating TOON/YAML; validation metrics (`cached_tokens`, `prefix_hash`, `parse_retries`) have clear owners in [ECONOMICS_VALIDATION.md](../../ECONOMICS_VALIDATION.md).
- **Negative:** Interim JSON-in-text retains up to **3** parse retries until **R033**; delimiter contracts require sidecar/daemon coordination for truncation boundaries.
- **Risks / follow-up:** Cache header owner (gateway vs native adapter) open — [INFERENCE_GATEWAY.md](../../INFERENCE_GATEWAY.md), [ADAPTERS.md](../../ADAPTERS.md); linter sandbox during tool loop — [AGENT_ACCESS_POLICY.md](../../AGENT_ACCESS_POLICY.md); NDJSON parser version if raw blocks change extension framing — [EXTENSION.md](../../EXTENSION.md).

## Related

- [AGENT_GRAPH_ARCHITECTURE.md](../../AGENT_GRAPH_ARCHITECTURE.md) — token playbook, anti-patterns, microcompaction
- [ADR 0022](0022-viewer-editor-subagent-topology.md) — Viewer/Editor topology
- [ADR 0016](0016-mcp-in-sidecar-envelope.md) — MCP Phase 2 with **R033**
- [CONTEXT_EFFICIENCY.md](../../CONTEXT_EFFICIENCY.md) — economics lever matrix
- [docs/research/GEMINI_DEEP_RESEARCH_PROMPT.md](../../research/GEMINI_DEEP_RESEARCH_PROMPT.md) — research prompt; findings incorporated in hub + this ADR
