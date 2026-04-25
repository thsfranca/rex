## Why

<!-- Why this change is needed. 1-2 sentences. -->

## What Changed

<!-- High-level summary of the most important changes. -->
- 

## Validation Proof Points

<!-- Keep this focused on concrete evidence for merge readiness. -->
- [ ] Runtime lifecycle proof:
  - daemon unavailable path validated
  - startup race recovery validated
  - stream terminal behavior validated
- [ ] CI gate proof:
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
- [ ] I added validation proof points appropriate for this change.
