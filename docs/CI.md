# CI and Branch Protection

This project uses a PR-first CI gate and merge-result validation for `main`.

## CI observability standard (GitHub-native, logs-first)

Use this standard for every CI job in `.github/workflows/ci.yml`.

### Required logging contract

- Use grouped sections in this order:
  - `::group::Setup`
  - `::group::BuildAndChecks`
  - `::group::TestExecution`
  - `::group::PostRunSummary`
- Emit annotations for meaningful signals:
  - `::notice::` for state/progress
  - `::warning::` for non-blocking issues
  - `::error::` for blocking failures
- Emit canonical failure signals for AI and human triage:
  - `CI_SIGNAL code=<CODE> stage=<STAGE> result=<RESULT> hint=<SHORT_HINT>`

### Required summary contract

- Write a concise markdown summary to `$GITHUB_STEP_SUMMARY` in every job.
- Include at least:
  - `result`
  - `fail_stage`
  - `fail_code`
  - `hint`
  - `run_id`
- In gate jobs, include upstream job results explicitly.

### Required artifact contract

- Upload diagnostics on failure with `actions/upload-artifact@v4`.
- Use predictable names:
  - `ci-logs-<job>-<run_id>`
  - `ci-test-report-<job>-<run_id>`
- Set `retention-days: 7` unless project needs change.
- Ensure gate jobs upload diagnostics on failure too (for this workflow: `ci-logs-rust-checks-<run_id>`).

### Workflow script separation contract

- Keep `.github/workflows/*.yml` focused on orchestration (triggers, jobs, permissions, and step order).
- When a check contains many operations, branching, or reusable command groups, move logic to scripts under `scripts/ci/`.
- Call scripts from workflow steps instead of embedding long inline `run` blocks.
- Keep script interfaces explicit with arguments or environment variables.

### Step naming contract

- Use concise, action-first step names that describe only the immediate operation.
  - Good: `Run Tests`, `Install CLI Binaries`, `Write Job Summary`.
  - Avoid: `Run tests with observability signals`, `Install CLI binaries via project script`.
- Do not include chat/session/process context in step names.
- Put extra context in script output, step logs, and docs, not in the step title.
- Keep names stable across edits so AI triage can map failures to the same operation over time.

### Failure taxonomy

Use one shared result vocabulary:

- `success`
- `failure`
- `error`
- `timeout`
- `cancellation`
- `skip`

Keep failure codes low-cardinality. Current baseline set:

- `FMT_FAIL`
- `CLIPPY_FAIL`
- `TEST_FAIL`
- `ENV_SETUP_FAIL`
- `GATE_FAIL`
- `NPM_CI_FAIL` (extension dependency install)
- `TYPECHECK_FAIL` (extension TypeScript typecheck)
- `LINT_FAIL` (extension ESLint)
- `BUILD_FAIL` (extension esbuild bundle)
- `PACKAGE_FAIL` (extension VSIX packaging)

### Reliability guardrails

- Set `timeout-minutes` per job.
- Keep `concurrency.cancel-in-progress: true`.
- Keep `ci-checks` as the single required protection check.
- Keep `rust-checks` and `extension-checks` as domain gate jobs feeding the final gate.

## Workflow triggers

- `pull_request`: runs checks for normal review feedback.
- `merge_group`: runs checks for merge queue validation against latest `main`.

Both events are configured in `.github/workflows/ci.yml`.

## Required status check for `main`

Set branch protection or ruleset on `main` to require:

- `ci-checks`

`ci-checks` is the final gate and fails when `rust-checks` or `extension-checks` fails. `rust-checks` gates Rust checks; `extension-checks` gates extension install/typecheck/lint/test/package. All gate jobs use canonical summary fields (`result`, `fail_stage`, `fail_code`, `hint`, `run_id`) plus upstream outcomes.

## Path-aware execution model

CI first evaluates changed paths, then runs only relevant domain checks.

### Change detection outputs

- `rust_changed`
- `extension_changed`
- `ci_changed`
- `global_changed`
- `rust_relevant`
- `extension_relevant`

### Path relevance defaults

- Rust-relevant:
  - `crates/**`
  - `Cargo.toml`
  - `Cargo.lock`
  - `scripts/install-cli.sh`
  - `scripts/ci/run_rust_*.sh`
  - `scripts/ci/enforce_rust_gate.sh`
- Extension-relevant:
  - `extensions/rex-vscode/**`
  - `scripts/ci/run_extension*.sh`
  - `scripts/ci/enforce_extension_gate.sh`
- Cross-domain triggers:
  - `.github/workflows/**`
  - `scripts/ci/**`
  - `README.md`
  - `docs/**`

### Dependency model

- Rust chain: `rust-fmt-clippy` + `rust-test` -> `rust-checks`
- Extension chain: `extension-lint-typecheck` -> `extension-test` -> `extension-package` -> `extension-checks`
- Top-level chain: `rust-checks` + `extension-checks` -> `ci-checks`

When a domain is non-relevant, leaf jobs skip and that domain gate exits with `result=skip` while returning success. This keeps required checks deterministic for merge queue.

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
4. `./scripts/ci/test_enforce_rust_gate.sh`
5. `cargo test -p rex-daemon --test uds_e2e -- --nocapture`

For lifecycle/race fixes, ensure E2E coverage includes:

- daemon unavailable connection path;
- deterministic startup race recovery path (unavailable -> ready);
- stream interruption/terminal behavior path.

## New CI job checklist

When adding a new CI job, verify all items:

- [ ] Uses the standard group order (`Setup`, `BuildAndChecks`, `TestExecution`, `PostRunSummary`).
- [ ] Emits `::notice::` and `::error::` where relevant.
- [ ] Emits at least one `CI_SIGNAL ...` line on blocking failure.
- [ ] Writes `$GITHUB_STEP_SUMMARY` with required fields.
- [ ] Uploads failure diagnostics as artifact with `retention-days`.
- [ ] Uses standard result vocabulary and low-cardinality failure codes.
- [ ] Sets `timeout-minutes`.
- [ ] Uses `scripts/ci/` for non-trivial check logic instead of long inline `run` blocks.
- [ ] Uses concise, context-free step names (action-first, no process/chat wording).
