# ADR 0026: Rex-owned observability storage; bundled Grafana via Rex OTel datasource

- **Date:** 2026-06-04
- **Status:** Accepted
- **Supersedes:** Visualization and BYOT-default portions of [ADR 0010](0010-daemon-exports-observability-via-otel-and-sidecar-api.md) and [ADR 0021](0021-rex-owned-economics-store-byot-visualization.md). Ingest semconv, daemon authority, and `SidecarObservabilityService` intent from ADR 0010 remain.

## Context

[ADR 0021](0021-rex-owned-economics-store-byot-visualization.md) established **`rex-obs-store`** under `$REX_ROOT` when observability is enabled. [ADR 0010](0010-daemon-exports-observability-via-otel-and-sidecar-api.md) and [OBSERVABILITY_INTEGRATIONS.md](../../historical/OBSERVABILITY_INTEGRATIONS.md) described **BYOT** as the default visualization path: operator-run collectors, PromQL/Loki/Tempo backends, and optional OTLP export to external stacks.

Product direction now requires:

1. **Rex-owned storage as system of record** — OpenTelemetry **semantic conventions** (`gen_ai.*`, `rex.*`) persisted under Rex control; storage engine per [ADR 0025](0025-dual-economics-store-engines.md) is an implementation detail; Grafana does **not** read store files directly.
2. **No external TSDB query plane** for the product path — no required OpenTelemetry Collector, Prometheus, Loki, or Tempo for local dashboards.
3. **Bundled Grafana** — Rex vendors Grafana (and provisioning) so operators install **Rex only**; one command (`rex obs up`, planned) starts the local suite with **default preset dashboards**.
4. **Rex observability read API** — loopback HTTP (and optional gRPC) serves historical query and realtime subscribe; a **Rex Grafana datasource plugin** maps OTel-shaped responses for panels. This is **not** PromQL, LogQL, or TraceQL against operator-managed stores.
5. **Optional OTLP export** — replication to external fleet backends remains a **Could** interop path, not the UI path.

Partial Phase 2 code may still export OTLP when `observability.otlp.endpoint` is set; documentation and follow-up PRs align code with optional interop.

## Decision

1. **System of record** — When `observability.enabled: true`, `rex-daemon` writes telemetry to **`rex-obs-store`** and exposes a **Rex read API** for UI consumers. Sidecars with OTel libraries report through **`SidecarObservabilityService`** on daemon UDS ([ADR 0010](0010-daemon-exports-observability-via-otel-and-sidecar-api.md)), not direct export to external collectors.

2. **Bundled Grafana suite (planned)** — Rex distributes a pinned **Grafana** binary (or kit under `$REX_ROOT/obs/vendor/`), **provisioning** (datasources, default dashboard JSON), and supervises process lifecycle via **`rex obs up`** (planned). Operators do not install Collector, Prometheus, Loki, or Tempo for the product path.

3. **Rex Grafana OTel datasource (planned)** — Grafana panels use a **Rex datasource plugin** that calls the Rex read API and presents **OpenTelemetry-shaped** metrics, traces, and logs. No SQLite file datasource, Prometheus scrape, or Mimir/Loki bridges for the primary path.

4. **Read API contract (planned)** — Loopback-only by default: catalog (`gen_ai.*`, `rex.*`), historical stream query, rollups, live SSE (or gRPC stream) for in-flight economics. Paths and OpenAPI/proto land in implementation PRs; hub documents intent in [OBSERVABILITY_AND_ECONOMICS.md](../../historical/OBSERVABILITY_AND_ECONOMICS.md).

5. **Optional OTLP interop (Could)** — `observability.otlp.*` may replicate aggregates to operator backends; not required when observability is enabled and not used by bundled Grafana.

| Do | Do not |
|----|--------|
| Own storage, read API, and bundled Grafana lifecycle | Require operator-installed Collector/TSDB for local UI |
| Ship default Grafana dashboards for core `rex.*` / `gen_ai.*` signals | Use PromQL/LogQL/TraceQL as the product read contract |
| Vendor Grafana + Rex datasource in Rex distribution | Mandate Docker for the local suite |
| Keep sidecar ingest on daemon UDS | Let sidecars bypass daemon to external OTLP endpoints |

## Consequences

- **Positive:** One install, one command, coherent economics UX; storage and UI aligned with daemon-first authority; OTel semconv without LGTM operational burden.
- **Negative:** Rex owns Grafana kit versioning, AGPL distribution considerations, datasource plugin maintenance, and read API schema evolution.
- **Risks / follow-up:** Implementation PRs for read API, plugin, `rex obs up`, and provisioning; narrow shipped OTLP export to optional interop; realtime path beyond terminal-only store append.

## Implementation status (2026-06)

Canonical hub: [OBSERVABILITY_AND_ECONOMICS.md](../../historical/OBSERVABILITY_AND_ECONOMICS.md).

| Item | Status |
|------|--------|
| Rex-owned SQLite store + read API | **Shipped** |
| Rex Grafana OTel datasource plugin | **Shipped** — `integrations/grafana-rex-otel/` |
| `rex obs up` + provisioning templates | **Shipped** — operator supplies Grafana binary from PATH or vendor dir |
| Default preset dashboards | **Shipped** — `templates/obs/` |
| CHCE mmap engine (`engine=mmap`) | **Planned** — [CHCE_ROADMAP.md](../../historical/CHCE_ROADMAP.md) |
| SSE live subscribe | **Planned** — Phase 6 |
| `SidecarObservabilityService` | **Planned** |

Decision bullets above record acceptance-time intent; this section tracks shipped vs planned code.

## Related

- [ADR 0010](0010-daemon-exports-observability-via-otel-and-sidecar-api.md) · [ADR 0020](0020-otel-genai-semconv-with-rex-pipeline-metrics.md) · [ADR 0021](0021-rex-owned-economics-store-byot-visualization.md) · [ADR 0025](0025-dual-economics-store-engines.md)
- [OBSERVABILITY_AND_ECONOMICS.md](../../historical/OBSERVABILITY_AND_ECONOMICS.md) · [OBSERVABILITY_INTEGRATIONS.md](../../historical/OBSERVABILITY_INTEGRATIONS.md) · [CONFIGURATION.md](../../CONFIGURATION.md)
- [README.md](README.md) (index)
