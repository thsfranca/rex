# ADR 0021: Rex-owned economics store; BYOT visualization

- **Date:** 2026-05-24
- **Status:** Accepted — **partially superseded by [ADR 0026](0026-rex-owned-storage-grafana-otel-datasource.md)** (BYOT visualization and “Rex does not run Grafana”). Rex-owned store and config snapshot dedup remain.

## Context

[ADR 0010](0010-daemon-exports-observability-via-otel-and-sidecar-api.md) rejects Rex **supervising** Grafana, VictoriaMetrics, or collector lifecycle. Operators want Rex to **persist** economics data under `$REX_ROOT` with optimizations (config snapshot dedup, validation runs) while using Grafana only as a viewer.

[CONFIGURATION.md](../../CONFIGURATION.md) defines product settings in JSON only; **`REX_ROOT`** is the sole bootstrap env var.

## Decision

1. **`rex-obs-store` (planned)** — SQLite at `$REX_ROOT/<observability.store.path>` (default `obs/store.sqlite`).

2. **Feature gate** — When **`observability.enabled: true`** in merged JSON, enable **store + OTLP export** together. When false or omitted, stdout grep only. No separate env toggle.

3. **Config snapshot dedup** — Table `config_snapshots` keyed by content hash; `streams` / `runs` store `snapshot_id` FK only.

4. **BYOT visualization** — Rex does not run Grafana or collectors. Operators use OTLP rollups, Prometheus scrape, Grafana SQLite datasource, or `rex obs query` (planned).

5. **Privacy** — No raw prompts or file bodies in the economics DB by default.

| Do | Do not |
|----|--------|
| Persist under `$REX_ROOT` when observability JSON enabled | Ship or supervise Grafana/collector |
| Read OTLP endpoint from `observability.otlp` | Use `REX_OBS_*` or `OTEL_*` on Rex process for product config |
| Export aggregates for operator backends | Replace fleet TSDB with Rex TSDB product |

## Consequences

- **Positive:** Model comparison and harness history without external DB; aligns with daemon-first economics.
- **Negative:** Schema migrations and retention are Rex responsibilities for local data.
- **Related:** Complements [ADR 0010](0010-daemon-exports-observability-via-otel-and-sidecar-api.md) — BYOT applies to visualization backends, not prohibition of local persistence.

## Related

- [ADR 0010](0010-daemon-exports-observability-via-otel-and-sidecar-api.md) · [ADR 0020](0020-otel-genai-semconv-with-rex-pipeline-metrics.md) · [ADR 0025](0025-dual-economics-store-engines.md) (dual engines — **SQLite remains default**; mmap opt-in on macOS) · [ADR 0026](0026-rex-owned-storage-grafana-otel-datasource.md)
- [OBSERVABILITY_AND_ECONOMICS.md](../../OBSERVABILITY_AND_ECONOMICS.md) · [OBS_STORE_MMAP_FORMAT.md](../../OBS_STORE_MMAP_FORMAT.md) · [ECONOMICS_VALIDATION.md](../../ECONOMICS_VALIDATION.md) · [CONFIGURATION.md](../../CONFIGURATION.md)
