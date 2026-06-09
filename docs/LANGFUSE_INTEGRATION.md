# LangFuse integration (design hub)

**Diátaxis role:** explanation — LangFuse Cloud as **primary** observability and evaluation platform for Rex.

**Status:** **design accepted** (docs pivot 2026-06); implementation **planned** pending [LANGFUSE_DISCOVERY_ROADMAP.md](LANGFUSE_DISCOVERY_ROADMAP.md).

**Supersedes (product direction):** Rex-owned observability store, bundled Grafana suite, CHCE mmap program — see cancelled hubs [CHCE_ROADMAP.md](CHCE_ROADMAP.md), [OBSERVABILITY_INTEGRATIONS.md](OBSERVABILITY_INTEGRATIONS.md). ADRs [0021](architecture/decisions/0021-rex-owned-economics-store-byot-visualization.md), [0025](architecture/decisions/0025-dual-economics-store-engines.md), [0026](architecture/decisions/0026-rex-owned-storage-grafana-otel-datasource.md), [0027](architecture/decisions/0027-chce-columnar-mmap-engine.md) remain **Accepted** in Git until **LF-D10** lands a supersession ADR.

## Purpose

Rex remains a **local runtime** (daemon, policy, broker, economics **execution**, editor contract). **LangFuse Cloud** is the **system of record** for traces, metrics dashboards, prompt management, datasets, experiments, and evals. Rex **exports** telemetry; it does not operate a parallel observability database or bundled Grafana on the Mac.

Aligns with [PURPOSE_AND_PRINCIPLES.md](PURPOSE_AND_PRINCIPLES.md): local inference and agent work stay on-device; observability UI and persistence move to LangFuse so **local RAM/CPU serve models**, not Postgres+ClickHouse+Redis+S3.

## Deployment stance

| Choice | Recommendation |
|--------|----------------|
| **LangFuse Cloud** | **Default** — EU, US, JP, or HIPAA regions per operator account |
| **Self-hosted LangFuse** | **Out of default scope** — competes with local model resources on Mac |
| **Secrets** | LangFuse public/secret keys via env or OS keychain — **not** committed JSON |
| **Offline** | Stdout economics grep when Cloud unreachable ([OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md) phase 0) |

## Prioritization

| Bucket | Rationale |
|--------|-----------|
| **Should** | Unblocks v1.0 observability without Rex DB program (**RC-LF1**); industry-standard LLM ops UX |
| **Rank** | After discovery **LF-D01–LF-D02**; implementation **LF-R01** (dead-code removal) recommended before **LF-F01** |

## Scope

**In:**

- Integration features **LF-F01–LF-F07** (implementation deferred until discovery closes)
- Daemon OTLP export shaped for LangFuse ingest ([ADR 0020](architecture/decisions/0020-otel-genai-semconv-with-rex-pipeline-metrics.md))
- Optional LiteLLM and LangGraph export paths
- Economics validation via LangFuse datasets ([ECONOMICS_VALIDATION.md](ECONOMICS_VALIDATION.md))

**Out:**

- Rex-owned `rex-obs-store`, read API, `rex obs up`, Grafana Rex OTel datasource (cancelled — removal **LF-R01**)
- CHCE mmap (**R043–R054** cancelled)
- `SidecarObservabilityService` ingest to Rex store (cancelled)
- Self-hosted LangFuse as product default
- LangFuse replacing daemon policy, broker, or `rex.v1` / NDJSON contract

## Boundaries

| Concern | Rex | LangFuse Cloud |
|---------|-----|----------------|
| Inference and stream lifecycle | `rex-daemon` | — |
| Access policy and approvals | daemon broker | — |
| Context pipeline, caches, routing | daemon | — |
| Trace persistence and UI | export only | system of record |
| Prompt CMS, playground | static-layer fetch (planned) | versioning, deploy labels |
| Datasets, experiments, LLM-as-judge | harness triggers | storage and UI |
| Editor / CLI contract | `rex.v1`, NDJSON | — |

## Integration features (LF-F01–LF-F07)

| ID | Feature | Owner | Discovery |
|----|---------|-------|-----------|
| **LF-F01** | Daemon OTLP export to LangFuse Cloud — economics fields as trace metadata / OTel metrics | `rex-daemon` | LF-D01, LF-D02 |
| **LF-F02** | LiteLLM `langfuse_otel` callback on gateway path | gateway config | LF-D03 |
| **LF-F03** | `rex-agent` LangGraph `CallbackHandler` (opt-in tracing) | sidecar | LF-D04 |
| **LF-F04** | Economics validation via LangFuse **datasets + dataset runs** | harness + Cloud | LF-D05 |
| **LF-F05** | Prompt management for **static** layers only; daemon `ContextPipeline` keeps workspace context | daemon/sidecar | LF-D06 |
| **LF-F06** | LLM-as-judge evaluators on sample/production traces | LangFuse project | LF-D07 |
| **LF-F07** | Human annotation queues for golden sets | LangFuse UI | LF-D08 |

## Attribute mapping (export reference)

Maps daemon economics at `stream.terminal` to LangFuse (refinement in **LF-D02**). Source shape: `StreamEconomicsRecord` in `crates/rex-daemon/src/economics_record.rs`.

| Rex field | LangFuse target |
|-----------|------------------|
| `trace_id` | Trace id |
| `turn_id` | `session_id` / session metadata |
| `snapshot_id` | Tag `config_snapshot_id` |
| `request_id` | Metadata `rex.request_id` |
| `terminal` | Tag or metadata `rex.terminal` |
| `route`, `cache_decision`, `decision_id` | Tags / metadata |
| `mode`, `model`, `inference_runtime` | Metadata |
| `elapsed_ms`, `chunks_sent` | Metrics or span duration |
| `prompt_tokens`, `context_tokens`, `context_candidates`, `context_selected` | Metrics (`gen_ai.*`, `rex.context.*`) |
| `context_truncated`, `retrieval`, `compression_strategy` | Metadata |
| `cached_tokens`, `prefix_hash`, `parse_retries` | Metadata or scores |

**PII default:** metadata and bounded tags only — no raw prompts or file bodies in export ([ECONOMICS_VALIDATION.md](ECONOMICS_VALIDATION.md) alignment).

## Implementation backlog

| ID | Theme | Blocked until |
|----|-------|---------------|
| **LF-R01** | Remove Rex-owned observability dead code | Docs pivot merged |
| **LF-F01** | Wire daemon OTLP → LangFuse Cloud | LF-D01, LF-D02; **LF-R01** recommended |

## Cross-links

- [LANGFUSE_DISCOVERY_ROADMAP.md](LANGFUSE_DISCOVERY_ROADMAP.md) — discovery tasks **LF-D01–LF-D10**
- [ROADMAP.md](ROADMAP.md) — queue mirror
- [V1_0.md](V1_0.md) — **RC-LF1** observability Must row
- [ECONOMICS_VALIDATION.md](ECONOMICS_VALIDATION.md) — validation program
- [INFERENCE_GATEWAY.md](INFERENCE_GATEWAY.md) — LiteLLM path for **LF-F02**
- [AGENT_GRAPH_ARCHITECTURE.md](AGENT_GRAPH_ARCHITECTURE.md) — sidecar graph for **LF-F03**
- [ADR 0010](architecture/decisions/0010-daemon-exports-observability-via-otel-and-sidecar-api.md), [ADR 0020](architecture/decisions/0020-otel-genai-semconv-with-rex-pipeline-metrics.md) — OTel semconv retained
