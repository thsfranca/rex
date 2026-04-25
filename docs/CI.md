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
