# Stream event fixtures

Shared **stream event lines** for internal contract tests ([docs/NDJSON_STREAM.md](../../docs/NDJSON_STREAM.md)).

Consumers:

- `crates/rex-stream-ui` — Rust unit/integration checks against golden lines.
- `scripts/ci/guidelines/` — NDJSON terminal and plan contract gates.

| File | Purpose |
|------|---------|
| `happy_path.ndjson` | Chunk + terminal `done` stream |
| `tool_step_stream.ndjson` | Additive `tool` / `step` events + terminal `done` |
| `plan_stream.ndjson` | Additive `plan` events (`clarify`, `ready`) + terminal `done` |
| `sidecar_setup_errors.ndjson` | Terminal `error` codes for RC-08 (`sidecar_unavailable`, `inference_config`) |

Update these together when the wire shape changes.
