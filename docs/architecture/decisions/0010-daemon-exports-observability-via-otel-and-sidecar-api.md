# ADR 0010: Daemon exports observability via OTLP; sidecar observability API

- **Date:** 2026-05-24
- **Status:** Accepted — **partially superseded by [ADR 0026](0026-rex-owned-storage-grafana-otel-datasource.md)** (BYOT-default visualization and OTLP-as-primary-UI-path). Daemon ingest, semconv, and `SidecarObservabilityService` remain.

## Context

Rex economics signals today are **stdout grep only** — sufficient for development, insufficient for operator dashboards, sidecar-authored metrics, or vendor-neutral export.

Requirements gathered from product architecture discussion:

1. **Vendor-neutral export** — Operators choose collector, storage, and UI (Grafana, VictoriaMetrics, Datadog OTLP, observr, etc.). Rex must not ship or mandate a bundled observability stack.

2. **Daemon authority** — Per [ADR 0001](0001-daemon-owns-agent-orchestration-and-economics.md), economics and stream authority stay in `rex-daemon`. Export must not add a second supervised process or duplicate the 0-or-1 agent sidecar model.

3. **Sidecar custom metrics** — Agent runtimes need a bounded path to declare and record custom instruments without direct host network access. Metrics must flow through daemon export so labels and resource attributes stay consistent.

4. **Non-blocking streams** — OTLP export runs on a background path; it must not block `StreamInference` or broker RPC latency.

5. **Stdout compatibility** — Grep recipes remain the phase-0 operator path; OTLP is additive when enabled.

Rejected alternatives considered:

- **Dedicated export sidecar** — extra hop and supervision cost; conflicts with sidecar budget.
- **Rex-managed Grafana / VictoriaMetrics stack** — operator BYOT is simpler and avoids Rex owning storage lifecycle.
- **Extension-embedded dashboard** — duplicates OSS UI investment; poor fit for headless daemon operators.

## Decision

1. **Daemon OTLP export (planned implementation)** 
 When **`observability.enabled`** is true in merged JSON ([CONFIGURATION.md](../../CONFIGURATION.md)), `rex-daemon` embeds the OpenTelemetry SDK and pushes **metrics** via **OTLP** using `observability.otlp.endpoint` and `observability.otlp.protocol`. Stdout grep is retained. Rex does not use `REX_OBS_*` or `OTEL_EXPORTER_OTLP_*` env vars for product configuration.

2. **`SidecarObservabilityService` on daemon UDS (planned implementation)** 
 Sidecars call **`SidecarObservabilityService`** over the **daemon UDS** (`daemon.socket`), not the sidecar control-plane socket. RPCs: `RegisterMetric`, `RecordMetric`, `GetEconomicsSnapshot`, `ReportResourceStats`. The daemon aggregates sidecar points and exports them as `rex.sidecar.custom.*` on the same OTLP stream as daemon economics.

3. **BYOT tooling** 
 Rex documents integration patterns only ([OBSERVABILITY_INTEGRATIONS.md](../../OBSERVABILITY_INTEGRATIONS.md)). Rex does not ship collectors, TSDBs, or dashboard servers.

4. **Primary contract: OTLP push** 
 OTLP is the primary interoperability surface. Optional Prometheus scrape from Rex is **deferred** unless an operator requests it.

5. **CLI helpers (planned, not shipped)** 
 Future `rex obs config|doctor|catalog` may assist operators (print merged JSON, health checks, instrument list); documentation targets only until implemented.

| Do | Do not |
|----|--------|
| Export stable `rex.*` instruments from the daemon | Ship Rex-managed Grafana/VM stacks |
| Expose a narrow sidecar observability API on **daemon UDS** | Add a dedicated observability-only sidecar |
| Document BYOT collector + UI wiring | Claim OTLP or sidecar API is implemented before code lands |
| Keep export off the inference hot path | Block streams on export backpressure |

## Consequences

- **Positive:** One reviewable export contract; operators bring familiar tooling; sidecar metrics stay brokered and label-consistent with daemon economics.
- **Negative:** Daemon gains OTel SDK complexity; sidecar authors must use daemon UDS for observability RPCs (distinct from sidecar broker socket).
- **Risks / follow-up:** PII must stay out of exported logs/traces by default; trace correlation between daemon and sidecar needs explicit propagation design in a later phase. Implementation PRs (proto, handlers, OTel wiring) follow this ADR. Local economics persistence is accepted in [ADR 0021](0021-rex-owned-economics-store-byot-visualization.md) — complementary to OTLP, not a rejected pattern.

## Implementation status (2026-06)

Canonical hub: [OBSERVABILITY_AND_ECONOMICS.md](../../OBSERVABILITY_AND_ECONOMICS.md).

| Item | Status |
|------|--------|
| SQLite `rex-obs-store` + OTLP export | **Shipped** |
| Rex observability read API (historical query) | **Shipped** — [OBS_READ_API.md](../../OBS_READ_API.md) |
| Grafana Rex OTel datasource plugin + `rex obs up` | **Shipped** — [OBSERVABILITY_INTEGRATIONS.md](../../OBSERVABILITY_INTEGRATIONS.md) |
| `rex obs` CLI (`serve`, `up`, `down`, `doctor`, `catalog`) | **Shipped** |
| CHCE mmap engine | **Planned** — [CHCE_ROADMAP.md](../../CHCE_ROADMAP.md), [ADR 0027](0027-chce-columnar-mmap-engine.md) |
| SSE live tail (read API) | **Planned** — Phase 6 |
| `SidecarObservabilityService` on daemon UDS | **Planned** |

## Related

- [ADR 0001](0001-daemon-owns-agent-orchestration-and-economics.md) · [ADR 0008](0008-dedicated-sidecar-control-plane-api.md)
- [ADR 0020](0020-otel-genai-semconv-with-rex-pipeline-metrics.md) · [ADR 0021](0021-rex-owned-economics-store-byot-visualization.md) · [ADR 0026](0026-rex-owned-storage-grafana-otel-datasource.md)
- [OBSERVABILITY_AND_ECONOMICS.md](../../OBSERVABILITY_AND_ECONOMICS.md) · [OBSERVABILITY_INTEGRATIONS.md](../../OBSERVABILITY_INTEGRATIONS.md) · [CONFIGURATION.md](../../CONFIGURATION.md) · [SIDECAR_RUNTIME.md](../../SIDECAR_RUNTIME.md)
- [README.md](README.md) (index)
