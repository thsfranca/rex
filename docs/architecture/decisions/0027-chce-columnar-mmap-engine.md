# ADR 0027: CHCE columnar-mmap observability engine

- **Date:** 2026-06-07
- **Status:** Accepted

## Context

[ADR 0025](0025-dual-economics-store-engines.md) establishes dual store engines: SQLite default and mmap opt-in on macOS. The prior mmap format draft described row-oriented musli records with `F_FULLFSYNC`. Deep research (2026-06-07) evaluated embedded SQL, cloud TSDBs, LSM KV, and columnar file formats against Rex constraints (hot-path latency, disk budget, programmatic API, Grafana loopback reads).

The research recommends **CHCE** — a Custom Hybrid Columnar-mmap Engine: lock-free ring buffer ingest, 16 KB columnar pages, global dictionary sidecar, zone-map skipping, and phased compression. Format detail lives in [OBS_STORE_MMAP_FORMAT.md](../../historical/OBS_STORE_MMAP_FORMAT.md).

## Decision

1. **Replace the mmap on-disk design** with CHCE columnar pages — not row-musli records within pages. SQLite encoding unchanged.

2. **Durability sync policy** — use **`F_BARRIERFSYNC`** when sealing 16 KB pages, not `F_FULLFSYNC`. Accept theoretical data loss on sudden power loss in exchange for sub-millisecond ingest on Apple NVMe. Document in [CONFIGURATION.md](../../CONFIGURATION.md).

3. **Trace and turn identifiers** — store `trace_id` and `turn_id` in a **sparse secondary index**, not in dense per-stream metric columns, to meet the &lt;8 MB / 1M streams disk target.

4. **Global dictionary** — categorical strings map through mmap sidecar `obs/store.dict` (u16 ordinals), not per-page inline dictionaries.

5. **Live tail** — `LiveRingBuffer` plus read API **SSE** (`GET /v1/metrics/stream`) with cursor merge semantics ([OBS_READ_API.md](../../historical/OBS_READ_API.md)); ships in hub Phase 6, designed now to avoid rework.

6. **Compression tiers** — v1: dictionary + TSZ timestamps + Zstd config snapshots + plain floats; v2: ALP for `elapsed_ms`, FastLanes/Gorilla for integers and time columns. Promote v2 only when v1 benchmarks miss targets.

7. **Format versioning** — block-level `FORMAT_VER` with musli-zerocopy incremental validation; incompatible versions fail closed with `store.format_version_unsupported`.

| Do | Do not |
|----|--------|
| Document CHCE in OBS_STORE_MMAP_FORMAT | Keep row-musli mmap v1 as normative |
| Preserve ADR 0025 dual-engine policy | Require mmap on Linux CI |
| Design SSE merge before implementation | Expose SQL or PromQL on store files |

## Consequences

- **Positive:** Single normative columnar design aligned with research; explicit durability trade-off; disk and ingest targets are testable.
- **Negative:** Higher implementation complexity than row-musli v1; custom codec pipeline to maintain; SSE merge needs Grafana integration testing.
- **Related:** [ADR 0025](0025-dual-economics-store-engines.md) dual-engine policy unchanged; format detail in hub spec, not this ADR.

## Related

- [ADR 0025](0025-dual-economics-store-engines.md) · [ADR 0021](0021-rex-owned-economics-store-byot-visualization.md) · [ADR 0026](0026-rex-owned-storage-grafana-otel-datasource.md)
- [OBS_STORE_MMAP_FORMAT.md](../../historical/OBS_STORE_MMAP_FORMAT.md) · [OBSERVABILITY_AND_ECONOMICS.md](../../historical/OBSERVABILITY_AND_ECONOMICS.md)
