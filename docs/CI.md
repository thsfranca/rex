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
- `NPM_CI_FAIL` (extension dependency install)
- `TYPECHECK_FAIL` (extension TypeScript typecheck)
- `LINT_FAIL` (extension ESLint)
- `BUILD_FAIL` (extension esbuild bundle)
- `PACKAGE_FAIL` (extension VSIX packaging)
- `GUIDELINES_FAIL` (documented guideline conformance — error code catalog sync and sibling checks under `scripts/ci/guidelines/`)
- `SIDECAR_FAIL` (builtin sidecar verify — `rex-sidecar-stub` / `rex-agent`)
- `AUDIT_FAIL` (Rust supply chain — `cargo audit` on `Cargo.lock` in `rust-verify`)
- `SAST_FAIL` (CodeQL security SAST — advisory in [`.github/workflows/codeql.yml`](../.github/workflows/codeql.yml); blocking when promoted per [CI_QUALITY_GATES.md](CI_QUALITY_GATES.md#codeql-triage-r024))

### Reliability guardrails

- Set `timeout-minutes` per job.
- **`rust-verify`** uses **mold** (`RUSTFLAGS=-C link-arg=-fuse-ld=mold`), lean **`apt-get install --no-install-recommends`**, **`cargo-nextest`** when available (falls back to `cargo test` in [`run_rust_tests.sh`](scripts/ci/run_rust_tests.sh)), and **`CARGO_TERM_PROGRESS_WHEN=never`** to trim log noise.
- **`extension-verify`** sets **`NODE_OPTIONS=--max-old-space-size=6144`** and **`CI=true`** so Vitest can use full CPU (`maxWorkers` in [`vitest.config.ts`](extensions/rex-vscode/vitest.config.ts)).
- **`ci-checks`** uses **sparse checkout** of `scripts/ci/` only (faster clone; gate only runs shell scripts there).
- Keep `concurrency.cancel-in-progress: true`.
- Keep **`ci-checks`** and **`Conventional PR title`** as required protection checks (see below).
- Do **not** require `rust-verify` or `extension-verify` — they skip on docs-only PRs and GitHub treats required skipped checks as blocking.

## Workflow triggers

- `pull_request`: runs checks for normal review feedback.

Configured in `.github/workflows/ci.yml`. Checkouts use `fetch-depth: 1` unless a job needs full history. Gate jobs use sparse checkout as noted above.

## Required status checks for `main`

Set branch protection or ruleset on `main` to require:

- `ci-checks`
- `Conventional PR title` (job in [`.github/workflows/pr-title-lint.yml`](../.github/workflows/pr-title-lint.yml))

`ci-checks` is the merge gate for code quality. It reads `rust-verify`, `sidecar-verify`, `extension-verify`, and `guidelines-verify` results. **`guidelines-verify`** always runs documented guideline conformance checks. `rust-verify`, `sidecar-verify`, and `extension-verify` use path-aware skip semantics: when a domain is not relevant, the verify job is skipped and `ci-checks` still passes. When a relevant verify job fails, `ci-checks` fails with `GATE_FAIL`. When `guidelines-verify` fails, `ci-checks` fails with `GUIDELINES_FAIL`.

**Conventional PR title** is required because squash-merge titles become commits on `main`, which feed release-plz and release-please (semver + changelog). See [CONTRIBUTING.md](../CONTRIBUTING.md).

Do **not** require `rust-verify`, `extension-verify`, `guidelines-verify`, or `changes` — domain verify jobs skip on docs-only PRs; `changes` is path detection only.

Informational jobs (not required): `rust-verify`, `sidecar-verify`, `extension-verify`, `guidelines-verify`, `changes`, CodeQL (`Analyze (rust)`, `Analyze (javascript)`, `Analyze (python)` in [`.github/workflows/codeql.yml`](../.github/workflows/codeql.yml)).

## Path-aware execution model

CI first evaluates changed paths, then runs only relevant domain checks.

### Change detection outputs

- `rust_changed`
- `extension_changed`
- `ci_changed`
- `global_changed`
- `sidecar_changed`
- `rust_relevant`
- `extension_relevant`
- `sidecar_relevant`

### Path relevance defaults

- Rust-relevant:
  - `crates/**`
  - `sidecars/rex-agent/**`
  - `proto/**`
  - `Cargo.toml`
  - `Cargo.lock`
  - `scripts/install-cli.sh`
  - `scripts/ci/run_rust_*.sh`
  - `scripts/ci/run_sidecar_verify.sh`
  - `scripts/ci/run_stub_sidecar_checks.sh`
  - `scripts/ci/run_rex_agent_checks.sh`
  - `scripts/ci/builtin_sidecars.txt`
- Extension-relevant:
  - `extensions/rex-vscode/**`
  - `scripts/ci/run_extension*.sh`
- Cross-domain triggers:
  - `.github/workflows/**`
  - `scripts/ci/**`
  - `Cargo.toml`
  - `Cargo.lock`
  - `docs/ERROR_HANDLING.md`
  - `fixtures/guidelines/**`
  - `fixtures/ndjson_contract/**`

### Guidelines verify (always runs)

Every pull request runs [`scripts/ci/run_guidelines_verify.sh`](scripts/ci/run_guidelines_verify.sh) in the **guidelines-verify** job. It validates documented rules that are not covered by fmt, clippy, or ESLint — starting with NDJSON stream **error code catalog** sync (`fixtures/guidelines/error_codes.yaml` ↔ TypeScript ↔ docs ↔ fixtures). Failure code: `GUIDELINES_FAIL`. See [ERROR_HANDLING.md](ERROR_HANDLING.md).

Local run:

```bash
./scripts/ci/run_guidelines_verify.sh
```

### Sidecar-relevant paths

- `sidecars/**`
- `crates/rex-sidecar-stub/**`
- `crates/rex-daemon/tests/stub_sidecar_smoke.rs`
- `crates/rex-daemon/tests/agent_scaffold_smoke.rs`
- `proto/rex/sidecar/**`
- `scripts/ci/run_sidecar_verify.sh`, `run_*sidecar*.sh`, `builtin_sidecars.txt`
- Cross-domain: `.github/workflows/**`, `scripts/ci/**`, `Cargo.toml`, `Cargo.lock` (same as other verify jobs)

### Dependency model

- Always: `guidelines-verify`
- Verify jobs (parallel, path-aware): `rust-verify`, `sidecar-verify`, `extension-verify`
- Merge gate: `guidelines-verify` + `rust-verify` + `sidecar-verify` + `extension-verify` → `ci-checks`

When a domain is non-relevant, its verify job skips. `ci-checks` runs with `if: always()` and passes when skipped verify results are acceptable for non-relevant paths.

Docs-only and README-only pull requests skip both verify jobs. `ci-checks` still runs and passes via [`enforce_ci_gate.sh`](../scripts/ci/enforce_ci_gate.sh).

## Release workflows

Release automation is documented in [RELEASE.md](RELEASE.md). Workflows:

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `release-plz.yml` | Push to `main`, `workflow_dispatch` | Open/update core Release PR; tag `v*` and GitHub Release notes on merge |
| `release.yml` | Push version tag, PR (dist plan) | Tag push: `run_rust_verify.sh` + `run_sidecar_verify.sh` in `plan`. PR: **`sidecar-verify`** job, then cargo-dist `build-local-artifacts` |
| `release-please-extension.yml` | Push to `main`, `workflow_dispatch` | Open/update extension Release PR; tag `rex-vscode-v*` on merge |
| `extension-release.yml` | Push tag `rex-vscode-v*`, `workflow_dispatch` | Build VSIX and optional marketplace publish |
| `pr-title-lint.yml` | Pull request | Conventional Commits on PR titles |

### Release workflow permissions

- **release-plz** and **release-please-extension:** `contents: write`, `pull-requests: write`.
- **release** (cargo-dist) and **extension-release:** `contents: write` (GitHub Release assets).

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

Before you run the **manual** editor checklist in [EXTENSION_LOCAL_E2E.md](EXTENSION_LOCAL_E2E.md) or [MVP_SPEC.md](MVP_SPEC.md), run the same **build + Rust verify + extension verify** sequence locally:

```bash
chmod +x ./scripts/verify_mvp_local.sh
./scripts/verify_mvp_local.sh
```

That script runs `cargo build --workspace`, then [`scripts/ci/run_rust_verify.sh`](../scripts/ci/run_rust_verify.sh), then [`scripts/ci/run_sidecar_verify.sh`](../scripts/ci/run_sidecar_verify.sh), then [`scripts/ci/run_extension_checks.sh`](../scripts/ci/run_extension_checks.sh), then `cargo test -p rex-daemon mvp_product_path`. It does **not** start `rex-daemon` for manual editor steps or **live LLM** dogfood ([MVP_SPEC.md](MVP_SPEC.md)).

**Two tiers:** PR CI uses **`mock`** / harness paths in `uds_e2e` and a **loopback OpenAI-compat HTTP fixture** in `mvp_product_path` (real `http_openai_compat`, no cloud API). Operator dogfood requires **live** JSON `inference.openai_compat` (Ollama, LM Studio, etc.).

**Builtin sidecars:** [`run_sidecar_verify.sh`](../scripts/ci/run_sidecar_verify.sh) runs in the **`sidecar-verify`** job (path-aware) and in **`Release / sidecar-verify`** on pull requests. Tag pushes run `run_rust_verify.sh` then `run_sidecar_verify.sh` in the Release `plan` job. A sidecar failure fails **`sidecar-verify`** (or the Release sidecar job) and, when sidecar paths are relevant, **`ci-checks`** (`GATE_FAIL`). Manifest: [`builtin_sidecars.txt`](../scripts/ci/builtin_sidecars.txt).

| Stage | Script | Content |
|-------|--------|---------|
| Setup | `run_sidecar_verify.sh` | pip deps, `rex proto install` |
| BuildAndChecks | `run_sidecar_verify.sh` | `cargo build -p rex-sidecar-stub -p rex` |
| TestExecution | [`run_stub_sidecar_checks.sh`](../scripts/ci/run_stub_sidecar_checks.sh) | `cargo test -p rex-sidecar-stub`; UDS [`stub_sidecar_smoke`](../crates/rex-daemon/tests/stub_sidecar_smoke.rs) |
| TestExecution | [`run_rex_agent_checks.sh`](../scripts/ci/run_rex_agent_checks.sh) | `pytest`; UDS [`agent_scaffold_smoke`](../crates/rex-daemon/tests/agent_scaffold_smoke.rs) |

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

| Job | When | Build trace |
|-----|------|-------------|
| `Analyze (rust)` | Rust-relevant paths | `cargo build --workspace --locked` (protoc + mold, same as `rust-verify`) |
| `Analyze (javascript)` | Extension-relevant paths | `npm ci` + `npm run build` in `extensions/rex-vscode/` |
| `Analyze (python)` | Sidecar-relevant paths | `pip install -e sidecars/rex-agent/[dev]` |

Path exclusions: [`.github/codeql/codeql-config.yml`](../.github/codeql/codeql-config.yml). Triage and promotion to blocking: [CI_QUALITY_GATES.md](CI_QUALITY_GATES.md#codeql-triage-r024). Failure code when blocking: `SAST_FAIL`.

## Planned quality and security gates

**R023** (supply chain: `cargo audit`, Dependabot) is **shipped** in `rust-verify`. **R024** (CodeQL) is **shipped** advisory in [`.github/workflows/codeql.yml`](../.github/workflows/codeql.yml) — see [CI_QUALITY_GATES.md](CI_QUALITY_GATES.md). Remaining phases **R025–R026** (Ruff on `rex-agent`, Rex-specific guidelines) are **not** in CI yet.

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

- [ ] Tag filters exclude the other plane (`v*` vs `rex-vscode-v*`).
- [ ] Core release runs `run_rust_verify.sh` and `run_sidecar_verify.sh` before dist build on tag pushes; PRs run **`sidecar-verify`** before `build-local-artifacts`.
