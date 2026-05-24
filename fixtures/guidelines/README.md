# Guidelines conformance fixtures

Machine-readable inputs for [`scripts/ci/run_guidelines_verify.sh`](../../scripts/ci/run_guidelines_verify.sh).

| File | Purpose |
|------|---------|
| [`error_codes.yaml`](error_codes.yaml) | NDJSON stream `error.code` catalog — keep in sync with [docs/ERROR_HANDLING.md](../../docs/ERROR_HANDLING.md) and `StreamErrorCode` in the extension |

Update yaml and the hub catalog table together when adding a stream code.
