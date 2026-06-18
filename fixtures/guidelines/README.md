# Guidelines conformance fixtures

Machine-readable inputs for [`scripts/ci/run_guidelines_verify.sh`](../../scripts/ci/run_guidelines_verify.sh).

| File | Purpose |
|------|---------|
| [`error_codes.yaml`](error_codes.yaml) | NDJSON stream `error.code` catalog — keep in sync with [docs/ERROR_HANDLING.md](../../docs/ERROR_HANDLING.md) and `StreamErrorCode` in the extension |
| [`broker_error_codes.yaml`](broker_error_codes.yaml) | Broker/policy deny codes — keep in sync with ERROR_HANDLING broker table and `access_policy.rs` |
| [`inference_provider_profiles.yaml`](inference_provider_profiles.yaml) | OpenAI-compat backend presets (design) — keep in sync with [OMLX_INFERENCE.md](../../docs/OMLX_INFERENCE.md) and [ADAPTERS.md](../../docs/ADAPTERS.md) |

Update yaml and the hub catalog table together when adding a stream or broker code.
