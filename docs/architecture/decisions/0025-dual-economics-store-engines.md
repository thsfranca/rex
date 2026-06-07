# ADR 0025: Dual economics store engines (SQLite default, mmap opt-in)

- **Date:** 2026-06-04
- **Status:** Accepted

## Context

[ADR 0021](0021-rex-owned-economics-store-byot-visualization.md) establishes a Rex-owned **SQLite** economics store at `$REX_ROOT` when `observability.enabled: true`. Apple Silicon deployments benefit from **memory-mapped**, append-oriented layouts aligned to **16 KB** pages and unified memory, but a single SQLite path does not capture that design space.

Deep research (2026-06-07) evaluated embedded SQL, cloud TSDBs, and columnar layouts. [ADR 0027](0027-chce-columnar-mmap-engine.md) adopts **CHCE** (columnar-mmap engine) for the mmap path; physical layout is in [OBS_STORE_MMAP_FORMAT.md](../../OBS_STORE_MMAP_FORMAT.md).

CI runs on **Linux** ([CI.md](../../CI.md)); the mmap engine must not be required on `ubuntu-latest`.

## Decision

1. **`rex-obs-store` exposes one logical write API** with two engines selected by merged JSON:
   - **`observability.store.engine: "sqlite"`** (default) — SQLite at `$REX_ROOT/<store.path>` (default `obs/store.sqlite`). [ADR 0021](0021-rex-owned-economics-store-byot-visualization.md) unchanged for this path.
   - **`observability.store.engine: "mmap"`** (opt-in, **macOS only**) — append-only mmap file per [OBS_STORE_MMAP_FORMAT.md](../../OBS_STORE_MMAP_FORMAT.md) (default path `obs/store.rexobs`).

2. **Feature gate unchanged** — `observability.enabled: true` enables **store + OTLP** together ([ADR 0021](0021-rex-owned-economics-store-byot-visualization.md)).

3. **Shared logical schema** — `config_snapshots`, `streams`, `runs`, `run_tasks` (names and FK semantics) are identical across engines; only on-disk encoding differs.

4. **Format versioning** — `observability.store.format_version` mirrors the mmap file header; incompatible versions fail closed with `store.format_version_unsupported`.

5. **Platform gate** — `engine=mmap` on non-macOS returns **`store.engine_unsupported`** (stable code — [ERROR_HANDLING.md](../../ERROR_HANDLING.md)).

6. **Default engine promotion** — Remains **`sqlite`** until promotion gates in [OBS_STORE_MMAP_FORMAT.md](../../OBS_STORE_MMAP_FORMAT.md) §Promotion gates are met; then a follow-up ADR amendment may flip the JSON default.

7. **Grafana UI** — Both engines expose data via the **Rex observability read API** and Rex Grafana datasource ([ADR 0026](0026-rex-owned-storage-grafana-otel-datasource.md)); Grafana does not read store files directly.

8. **Privacy** — No raw prompts or file bodies in either engine ([ADR 0021](0021-rex-owned-economics-store-byot-visualization.md)).

| Do | Do not |
|----|--------|
| Ship SQLite Phase 2 write path first | Require mmap on Linux CI |
| Document CHCE layout in OBS_STORE_MMAP_FORMAT + ADR 0027 | Assume ALP/Gorilla in mmap v1 ship |
| Version mmap files with `format_version` | Store prompts in economics files |

## Consequences

- **Positive:** Phase 2 can ship portable SQLite; Apple Silicon operators can opt into mmap without forking the logical schema; promotion path is explicit.
- **Negative:** Two engines to test and maintain; mmap migrations are layout-based, not SQL `ALTER TABLE`.
- **Related:** Dual-engine policy lives here; CHCE on-disk detail in [OBS_STORE_MMAP_FORMAT.md](../../OBS_STORE_MMAP_FORMAT.md) and [ADR 0027](0027-chce-columnar-mmap-engine.md).

## Related

- [ADR 0021](0021-rex-owned-economics-store-byot-visualization.md) · [ADR 0010](0010-daemon-exports-observability-via-otel-and-sidecar-api.md) · [ADR 0020](0020-otel-genai-semconv-with-rex-pipeline-metrics.md) · [ADR 0026](0026-rex-owned-storage-grafana-otel-datasource.md) · [ADR 0027](0027-chce-columnar-mmap-engine.md)
- [OBS_STORE_MMAP_FORMAT.md](../../OBS_STORE_MMAP_FORMAT.md) · [OBSERVABILITY_AND_ECONOMICS.md](../../OBSERVABILITY_AND_ECONOMICS.md) · [CONFIGURATION.md](../../CONFIGURATION.md)
