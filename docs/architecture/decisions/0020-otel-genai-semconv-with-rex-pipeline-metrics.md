# ADR 0020: OpenTelemetry GenAI semconv plus Rex pipeline metrics

- **Date:** 2026-05-24
- **Status:** Accepted

## Context

[ADR 0010](0010-daemon-exports-observability-via-otel-and-sidecar-api.md) commits to OTLP export from merged JSON when `observability.enabled` is true. Industry backends expect **OpenTelemetry Generative AI** semantic conventions (`gen_ai.client.*`).

Rex-specific pipeline stages need stable names that do not collide with generic client spans.

## Decision

1. **Dual namespace at export**
   - Emit **`gen_ai.client.*`** per [OTel GenAI semconv](https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-metrics/).
   - Emit **`rex.stream.*`**, **`rex.cache.*`**, **`rex.pipeline.*`**, and **`rex.local.*`** for product attribution.

2. **Configuration** — Read from merged JSON `observability` section ([CONFIGURATION.md](../../CONFIGURATION.md)). No `REX_OBS_*` env vars.

3. **Trace hierarchy (planned)** — Root `invoke_agent` with children `chat`, `retrievals`, `execute_tool`; high-cardinality ids on span attributes only.

4. **Label bounds** — Metrics: `model_id`, `route`, `mode`, `quant`, `lever_flags`, `cache_decision`, `finish_reason`. Forbidden: prompts, paths, `conversation_id`.

## Consequences

- **Positive:** OTel semconv enables bundled Grafana (Rex datasource) and optional interop export; pipeline stages remain attributable — [ADR 0026](0026-rex-owned-storage-grafana-otel-datasource.md).
- **Negative:** Export maps stdout → two instrument families.
- **Follow-up:** Daemon OTel wiring; catalog in [OBSERVABILITY_INTEGRATIONS.md](../../OBSERVABILITY_INTEGRATIONS.md).

## Related

- [ADR 0010](0010-daemon-exports-observability-via-otel-and-sidecar-api.md) · [ADR 0021](0021-rex-owned-economics-store-byot-visualization.md) · [ADR 0026](0026-rex-owned-storage-grafana-otel-datasource.md)
- [OBSERVABILITY_AND_ECONOMICS.md](../../OBSERVABILITY_AND_ECONOMICS.md)
