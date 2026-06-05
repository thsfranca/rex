# CI quality and security gates (design hub)

This document is the **single source** for post-v1.0 CI gates that harden Rex for AI-assisted development: supply chain, security SAST, Python static analysis on `rex-agent`, and Rex-specific invariant checks. **Shipped gates** are documented in [CI.md](CI.md); remaining backlog item is **R026**.

See [DOCUMENTATION.md](DOCUMENTATION.md) for the feature-area hub convention. [ROADMAP.md](ROADMAP.md) links here; avoid duplicating the phase table elsewhere.

## Purpose

- Close quality and security gaps that native linters do not cover (dependency advisories, security patterns, unlinted Python) without duplicating **clippy** or **ESLint**.
- Preserve [RC-10](V1_0.md) behavior: PR CI stays green without a live LLM; new jobs are additive and path-aware where practical.

AI-assisted changes often introduce contract drift, dependency bumps, and sidecar Python edits. This program tracks gates that catch those failures with high signal and manageable triage cost.

## Status

**shipped** — **R023**, **R024**, **R025**, and **R026** landed (supply chain audit in `rust-verify`; advisory CodeQL in [`.github/workflows/codeql.yml`](../.github/workflows/codeql.yml); Ruff on `rex-agent` in sidecar verify; Rex-specific guideline checks in `guidelines-verify`). Optional **`cargo-deny`** (licenses/bans) deferred to a follow-up slice.

## Scope

**In:**

- Phased gates below, aligned with [PRIORITIZATION.md](PRIORITIZATION.md) (CI cost, blast radius).
- Path-aware CI consistent with [.github/workflows/ci.yml](../.github/workflows/ci.yml) (rust-verify / extension-verify / guidelines-verify model).
- Failure codes: `AUDIT_FAIL`, `SAST_FAIL`, `RUFF_FAIL` (shipped — [CI.md](CI.md)); **R026** guidelines extensions planned.

**Out:**

- **SonarCloud / SonarQube** — overlaps clippy/ESLint; weaker Rust signal; high tuning cost (**Won’t (now)**).
- Global coverage percentage gates.
- Blocking `npm audit` on every PR (extension CI uses `npm ci --no-audit` today; repo-level Dependabot is the preferred path).
- Live LLM or cloud inference in PR CI.

## Boundaries

| Concern | Owner |
|---------|--------|
| Supply chain and security SAST | This program (**R023** Done, **R024** Done — advisory CodeQL) |
| Product contracts (NDJSON error codes, protos) | [scripts/ci/guidelines/](scripts/ci/guidelines/) and [ERROR_HANDLING.md](ERROR_HANDLING.md) — extended in **R026** |
| Broker / access policy product behavior | Daemon tests and [AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md); optional Semgrep rules in **R026** |
| Baseline fmt/clippy/test/ESLint | [CI.md](CI.md) (unchanged) |

## Phased program

```mermaid
flowchart LR
  p1[R023_SupplyChain]
  p2[R024_SAST_CodeQL]
  p3[R025_Ruff_rex_agent]
  p4[R026_Guidelines_Semgrep]
  p1 --> p2
  p1 --> p3
  p2 --> p4
  p3 --> p4
```

| Phase | ID | Bucket | What | Acceptance when Done |
|-------|-----|--------|------|----------------------|
| 1 | **R023** | **Should** | `cargo-audit` (+ optional `cargo-deny` licenses/bans); GitHub **Dependabot** for `Cargo.lock`, `package-lock.json`, pip | **Done** — PR fails on RustSec advisories; [`.github/dependabot.yml`](../.github/dependabot.yml); [DEPENDENCIES.md](DEPENDENCIES.md) |
| 2 | **R024** | **Should** | **CodeQL** workflow (Rust + JS + Python); **advisory** on first land | **Done** — [`.github/workflows/codeql.yml`](../.github/workflows/codeql.yml); triage below; default security queries only |
| 3 | **R025** | **Should** | **Ruff** on [sidecars/rex-agent/](../sidecars/rex-agent/) via [run_rex_agent_checks.sh](../scripts/ci/run_rex_agent_checks.sh) | **Done** — Ruff check in sidecar CI path; dev dep in `pyproject.toml`; `RUFF_FAIL` |
| 4 | **R026** | **Could** | Extend [scripts/ci/guidelines/](../scripts/ci/guidelines/) + optional **Semgrep** for Rex invariants | **Done** — terminal/plan NDJSON + broker policy catalog checks; [test_guidelines_checks.sh](../scripts/ci/test_guidelines_checks.sh) |

### Prioritization (vs peers)

| Item | Bucket | Rank | Rationale |
|------|--------|------|-----------|
| R023 | Should | 1 | Safety, low noise, small blast radius — **Done** |
| R024 | Should | 2 | Security; separate workflow; public GitHub repo enables CodeQL — **Done** |
| R025 | Should | 3 | `rex-agent` growing; CI runs pytest only today — **Done** |
| R026 | Could | 4 | Highest Rex-specific value; needs rule design — **Done** |

May run **in parallel** with **RC-S2** (extension) or **R016** (Could) when CI capacity allows — different blast radii per [PRIORITIZATION.md](PRIORITIZATION.md).

## Won’t (now)

| Tool | Reason |
|------|--------|
| SonarCloud / SonarQube | Duplicates clippy/ESLint; weaker Rust analysis; ongoing quality-gate tuning |

## Interfaces (intent)

Implemented and planned checks follow the [CI observability standard](CI.md#ci-observability-standard-github-native-logs-first): grouped steps, `CI_SIGNAL`, job summary, failure artifacts. Register new low-cardinality codes in [CI.md](CI.md) failure taxonomy.

## Implementation notes

| Phase | Shipped paths |
|-------|----------------|
| R023 | [`.github/dependabot.yml`](../.github/dependabot.yml), [`.cargo/audit.toml`](../.cargo/audit.toml), [`run_rust_supply_chain.sh`](../scripts/ci/run_rust_supply_chain.sh), [`run_rust_verify.sh`](../scripts/ci/run_rust_verify.sh), [DEPENDENCIES.md](DEPENDENCIES.md) |
| R023 tuning | Dependabot groups/cooldown/stagger; audit policy file; `lru >= 0.16.3` (RUSTSEC-2026-0002) |
| R024 | [`.github/workflows/codeql.yml`](../.github/workflows/codeql.yml), [`.github/codeql/codeql-config.yml`](../.github/codeql/codeql-config.yml) |
| R025 | [sidecars/rex-agent/pyproject.toml](../sidecars/rex-agent/pyproject.toml), [run_rex_agent_checks.sh](../scripts/ci/run_rex_agent_checks.sh), [run_sidecar_verify.sh](../scripts/ci/run_sidecar_verify.sh) |

| R026 | [scripts/ci/guidelines/check_ndjson_terminal.sh](../scripts/ci/guidelines/check_ndjson_terminal.sh), [check_ndjson_plan_contract.sh](../scripts/ci/guidelines/check_ndjson_plan_contract.sh), [check_broker_policy_codes.sh](../scripts/ci/guidelines/check_broker_policy_codes.sh), [fixtures/guidelines/broker_error_codes.yaml](../fixtures/guidelines/broker_error_codes.yaml) |

Recommended follow-up: optional **`cargo-deny`** or **Semgrep** rules if CodeQL + guidelines leave gaps. Optional **`cargo-deny`** may land as a small follow-up. Promote CodeQL from advisory to blocking after triage baseline (see below).

## CodeQL triage (R024)

CodeQL runs in a **separate** workflow ([`.github/workflows/codeql.yml`](../.github/workflows/codeql.yml)), not in `ci-checks`. First land is **advisory**: the analyze step uses `continue-on-error: true`, so the workflow may be green while alerts exist.

### Where to view results

- GitHub **Security → Code scanning alerts** (SARIF from `github/codeql-action/analyze`).
- Job summaries link to the Security tab; primary triage happens there, not in `CI_SIGNAL` logs.

### Advisory semantics

- CodeQL is **not** a required branch protection check on first land.
- Default **security** query suite only — not a substitute for clippy, ESLint, or Ruff (R025).
- Path-aware jobs on PR/push use **stricter** gates than functional verify: Rust, JavaScript (extension), and Python (`rex-agent`) analyze only when their `*_changed` paths match — not on `ci_changed` or `global_changed` alone. The **weekly schedule** runs all three jobs for full-repo coverage.

### Triage workflow

1. Open the alert in **Code scanning**; confirm file/line and query rule.
2. **True positive:** fix in a follow-up PR; link the alert when closing.
3. **False positive:** dismiss with reason (won't fix / false positive); note pattern if recurring.
4. **Won't fix (accepted risk):** dismiss with documented rationale for local-only / study-project scope.

### Promotion to blocking (follow-up slice)

After **main** has a stable baseline (no open **high** or **critical** alerts for several weeks, or all remaining alerts dismissed with rationale):

1. Remove `continue-on-error: true` from analyze steps.
2. Optionally add CodeQL jobs to branch protection required checks.
3. Register blocking failures under `SAST_FAIL` in [CI.md](CI.md) observability contract.

## Cross-links

- [CI.md](CI.md) — shipped gates and observability contract
- [ROADMAP.md](ROADMAP.md) — **R023–R026** engineering backlog
- [DEVELOPER_EXPERIENCE_GUIDE.md](DEVELOPER_EXPERIENCE_GUIDE.md) — local checks before PR
- [DEPENDENCIES.md](DEPENDENCIES.md) — toolchain and audit tooling
- [ERROR_HANDLING.md](ERROR_HANDLING.md) — error code catalog (guidelines sync)
- [AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md) — policy invariants for optional Semgrep
- [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md) — **R025** and harness notes
