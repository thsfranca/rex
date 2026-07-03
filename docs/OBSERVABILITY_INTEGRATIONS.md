# Observability integrations (bundled Grafana suite)

> **Status: Cancelled (2026-06-09).** Rex-owned Grafana suite and `rex obs up` are **not** the product path. Use **[LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md)** (LangFuse Cloud). Shipped code **removed in LF-R01**.

**Diátaxis role:** how-to — **historical** — Rex-owned local observability suite.

**Status:** **cancelled** — superseded by LangFuse Cloud ([LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md)).

**Decision records:** [ADR 0010](architecture/decisions/0010-daemon-exports-observability-via-otel-and-sidecar-api.md) · [ADR 0020](architecture/decisions/0020-otel-genai-semconv-with-rex-pipeline-metrics.md) · [ADR 0021](architecture/decisions/0021-rex-owned-economics-store-byot-visualization.md) · [ADR 0025](architecture/decisions/0025-dual-economics-store-engines.md) · [ADR 0026](architecture/decisions/0026-rex-owned-storage-grafana-otel-datasource.md) · [ADR 0027](architecture/decisions/0027-chce-columnar-mmap-engine.md) · **Design hub:** [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md)

## Configuration surface

Rex observability is configured only in **merged JSON** ([CONFIGURATION.md](CONFIGURATION.md)). Set optional bootstrap env **`REX_ROOT`** for layout location. Do **not** use `REX_OBS_*` environment variables — they are not part of the product configuration surface.

## Purpose

Rex **owns telemetry storage** and serves **OpenTelemetry-shaped** data to **bundled Grafana** through a **Rex datasource plugin**. Operators install **Rex only** — no separate Collector, Prometheus, Loki, or Tempo installs for the product path. One command (**`rex obs up`**) starts the local suite with **default preset dashboards**.

## Prerequisites (product path)

- Rex installed (includes vendored Grafana kit when implementation lands).
- `observability.enabled: true` in merged JSON.
- `rex-daemon` running (started by `rex obs up` or separately).

## Quick start

```bash
rex obs up
```

Expected behavior:

1. Start Rex observability read API on loopback (default `127.0.0.1`).
2. Start vendored **Grafana** with provisioning from `$REX_ROOT/obs/grafana/provisioning/`.
3. Ensure daemon observability ingest is active.
4. Open Grafana (default `http://127.0.0.1:3000`) with **Rex Economics** dashboard folder preloaded.

No Docker. No operator-installed TSDBs.

## JSON configuration

Keys live under `observability` in `$REX_ROOT/config.json` (and optional `.rex/config.json`). Full table: [CONFIGURATION.md — Observability](CONFIGURATION.md#observability).

Example (product path):

```json
{
 "observability": {
 "enabled": true,
 "service_name": "rex-daemon",
 "read_api": {
 "listen": "127.0.0.1:9470"
 },
 "ui": {
 "enabled": true,
 "grafana": {
 "port": 3000
 }
 },
 "store": {
 "engine": "sqlite",
 "path": "obs/store.sqlite",
 "format_version": 1
 }
 }
}
```

Optional OTLP interop (not required for bundled Grafana):

```json
"otlp": {
 "endpoint": "http://127.0.0.1:4317",
 "protocol": "grpc"
}
```

After editing config, run `rex config validate` and restart the daemon.

## Rex Grafana OTel datasource (planned)

Grafana panels use a **Rex datasource plugin** — not Prometheus, Loki, Tempo, or a SQLite file datasource.

| Concern | Contract |
|---------|----------|
| **Transport** | HTTP to Rex read API on loopback (`observability.read_api.listen`) |
| **Auth** | Loopback-only; no tokens in product JSON (local dev) |
| **Metrics** | OTel-shaped series for `gen_ai.*` and `rex.*` ([metric catalog](#metric-catalog)) |
| **Traces / logs** | OTel-shaped spans and log records when Phases 6+ land |
| **Live data** | `GET /v1/metrics/stream` SSE with `cursor_commit_ms` merge — **planned** Phase 6 ([OBS_READ_API.md](OBS_READ_API.md), [ADR 0027](architecture/decisions/0027-chce-columnar-mmap-engine.md)); plugin `"streaming": true` when shipped |
| **Provisioning** | `$REX_ROOT/obs/grafana/provisioning/datasources/rex.yml` (planned) |

**Not used:** PromQL, LogQL, TraceQL, OpenTelemetry Collector, Mimir, Loki, Tempo.

## Default dashboards (planned)

Shipped dashboard JSON under `$REX_ROOT/obs/grafana/provisioning/dashboards/` (or Rex distribution templates). Planned folder: **Rex Economics**.

| Dashboard | Panels (intent) |
|-----------|-----------------|
| **rex-economics-overview** | Cache decision rate (`rex.cache.decisions`); stream latency (`gen_ai.client.operation.duration`); prompt vs selected tokens |
| **rex-pipeline** | Retrieval duration; compression ratio; route breakdown |
| **rex-streams-live** | Active streams; terminal outcomes; realtime rollup |

Dashboard UIDs and JSON land in implementation PRs.

## Metric catalog

Stable instrument names for panels, store, and read API. See [ADR 0020](architecture/decisions/0020-otel-genai-semconv-with-rex-pipeline-metrics.md).

### OpenTelemetry GenAI

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

## Sidecar author flow (planned)

1. Connect gRPC client to **daemon UDS** (`daemon.socket` in config).
2. Call `RegisterMetric` with name, type, and allowed label keys.
3. Call `RecordMetric` during agent turns; daemon persists and serves via read API as `rex.sidecar.custom.<name>`.
4. Optionally call `GetEconomicsSnapshot` for bounded recent summaries.
5. Optionally call `ReportResourceStats` for self-reported CPU/memory.

Broker RPCs remain on the sidecar control-plane socket; observability RPCs use **daemon UDS** only — [ADR 0010](architecture/decisions/0010-daemon-exports-observability-via-otel-and-sidecar-api.md).

## CLI helpers (planned)

| Command | Purpose |
|---------|---------|
| `rex obs up` | Start read API + bundled Grafana + default dashboards |
| `rex obs down` | Stop supervised observability processes |
| `rex obs config` | Print merged `observability` JSON fragment |
| `rex obs doctor` | Check read API, Grafana, and daemon ingest health |
| `rex obs catalog` | List stable `gen_ai.*` and `rex.*` instrument names |
| `rex obs compare` | Query store for model/config snapshot A/B |

## PII and data handling

- Do **not** persist raw prompts, file contents, or user identifiers in metric labels or store fields exposed to Grafana.
- Prefer bounded cardinality labels (`route`, `terminal`, `decision`) over free-text.
- Logs and traces phases must default to **no prompt bodies** — see [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md).

## Troubleshooting

| Symptom | Likely cause | Check |
|---------|--------------|-------|
| Empty Grafana panels | Read API not running or datasource URL wrong | `rex obs doctor`; `observability.read_api.listen` |
| No new data | `observability.enabled` not `true` | `rex config show`; restart daemon |
| Sidecar metrics missing | API called on wrong socket | Use daemon UDS (`daemon.socket`), not sidecar socket |
| Stdout shows `obs.export=degraded` | Optional OTLP interop endpoint down | Not required for bundled Grafana; check `observability.otlp` only if interop enabled |

## Optional interop (Could)

Not the product UI path. Use when replicating telemetry to an operator-managed backend.

| Pattern | Notes |
|---------|--------|
| OTLP export | Set `observability.otlp.endpoint`; daemon pushes to **your** collector |
| External Grafana Cloud / Datadog | Collector or intake on operator side; API keys stay outside Rex JSON |

Former BYOT bridge patterns (Collector + Mimir, Prometheus scrape, SQLite file datasource) are **deprecated** for the Rex product path per [ADR 0026](architecture/decisions/0026-rex-owned-storage-grafana-otel-datasource.md).

## Related

- [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md) · [ECONOMICS_VALIDATION.md](ECONOMICS_VALIDATION.md)
- [CONFIGURATION.md](CONFIGURATION.md) · [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md) · [ARCHITECTURE.md](ARCHITECTURE.md#observability) · [CI.md](CI.md)
