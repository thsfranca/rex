# Rex observability read API

**Diátaxis role:** reference — loopback HTTP contract for Rex-owned economics storage.

**Status:** **implemented** — SQLite engine interim; Rex-native store target per [ADR 0025](architecture/decisions/0025-dual-economics-store-engines.md).

**Hub:** [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md) · **Integrations:** [OBSERVABILITY_INTEGRATIONS.md](OBSERVABILITY_INTEGRATIONS.md) · **ADR:** [0026](architecture/decisions/0026-rex-owned-storage-grafana-otel-datasource.md)

## Purpose

Expose **engine-agnostic** historical queries over logical `streams` records as **OpenTelemetry-shaped JSON** for the Rex Grafana OTel datasource. Grafana does not read store files or SQL.

## Bind

- Config: `observability.read_api.listen` (default `127.0.0.1:9470`)
- **Loopback only** — non-loopback hosts fail `rex config validate`

## Endpoints

| Method | Path | Response |
|--------|------|----------|
| `GET` | `/health` | `{ "status": "ok", "service": "rex-obs-read-api" }` |
| `GET` | `/v1/catalog` | `{ "instruments": [ … InstrumentCatalogEntry … ] }` |
| `POST` | `/v1/metrics/query` | OTLP-style `{ "resourceMetrics": [ … ] }` |

### Metrics query body

```json
{
  "start_ms": 0,
  "end_ms": 9999999999999,
  "instruments": ["rex.stream.requests"],
  "labels": { "terminal": "done" }
}
```

Fixture: [fixtures/obs_read_api/metrics_query_request.json](../fixtures/obs_read_api/metrics_query_request.json).

## CLI

```bash
rex obs serve    # read API only
rex obs catalog  # instrument list
rex obs doctor   # config + TCP health
rex obs up       # read API + Grafana provisioning
```

## Error codes

| Code | When |
|------|------|
| `obs.read_api.bind_failed` | Invalid or non-loopback bind |
| `obs.read_api.query_invalid` | Malformed JSON query body |

## Related

- [CONFIGURATION.md — Observability](CONFIGURATION.md#observability)
- [ADR 0020](architecture/decisions/0020-otel-genai-semconv-with-rex-pipeline-metrics.md) — instrument names
