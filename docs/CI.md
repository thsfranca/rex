# CI and Branch Protection

This project uses a **pull_request** workflow on GitHub Actions. Required status checks gate merges into `main`.

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
- On failure, append a **Failure excerpt** block (via `scripts/ci/annotate_ci_failure.sh`):
 - Pulls relevant lines from `ci-observability/*.log` using `scripts/ci/extract_log_excerpt.sh`
 - For `UI_FAIL`, prefers `scripts/ci/extract_ui_harness_failure.sh` on `ui-harness.log` (parses `UI_HARNESS_*` lines and failed harness JSON steps)
 - Emits up to 25 `::error title=<fail_code>::` annotations so `gh run view --log-failed` shows the root cause
 - Maps `CI_FAIL_CODE` to the primary log (`test.log`, `clippy.log`, `sidecar-build.log`, etc.)
- Verify scripts call `finish_verify_job.sh` on failure; workflow **Enforce** steps call `enforce_verify_job.sh` (re-print excerpt after `continue-on-error` verify steps).

### Required artifact contract

- Upload diagnostics on failure with `actions/upload-artifact@v4`.
- Use predictable names:
 - `ci-logs-<job>-<run_id>`
 - `ci-test-report-<job>-<run_id>`
- Set `retention-days: 7` unless project needs change.
- Ensure gate jobs upload diagnostics on failure too (for this workflow: `ci-logs-ci-checks-<run_id>`).

### Workflow script separation contract

- Keep `.github/workflows/*.yml` focused on orchestration (triggers, jobs, permissions, and step order).
- When a check contains many operations, branching, or reusable command groups, move logic to scripts under `scripts/ci/`.
- Call scripts from workflow steps instead of embedding long inline `run` blocks.
- Keep script interfaces explicit with arguments or environment variables.

### Step naming contract

- Use concise, action-first step names that describe only the immediate operation.
 - Good: `Run Tests`, `Run Rust verify`, `Write Job Summary`.
 - Avoid: `Run tests with observability signals`, or embedding session/process context in the title.
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
- `GUIDELINES_FAIL` (documented guideline conformance — error code catalog sync and sibling checks under `scripts/ci/guidelines/`)
- `UI_FAIL` (Web UI harness scenarios — build gate on Linux, desktop + daemon on macOS via [`run_ui_verify.sh`](scripts/ci/run_ui_verify.sh))
- `UI_BUILD_FAIL` (rex-web or rex-ui-harness build step in UI verify)
- `SIDECAR_FAIL` (builtin sidecar verify — `rex-sidecar-stub` / `rex-agent`; inner `RUFF_FAIL` from rex-agent ruff check)
- `RUFF_FAIL` (rex-agent Ruff static analysis in sidecar verify)
- `AUDIT_FAIL` (Rust supply chain — `cargo audit` on `Cargo.lock` in `rust-verify`)
- `SAST_FAIL` (CodeQL security SAST — advisory in [`.github/workflows/codeql.yml`](../.github/workflows/codeql.yml); blocking when promoted per [CI_QUALITY_GATES.md](CI_QUALITY_GATES.md#codeql-triage-r024))

### Reliability guardrails

- Set `timeout-minutes` per job.
- **`rust-verify`** uses **mold** (`RUSTFLAGS=-C link-arg=-fuse-ld=mold`), lean **`apt-get install --no-install-recommends`**, **`cargo-nextest`** when available (falls back to `cargo test` in [`run_rust_tests.sh`](scripts/ci/run_rust_tests.sh)), and **`CARGO_TERM_PROGRESS_WHEN=never`** to trim log noise.
- **`ci-checks`** uses **sparse checkout** of `scripts/ci/` only (faster clone; gate only runs shell scripts there).
- Keep `concurrency.cancel-in-progress: true`.
- Keep **`ci-checks`** and **`Conventional PR title`** as required protection checks (see below).
- Do **not** require `rust-verify` — it skips on docs-only PRs and GitHub treats required skipped checks as blocking.

## Workflow triggers

- `pull_request`: runs checks for normal review feedback.

Configured in `.github/workflows/ci.yml`. Checkouts use `fetch-depth: 1` unless a job needs full history. Gate jobs use sparse checkout as noted above.

## Required status checks for `main`

Set branch protection or ruleset on `main` to require:

- `ci-checks`
- `Conventional PR title` (job in [`.github/workflows/pr-title-lint.yml`](../.github/workflows/pr-title-lint.yml))

`ci-checks` is the merge gate for code quality. It reads `rust-verify`, `sidecar-verify`, `guidelines-verify`, and `ui-verify` results. All four verify jobs use path-aware skip semantics: when a domain is not relevant, the verify job is skipped and `ci-checks` still passes. When a relevant verify job fails, `ci-checks` fails with `GATE_FAIL`. When relevant `guidelines-verify` fails, `ci-checks` fails with `GUIDELINES_FAIL`. When relevant `ui-verify` fails, `ci-checks` fails with `UI_FAIL`.

**Conventional PR title** is required because squash-merge titles become commits on `main`, which feed release-plz (semver + changelog). See [CONTRIBUTING.md](../CONTRIBUTING.md).

Do **not** require `rust-verify`, `guidelines-verify`, or `changes` — domain verify jobs skip on docs-only PRs; `changes` is path detection only.

Informational jobs (not required): `rust-verify`, `sidecar-verify`, `guidelines-verify`, `ui-verify`, `changes`, CodeQL (`Analyze (rust)`, `Analyze (python)` in [`.github/workflows/codeql.yml`](../.github/workflows/codeql.yml)).

## Path-aware execution model

CI first evaluates changed paths, then runs only relevant domain checks.

### Change detection outputs

- `rust_verify_changed`
- `sidecar_changed`
- `guidelines_changed`
- `ui_verify_changed`
- `rust_codeql_changed`
- `python_codeql_changed`
- `rust_relevant`
- `sidecar_relevant`
- `guidelines_relevant`
- `ui_relevant`
- `rust_codeql_relevant`
- `python_codeql_relevant`

Canonical path filters live in [`.github/ci-path-filters.yaml`](../.github/ci-path-filters.yaml) and are applied by the [`.github/actions/detect-ci-changes`](../.github/actions/detect-ci-changes/action.yml) composite action. Relevance mapping is in [`scripts/ci/evaluate_ci_relevance.sh`](../scripts/ci/evaluate_ci_relevance.sh).

### Path relevance defaults

- **Rust-relevant** (`rust_verify_changed` only):
 - `crates/**`
 - `proto/**`
 - `Cargo.toml`
 - `Cargo.lock`
- **Sidecar-relevant** (`sidecar_changed` only):

- `sidecars/**`
- `crates/rex-sidecar-stub/**`
- `crates/rex-daemon/src/sidecar_*.rs`
- `crates/rex-daemon/tests/*sidecar*`
- `crates/rex-daemon/tests/agent_scaffold_smoke.rs`
- `crates/rex-daemon/tests/mvp_product_path.rs`
- `proto/rex/sidecar/**`
- `scripts/ci/run_sidecar_verify.sh`, `run_*sidecar*.sh`, `run_rex_agent_checks.sh`, `builtin_sidecars.txt`

### Guidelines verify (path-aware)

When guideline conformance paths change, **guidelines-verify** runs [`scripts/ci/run_guidelines_verify.sh`](scripts/ci/run_guidelines_verify.sh). It validates documented rules that are not covered by fmt or clippy — NDJSON **error code catalog** sync, **terminal event** invariant, **plan** stream contract, and **broker policy** code catalog. Failure code: `GUIDELINES_FAIL`. See [ERROR_HANDLING.md](ERROR_HANDLING.md).

**Guidelines-relevant** (`guidelines_changed` only):

- `fixtures/guidelines/**`
- `fixtures/ndjson_contract/**`
- `docs/ERROR_HANDLING.md`
- `docs/NDJSON_STREAM.md`
- `crates/rex-cli/src/runtime.rs`
- `crates/rex-daemon/src/access_policy.rs`
- `scripts/ci/guidelines/**`
- `scripts/ci/run_guidelines_verify.sh`
- `scripts/ci/test_guidelines_checks.sh`

Local run (any time):

```bash
./scripts/ci/run_guidelines_verify.sh
```

### UI verify (path-aware)

When Web UI probe or harness paths change, CI runs a shared **`ui-build`** job then **`ui-verify`** matrix legs:

1. **`ui-build`** (`ubuntu-latest`) — [`scripts/ci/build_ui_artifacts.sh`](scripts/ci/build_ui_artifacts.sh) builds `apps/rex-web/dist` and `crates/rex-ui-harness/dist` once; uploads artifacts for matrix reuse.
2. **`ui-verify`** matrix — [`scripts/ci/run_ui_verify.sh`](scripts/ci/run_ui_verify.sh) downloads artifacts and runs scenarios:
   - **build** (`ubuntu-latest`) — rex-web production bundle compile gate (`--mode build`); harness build-only step (no browser)
   - **desktop** (`macos-latest`) — reused web/harness dist + `npm ci` for preview/runtime deps; `cargo build -p rex -p rex-desktop --features e2e-testing`; native Playwright harness with probe daemon

Matrix legs pass `--skip-harness-build`; desktop also passes `--skip-web-build`. Local full builds omit skip flags.

Failure code: `UI_FAIL` (scenario failures) or `UI_BUILD_FAIL` (build step). See [WEB_UI_AGENT_VALIDATION.md](WEB_UI_AGENT_VALIDATION.md).

**UI-relevant** (`ui_verify_changed` only):

- `apps/rex-web/**`
- `crates/rex-ui-harness/**`
- `crates/rex-desktop/**`
- `fixtures/ui_probe/**`
- `scripts/ci/run_ui_verify.sh`
- `scripts/ci/build_ui_artifacts.sh`

Local run:

```bash
./scripts/ci/build_ui_artifacts.sh
./scripts/ci/run_ui_verify.sh --mode build --skip-harness-build
./scripts/ci/run_ui_verify.sh --mode desktop --skip-web-build --skip-harness-build   # macOS only
```

Full local build (no artifacts):

```bash
./scripts/ci/run_ui_verify.sh --mode build
./scripts/ci/run_ui_verify.sh --mode desktop   # macOS only
```

### Dependency model

- Path detection: `changes`
- Verify jobs (parallel, path-aware): `rust-verify`, `sidecar-verify`, `guidelines-verify`, `ui-build` → `ui-verify` matrix
- Merge gate: all verify jobs → `ci-checks`

When a domain is non-relevant, its verify job skips. `ci-checks` runs with `if: always` and passes when skipped verify results are acceptable for non-relevant paths.

Docs-only and README-only pull requests skip all verify jobs. `ci-checks` still runs and passes via [`enforce_ci_gate.sh`](../scripts/ci/enforce_ci_gate.sh).

## Release workflows

Release automation is documented in [RELEASE.md](RELEASE.md). Workflows:

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `release-plz.yml` | Push to `main`, `workflow_dispatch` | Open/update core Release PR; tag `v*` and GitHub Release notes on merge |
| `release.yml` | Push version tag, PR (dist plan) | Tag push: `run_rust_verify.sh` + `run_sidecar_verify.sh` in `plan`. PR: **`sidecar-verify`** job, then cargo-dist `build-local-artifacts` |
| `pr-title-lint.yml` | Pull request | Conventional Commits on PR titles |
| `auto-approve.yml` | After `ci` or `PR title lint` completes | Approve PRs from allowlisted authors when required checks pass |

### Auto-approve (trusted authors)

Optional workflow [`.github/workflows/auto-approve.yml`](../.github/workflows/auto-approve.yml) submits an **APPROVE** review from a dedicated bot account when:

- The author is listed in [`.github/auto-approve.json`](../.github/auto-approve.json) `users`, or is an active member of the configured org `team`;
- The PR is not a draft, not from a fork, and not authored by a bot (unless you change policy);
- The PR title does not match a `skip_title_prefixes` entry (release automation PRs by default);
- Required checks (`ci-checks`, `Conventional PR title`) are **success** on the PR head commit.

Configure allowlisted GitHub logins in `.github/auto-approve.json`. The workflow runs from **`main`** ( `workflow_run` semantics); merge config changes to `main` before expecting auto-approve on open PRs.

**GitHub setup (maintainers):**

1. Create a **machine user** or **GitHub App** bot account (not the PR author). Grant it **Read** access to the repository ( **Write** if you later add auto-merge ).
2. Create a classic PAT or app installation token with **`repo`** (private) or **`public_repo`** (public) and, if using org team membership, **`read:org`**.
3. Add repository secret **`REX_AUTO_APPROVE_TOKEN`** with that token (Settings → Secrets and variables → Actions).
4. Add trusted author logins to `.github/auto-approve.json` → `users` (and optionally set `org` + `team`).
5. If you want merges to require this approval, enable **Require pull request reviews before merging** on `main` (branch protection or ruleset). Without that, auto-approve is informational only.

The bot cannot approve its own PRs. Do not use your personal PAT if you are the PR author.

### Release workflow permissions

- **release-plz:** `contents: write`, `pull-requests: write`.
- **release** (cargo-dist): `contents: write` (GitHub Release assets).

### Release failure codes (baseline)

Use the same `CI_SIGNAL` pattern when adding release-specific scripts:

- `RELEASE_BUILD_FAIL` — cargo-dist build failed after verify
- `RELEASE_VERIFY_FAIL` — `run_rust_verify.sh` failed in release plan job

### Maintainer notes

- Re-running `dist generate` recreates `.github/workflows/release.yml`; re-apply the Rust verify step per [RELEASE.md](RELEASE.md).
- Do not enable auto-merge on Release PRs until [V1_0.md](V1_0.md) gates are satisfied for `1.0.0`.

## Why this setup

- Avoids duplicate CI runs from branch `push` + `pull_request` (workflow is PR-scoped).
- Uses shallow clones where safe to reduce checkout time.
- Keeps stable required check names as the protection contract.

## Local MVP preflight (operator path)

Before you run the **manual** desktop operator checklist in [OPERATOR_UX.md](OPERATOR_UX.md) or [MVP_SPEC.md](MVP_SPEC.md), run the same **build + Rust verify + sidecar verify** sequence locally:

```bash
chmod +x ./scripts/verify_mvp_local.sh
./scripts/verify_mvp_local.sh
```

That script runs `cargo build --workspace`, then [`scripts/ci/run_rust_verify.sh`](../scripts/ci/run_rust_verify.sh), then [`scripts/ci/run_sidecar_verify.sh`](../scripts/ci/run_sidecar_verify.sh), then `cargo test -p rex-daemon mvp_product_path`. It does **not** start the desktop app for manual operator steps or **live LLM** dogfood ([MVP_SPEC.md](MVP_SPEC.md)).

**Three tiers:** (1) **Per-PR** — **`mock`** / harness paths in `uds_e2e` and a **loopback OpenAI-compat HTTP fixture** in `mvp_product_path` (real `http_openai_compat`, no cloud API) — **RC-10**. (2) **Planned opt-in + nightly** — live Ollama smoke (**R039**, **R040**) — [ECONOMICS_VALIDATION.md](ECONOMICS_VALIDATION.md). (3) **Operator dogfood** — desktop manual path ([OPERATOR_UX.md](OPERATOR_UX.md)) and rex-ui-harness ([WEB_UI_AGENT_VALIDATION.md](WEB_UI_AGENT_VALIDATION.md)); live native tool scripts remain blocked pending harness rewrite.

**Builtin sidecars:** [`run_sidecar_verify.sh`](../scripts/ci/run_sidecar_verify.sh) runs in the **`sidecar-verify`** job (path-aware) and in **`Release / sidecar-verify`** on pull requests. Tag pushes run `run_rust_verify.sh` then `run_sidecar_verify.sh` in the Release `plan` job. A sidecar failure fails **`sidecar-verify`** (or the Release sidecar job) and, when sidecar paths are relevant, **`ci-checks`** (`GATE_FAIL`). Manifest: [`builtin_sidecars.txt`](../scripts/ci/builtin_sidecars.txt).

| Stage | Script | Content |
|-------|--------|---------|
| Setup | `run_sidecar_verify.sh` | pip deps, `rex proto install` |
| BuildAndChecks | `run_sidecar_verify.sh` | `cargo build -p rex-sidecar-stub -p rex` |
| TestExecution | [`run_stub_sidecar_checks.sh`](../scripts/ci/run_stub_sidecar_checks.sh) | `cargo test -p rex-sidecar-stub`; UDS [`stub_sidecar_smoke`](../crates/rex-daemon/tests/stub_sidecar_smoke.rs) |
| TestExecution | [`run_rex_agent_checks.sh`](../scripts/ci/run_rex_agent_checks.sh) | `ruff check`; `pytest`; UDS [`agent_scaffold_smoke`](../crates/rex-daemon/tests/agent_scaffold_smoke.rs) |

Local: `./scripts/ci/run_sidecar_verify.sh`. Failure code: `SIDECAR_FAIL`. Requires Python 3.10+ (`python3.11` / `python3.10` preferred).

### Rust supply chain audit (`rust-verify`)

[`run_rust_supply_chain.sh`](../scripts/ci/run_rust_supply_chain.sh) runs in **BuildAndChecks** inside [`run_rust_verify.sh`](../scripts/ci/run_rust_verify.sh) (after fmt/clippy, before tests). It scans `Cargo.lock` with `cargo audit`, using repo policy in [`.cargo/audit.toml`](../.cargo/audit.toml) (fail on advisories and **yanked** crates). Failure code: `AUDIT_FAIL`.

Local:

```bash
./scripts/ci/run_rust_supply_chain.sh
```

Requires `cargo-audit` on `PATH` (see [DEPENDENCIES.md](DEPENDENCIES.md)). CI installs `cargo-audit@0.22` via `taiki-e/install-action` in the **`rust-verify`** job.

Dependabot (cargo, npm, pip) is configured in [`.github/dependabot.yml`](../.github/dependabot.yml): grouped weekly updates on staggered days, cooldown on npm/pip version bumps (security updates are immediate). Triage bot PRs separately from this gate.

## Local verification flow for reliability changes

Run this sequence before opening PRs that change stream lifecycle behavior:

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets --locked -- -D warnings`
3. `cargo test --workspace --all-targets --locked` (optional: `cargo install cargo-nextest` then `cargo nextest run --workspace --all-targets --locked` to mirror CI)
4. `./scripts/ci/test_enforce_ci_gate.sh`
5. `cargo test -p rex-daemon --test uds_e2e -- --nocapture`

For lifecycle/race fixes, ensure E2E coverage includes:

- daemon unavailable connection path;
- deterministic startup race recovery path (unavailable -> ready);
- stream interruption/terminal behavior path.

## CodeQL security SAST (`codeql.yml`)

**R024** ships an **advisory** CodeQL workflow separate from `ci-checks`. Triggers: `pull_request`, push to `main`, weekly schedule.

On **pull_request** and **push**, CodeQL uses **source-only** path gates (stricter than functional verify): each language runs only when its `*_codeql_changed` filter matches — not on CI scripts, manifests, or workflow edits alone. The **weekly schedule** runs both language jobs regardless of path filters (full-repo backstop).

| Job | PR/push gate | Schedule | Build trace |
|-----|--------------|----------|-------------|
| `Analyze (rust)` | `rust_codeql_changed` (`crates/**`, `proto/**`) | always | `cargo build --workspace --locked` (protoc + mold, same as `rust-verify`) |
| `Analyze (python)` | `python_codeql_changed` (`sidecars/**`) | always | `pip install -e sidecars/rex-agent/[dev]` |

Relevance outputs: `rust_codeql_relevant`, `python_codeql_relevant` from [`evaluate_ci_relevance.sh`](../scripts/ci/evaluate_ci_relevance.sh).

Path exclusions: [`.github/codeql/codeql-config.yml`](../.github/codeql/codeql-config.yml). Triage and promotion to blocking: [CI_QUALITY_GATES.md](CI_QUALITY_GATES.md#codeql-triage-r024). Failure code when blocking: `SAST_FAIL`.

## Planned quality and security gates

**R023** (supply chain: `cargo audit`, Dependabot) is **shipped** in `rust-verify`. **R024** (CodeQL) is **shipped** advisory in [`.github/workflows/codeql.yml`](../.github/workflows/codeql.yml). **R025** (Ruff on `rex-agent`) is **shipped** in sidecar verify. **R026** (NDJSON terminal/plan contract + broker policy catalog sync) is **shipped** in `guidelines-verify` — see [CI_QUALITY_GATES.md](CI_QUALITY_GATES.md).

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

Release workflows additionally:

- [ ] Core release runs `run_rust_verify.sh` and `run_sidecar_verify.sh` before dist build on tag pushes; PRs run **`sidecar-verify`** before `build-local-artifacts`.
