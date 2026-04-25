# CI and Branch Protection

This project uses a PR-first CI gate and merge-result validation for `main`.

## Workflow triggers

- `pull_request`: runs checks for normal review feedback.
- `merge_group`: runs checks for merge queue validation against latest `main`.

Both events are configured in `.github/workflows/ci.yml`.

## Required status check for `main`

Set branch protection or ruleset on `main` to require:

- `rust-checks`

`rust-checks` is the final gate job and fails when any required upstream CI job fails.

## Merge queue settings

In GitHub repository settings for `main`:

1. Enable **Require merge queue**.
2. Keep **Only merge non-failing pull requests** enabled.
3. Keep build concurrency low for this project size (start with `1`).

## Why this setup

- Avoids duplicate CI runs from branch `push` + `pull_request`.
- Validates merge results before integration into `main`.
- Keeps one stable required check name as the protection contract.

## Local verification flow for reliability changes

Run this sequence before opening PRs that change stream lifecycle behavior:

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets --locked -- -D warnings`
3. `cargo test --workspace --all-targets --locked`

For lifecycle/race fixes, ensure E2E coverage includes:

- daemon unavailable connection path;
- startup race recovery path;
- stream interruption/terminal behavior path.
