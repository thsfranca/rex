# Observability integrations (BYOT how-to)

**Diátaxis role:** how-to — connect Rex OTLP export to **your** collector, storage, and UI.

**Status:** **design documented** — JSON keys, metric names, and patterns below describe **planned** behavior. OTLP export, `SidecarObservabilityService`, and `rex obs` helpers are **not shipped** until implementation PRs land. Phase 0 today: [stdout grep in OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md).

**Decision records:** [ADR 0010](architecture/decisions/0010-daemon-exports-observability-via-otel-and-sidecar-api.md) · [ADR 0020](architecture/decisions/0020-otel-genai-semconv-with-rex-pipeline-metrics.md) · [ADR 0021](architecture/decisions/0021-rex-owned-economics-store-byot-visualization.md) · **Design hub:** [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md)

## Configuration surface

Rex observability is configured only in **merged JSON** ([CONFIGURATION.md](CONFIGURATION.md)). Set optional bootstrap env **`REX_ROOT`** for layout location. Do **not** use `REX_OBS_*` environment variables — they are not part of the product configuration surface.

Your **collector** (OpenTelemetry Collector, Grafana Agent, etc.) may use its own env or yaml; that is operator-owned and outside Rex.

## Purpose

Rex emits **OTLP metrics** from `rex-daemon` when `observability.enabled` is true. Operators **bring their own tooling (BYOT)** — any OpenTelemetry-compatible collector and any UI (Grafana, VictoriaMetrics, Datadog, observr, etc.). Rex does not ship collectors, TSDBs, or dashboard servers.

## Prerequisites

- `rex-daemon` running with economics stdout (phase 0 baseline).
- An OTLP-capable collector reachable from the daemon host.
- After implementation: `observability.enabled: true` and `observability.otlp.endpoint` in merged config.

## JSON configuration (planned)

Keys live under `observability` in `$REX_ROOT/config.json` (and optional `.rex/config.json`). Full table: [CONFIGURATION.md — Observability](CONFIGURATION.md#observability-planned).

Example:

```json
{
  "observability": {
    "enabled": true,
    "service_name": "rex-daemon",
    "custom_sidecar_metrics": true,
    "otlp": {
      "endpoint": "http://127.0.0.1:4317",
      "protocol": "grpc"
    },
    "store": { "path": "obs/store.sqlite" }
  }
}
```

After editing config, run `rex config validate` and restart the daemon.

## Metric catalogs (planned)

### OpenTelemetry GenAI (interop)

| Instrument | Type | Labels (bounded) | Maps from stdout |
|------------|------|------------------|------------------|
| `gen_ai.client.token.usage` | histogram | `gen_ai.token.type`, `model_id`, `route` | `prompt_tokens`, completion estimates |
| `gen_ai.client.operation.duration` | histogram | `model_id`, `route`, `error.type` | `elapsed_ms` |
| `gen_ai.client.operation.time_to_first_chunk` | histogram | `model_id`, `route` | first NDJSON chunk delta |

### Rex pipeline (`rex.*`)

| Instrument | Type | Labels (bounded) | Maps from stdout |
|------------|------|------------------|------------------|
| `rex.stream.requests` | counter | `terminal`, `inference_runtime`, `route` | `stream.terminal`, `route=` |
| `rex.stream.duration_ms` | histogram | same | `elapsed_ms` |
| `rex.cache.decisions` | counter | `decision` | `cache_decision=` |
| `rex.context.prompt_tokens` | histogram | `mode`, `route` | `stream.metrics` |
| `rex.context.selected_tokens` | histogram | `mode`, `route` | `context_tokens` |
| `rex.pipeline.retrieval.duration` | histogram | `retrieval_status` | `retrieval=` |
| `rex.pipeline.compression.ratio` | histogram | `compression_strategy` | `compression_strategy` |
| `rex.local.hardware.load_duration` | histogram | `model_id`, `quant` | Ollama `load_duration` (local route) |
| `rex.approval.decisions` | counter | `outcome` | `approval=` |
| `rex.broker.inference` | counter | `outcome` | `broker.inference=*` |
| `rex.obs.export.errors` | counter | `reason` | `obs.export=degraded` |

**Forbidden on metrics:** prompts, file paths, `conversation_id`, unbounded error text.

### Sidecar custom (via API)

| Prefix | Source |
|--------|--------|
| `rex.sidecar.custom.*` | `RegisterMetric` + `RecordMetric` on `SidecarObservabilityService` |

## Grafana bridges (Rex does not run Grafana)

| Bridge | Rex provides | Operator provides |
|--------|--------------|-------------------|
| **A. OTLP rollups** | Daemon pushes aggregates from `rex-obs-store` | Collector + Grafana/Mimir |
| **B. Prometheus scrape** | HTTP `/metrics` from rollups (planned) | Prometheus + Grafana |
| **C. SQLite datasource** | `$REX_ROOT/obs/store.sqlite` | Grafana file datasource |
| **D. `rex obs query`** | JSON/CSV export (planned) | Scripts, custom panels |

## Integration patterns

Use **config.json** snippets — no committed collector yaml in the Rex repo.

### Grafana + OpenTelemetry Collector

1. Run an [OpenTelemetry Collector](https://opentelemetry.io/docs/collector/) with an OTLP receiver and exporter to your Grafana Cloud / Mimir backend.
2. Enable observability in Rex config and point at the collector:

```json
"observability": {
  "enabled": true,
  "otlp": {
    "endpoint": "http://127.0.0.1:4317",
    "protocol": "grpc"
  }
}
```

3. `rex config validate` and restart `rex daemon`.
4. Build Grafana panels on `rex.cache.decisions`, `gen_ai.client.operation.duration`, and `rex.context.prompt_tokens`.

### VictoriaMetrics (OTLP ingest)

1. Enable OTLP ingestion on VictoriaMetrics per your deployment docs.
2. Set `observability.otlp.endpoint` to the VM OTLP URL; use `protocol: "http/protobuf"` if required.
3. Query using `rex_*` and `gen_ai_*` names after export lands.

### observr (local Mac)

1. Start observr with OTLP on a local port.
2. Set `observability.otlp.endpoint` to observr’s listener; keep `observability.enabled: true`.
3. Use observr for quick local validation — Rex does not ship a UI.

### Datadog (OTLP intake)

1. Configure Datadog OTLP intake per Datadog docs (headers/API key on **your** collector, not Rex JSON).
2. Rex daemon exports to the collector; collector forwards to Datadog.
3. Map `rex.*` metrics to monitors for cache hit rate and latency regressions.

## Example dashboard panels (prose)

- **Latency vs cache:** heatmap of `gen_ai.client.operation.duration` by `cache_decision` label.
- **Compression savings:** time series of `gen_ai.client.token.usage` (input) vs baseline intent volume when `rex.pipeline.compression.ratio` is present.
- **Local queue pressure:** gauge from operator-scraped vLLM `num_requests_waiting` when using local inference (not emitted by Rex).

## Sidecar author flow (planned)

1. Connect gRPC client to **daemon UDS** (`daemon.socket` in config).
2. Call `RegisterMetric` with name, type, and allowed label keys.
3. Call `RecordMetric` during agent turns; daemon exports as `rex.sidecar.custom.<name>`.
4. Optionally call `GetEconomicsSnapshot` for bounded recent summaries.
5. Optionally call `ReportResourceStats` for self-reported CPU/memory.

Broker RPCs remain on the sidecar control-plane socket; observability RPCs use **daemon UDS** only — [ADR 0010](architecture/decisions/0010-daemon-exports-observability-via-otel-and-sidecar-api.md).

## CLI helpers (planned, not shipped)

| Command | Purpose |
|---------|---------|
| `rex obs config` | Print merged `observability` JSON fragment (not shell `export` lines) |
| `rex obs doctor` | Check collector reachability and export health |
| `rex obs catalog` | List stable `gen_ai.*` and `rex.*` instrument names |
| `rex obs compare` | Query `rex-obs-store` for model/config snapshot A/B |

## PII and data handling

- Do **not** export raw prompts, file contents, or user identifiers in metric labels.
- Prefer bounded cardinality labels (`route`, `terminal`, `decision`) over free-text.
- Logs and traces phases must default to **no prompt bodies** — see [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md).

## Troubleshooting (after implementation)

| Symptom | Likely cause | Check |
|---------|--------------|-------|
| No metrics in backend | `observability.enabled` not `true` | `rex config show`; restart daemon |
| Collector unreachable | Wrong `observability.otlp.endpoint` | `rex obs doctor` or tcp probe |
| Sidecar metrics missing | API called on wrong socket | Use daemon UDS (`daemon.socket`), not sidecar socket |
| Stdout shows `obs.export=degraded` | Export backpressure or collector down | Collector logs; stdout grep still works |
| High cardinality | Custom metric labels unbounded | Fix sidecar `RegisterMetric` label set |

## Related

- [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md) · [ECONOMICS_VALIDATION.md](ECONOMICS_VALIDATION.md)
- [CONFIGURATION.md](CONFIGURATION.md) · [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md) · [ARCHITECTURE.md](ARCHITECTURE.md#observability) · [CI.md](CI.md)
