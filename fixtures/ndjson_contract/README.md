# NDJSON stream contract fixtures

Shared **stdout lines** for `rex-cli complete --format ndjson` ([docs/NDJSON_STREAM.md](../../docs/NDJSON_STREAM.md) stream contract).

Consumers:

- `crates/rex-cli/tests/ndjson_contract_conformance.rs` — Rust integration checks.
- `src/test/ndjson_contract_fixture.test.ts` — extension parser (resolves the file relative to **cwd** `rex CLI`, matching `npm test` and CI).

| File | Purpose |
|------|---------|
| `happy_path.ndjson` | Chunk + terminal `done` stream |
| `tool_step_stream.ndjson` | Additive `tool` / `step` events + terminal `done` |
| `plan_stream.ndjson` | Additive `plan` events (`clarify`, `ready`) + terminal `done` |
| `sidecar_setup_errors.ndjson` | Terminal `error` codes for RC-08 (`sidecar_unavailable`, `inference_config`) |

Update these together when the wire shape changes.
