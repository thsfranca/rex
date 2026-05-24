# Observability integrations (BYOT how-to)

**Diátaxis role:** how-to — connect Rex OTLP export to **your** collector, storage, and UI.

**Status:** **design documented** — env vars, metric names, and patterns below describe **planned** behavior. OTLP export, `SidecarObservabilityService`, and `rex-cli obs` helpers are **not shipped** until implementation PRs land. Phase 0 today: [stdout grep in OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md).

**Decision record:** [ADR 0010](architecture/decisions/0010-daemon-exports-observability-via-otel-and-sidecar-api.md) · **Design hub:** [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md)

## Purpose

Rex emits **OTLP metrics** from `rex-daemon` when enabled. Operators **bring their own tooling (BYOT)** — any OpenTelemetry-compatible collector and any UI (Grafana, VictoriaMetrics, Datadog, observr, etc.). Rex does not ship collectors, TSDBs, or dashboard servers.

## Prerequisites

- `rex-daemon` running with economics stdout (phase 0 baseline).
- An OTLP-capable collector reachable from the daemon host.
- After implementation: `REX_OBS_ENABLED=1` and standard `OTEL_EXPORTER_OTLP_*` variables.

## Environment variables (planned)

| Variable | Default | Purpose |
|----------|---------|---------|
| `REX_OBS_ENABLED` | `0` | Enable daemon OTLP export when `1` |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | — | Collector OTLP endpoint (gRPC or HTTP per OTel SDK) |
| `OTEL_EXPORTER_OTLP_PROTOCOL` | `grpc` | `grpc` or `http/protobuf` |
| `OTEL_SERVICE_NAME` | `rex-daemon` | Resource attribute for daemon metrics |
| `REX_OBS_CUSTOM_METRICS` | `1` | When `0`, drop sidecar-registered custom metrics at export |

See [CONFIGURATION.md](CONFIGURATION.md) for daemon socket and adapter vars. Sidecar observability RPCs use **`REX_DAEMON_SOCKET`** (daemon UDS), not the sidecar control-plane socket — [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md).

## `rex.*` metric catalog (design names)

Stable instrument names for dashboards. **Not emitted until implementation.**

### Daemon economics

| Instrument | Type | Labels (illustrative) | Maps from stdout |
|------------|------|----------------------|------------------|
| `rex.stream.requests` | counter | `terminal`, `inference_runtime`, `route` | `stream.terminal`, `inference_runtime`, `route=` |
| `rex.stream.duration_ms` | histogram | same | `elapsed_ms` |
| `rex.cache.decisions` | counter | `decision` (`hit`, `miss_stored`, `bypass`, `uncacheable_mode`) | `cache_decision=` |
| `rex.context.prompt_tokens` | histogram | `mode`, `route` | `stream.metrics` / `prompt_tokens` |
| `rex.context.selected_tokens` | histogram | `mode`, `route` | `context_tokens`, `selected` |
| `rex.approval.decisions` | counter | `outcome` | `approval=` |
| `rex.broker.inference` | counter | `outcome` | `broker.inference=*` |
| `rex.obs.export.errors` | counter | `reason` | `obs.export=degraded` (stdout fallback) |

### Sidecar custom (via API)

| Prefix | Source |
|--------|--------|
| `rex.sidecar.custom.*` | `RegisterMetric` + `RecordMetric` on `SidecarObservabilityService` |

## Integration patterns

Each pattern uses **env blocks only** — no committed yaml/json assets in the Rex repo.

### Grafana + OpenTelemetry Collector

1. Run an [OpenTelemetry Collector](https://opentelemetry.io/docs/collector/) with an OTLP receiver and a Prometheus remote-write or OTLP exporter to your Grafana Cloud / Mimir backend.
2. Point the daemon at the collector:

```bash
export REX_OBS_ENABLED=1
export OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4317
export OTEL_EXPORTER_OTLP_PROTOCOL=grpc
```

3. Import or build Grafana panels on `rex.cache.decisions`, `rex.stream.duration_ms`, and `rex.context.prompt_tokens`.

### VictoriaMetrics (OTLP ingest)

1. Enable OTLP ingestion on VictoriaMetrics (single-node or cluster per your deployment docs).
2. Set `OTEL_EXPORTER_OTLP_ENDPOINT` to the VM OTLP URL; use HTTP/protobuf if your VM build requires it.
3. Query in Grafana or vmui using the `rex_*` metric names after export lands.

### observr (local Mac)

1. Start observr with OTLP enabled on a local port.
2. Set `OTEL_EXPORTER_OTLP_ENDPOINT` to observr’s listener; keep `REX_OBS_ENABLED=1`.
3. Use observr for quick local validation during development — no Rex-managed UI.

### Datadog (OTLP intake)

1. Configure Datadog OTLP intake endpoint and API key per Datadog docs.
2. Export with `OTEL_EXPORTER_OTLP_ENDPOINT` and required Datadog headers/env.
3. Map `rex.*` metrics to monitors for cache hit rate and stream latency regressions.

## Sidecar author flow (planned)

1. Connect gRPC client to **daemon UDS** (`REX_DAEMON_SOCKET`).
2. Call `RegisterMetric` with name, type (counter/gauge/histogram), and allowed label keys.
3. Call `RecordMetric` during agent turns; daemon exports as `rex.sidecar.custom.<name>`.
4. Optionally call `GetEconomicsSnapshot` for bounded recent cache/route/token summaries — not a time-series query API.
5. Optionally call `ReportResourceStats` for self-reported CPU/memory.

Broker RPCs (`RunTurn`, inference, tools) remain on the sidecar control-plane socket; observability RPCs use **daemon UDS** only — [ADR 0010](architecture/decisions/0010-daemon-exports-observability-via-otel-and-sidecar-api.md).

## CLI helpers (planned, not shipped)

| Command | Purpose |
|---------|---------|
| `rex-cli obs env` | Print recommended `REX_OBS_*` and `OTEL_*` exports for copy-paste |
| `rex-cli obs doctor` | Check collector reachability and export health |
| `rex-cli obs catalog` | List stable `rex.*` instrument names |

## PII and data handling

- Do **not** export raw prompts, file contents, or user identifiers in metric labels.
- Prefer bounded cardinality labels (`route`, `terminal`, `decision`) over free-text.
- Logs and traces phases must default to **no prompt bodies** — see open questions in [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md).

## Troubleshooting (after implementation)

| Symptom | Likely cause | Check |
|---------|--------------|-------|
| No metrics in backend | `REX_OBS_ENABLED` not set | Env + daemon restart |
| Collector unreachable | Wrong host/port or TLS | `rex-cli obs doctor` (when shipped) or curl/tcp probe |
| Sidecar metrics missing | API called on wrong socket | Use daemon UDS, not sidecar socket |
| Stdout shows `obs.export=degraded` | Export backpressure or collector down | Collector logs; stdout grep still works |
| High cardinality | Custom metric labels unbounded | Fix sidecar `RegisterMetric` label set |

## Related

- [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md) · [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md) · [ARCHITECTURE.md](ARCHITECTURE.md#observability)
- [CONFIGURATION.md](CONFIGURATION.md) · [CI.md](CI.md)
