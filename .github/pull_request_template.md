## Why

<!-- Why this change is needed. 1-2 sentences. -->

## What Changed

<!-- High-level summary of the most important changes. -->
- 

## How To Test

<!-- Add exact verification steps reviewers can run. -->
- [ ] Automated tests:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`
  - `cargo test --workspace --all-targets --locked`
  - `./scripts/ci/test_enforce_rust_gate.sh`
  - `cargo test -p rex-daemon --test uds_e2e -- --nocapture`
- [ ] Manual checks:
  - Start `rex-daemon` and run `rex-cli status`
  - Run `rex-cli complete "hello from rex"` while daemon is available
  - Retry `rex-cli` command during daemon startup and verify recovery once ready

## Validation Proof Points

<!-- Keep this focused on concrete evidence for merge readiness. -->
- [ ] Runtime lifecycle proof:
  - daemon unavailable path validated
  - deterministic startup race recovery validated (unavailable -> ready)
  - stream terminal behavior validated
- [ ] CI gate proof:
  - `./scripts/ci/test_enforce_rust_gate.sh` passes for success/failure contract scenarios
  - `rust-checks` summary includes `result`, `fail_stage`, `fail_code`, `hint`, `run_id`
  - failure diagnostics artifact behavior verified for changed jobs
- [ ] Docs proof:
  - runtime/CI behavior in docs matches implemented behavior

## Risk And Rollback

<!-- Note side effects, migration concerns, and rollback path. -->
- Risk:
- Rollback:

## Checklist

- [ ] I linked the related issue/ticket (or explained why none exists).
- [ ] I kept this PR focused on one primary concern.
- [ ] I updated docs when behavior, setup, or operations changed.
- [ ] I confirmed this change with tests or explicit manual verification.
- [ ] I enabled auto-merge (at PR creation or immediately after).
