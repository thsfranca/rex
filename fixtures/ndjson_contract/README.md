# NDJSON stream contract fixtures

Shared **stdout lines** for `rex-cli complete --format ndjson` ([docs/EXTENSION.md](../../docs/EXTENSION.md) stream contract).

Consumers:

- `crates/rex-cli/tests/ndjson_contract_conformance.rs` — Rust integration checks.
- `extensions/rex-vscode/src/test/ndjson_contract_fixture.test.ts` — extension parser.

Update these together when the wire shape changes.
