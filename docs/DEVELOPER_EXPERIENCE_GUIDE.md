# Developer Experience Guide For REX


> Role: how-to | Status: active | Audience: contributors | Read when: contributor workflow
> Prefer: ## 1) Project purpose

This guide defines how to develop REX with AI assistance and how to design plugin-facing changes without breaking contracts.

## 1) Project purpose and architecture in 2 minutes

Canonical **purpose and principles**: [PURPOSE_AND_PRINCIPLES.md](PURPOSE_AND_PRINCIPLES.md).

**Architecture policies and ownership** (how docs and ADRs relate, `rex.v1` vs sidecar plane): [ARCHITECTURE_GUIDELINES.md](ARCHITECTURE_GUIDELINES.md).

REX provides a local AI runtime with one daemon as the **system authority** for **streaming contracts, adapter policy, caches, pipelines, and the agent/economics roadmap** ([ADR 0001](architecture/decisions/0001-daemon-owns-agent-orchestration-and-economics.md)). Isolated **agent runtime environments** (when implemented) remain **supervised and policy-bound** to the daemon—see [ADR 0005](architecture/decisions/0005-rex-owns-sidecar-environment-not-agent-implementations.md). **Sidecar ↔ daemon** integration uses a **dedicated brokered API**, not **`rex.v1`** — [ADR 0008](architecture/decisions/0008-dedicated-sidecar-control-plane-api.md).

The interactive **desktop web UI** (bare **`rex`**) is the product usage path. The product shell is **Electron** at `apps/rex-desktop` ([ADR 0043](architecture/decisions/0043-electron-shell-for-electric-alive-compositor.md)). Daemon boundary and UDS `rex.v1` are unchanged ([ADR 0042](architecture/decisions/0042-web-desktop-presentation-pivot.md) pivot).

| Component | Responsibility |
|---|---|
| `rex` | Unified CLI: bare entry opens the desktop app (product); setup/doctor commands. |
| `apps/rex-desktop` | Electron shell: loads `apps/rex-web`; compositor proof; UDS proxy in main (W127). |
| `apps/rex-web` | React presentation client (transcript, composer, timeline, Electric Alive). |
| `rex-daemon` | Model/agent policy, adapters, caches, **`StreamInference`** lifecycle. |
| `rex-proto` | `rex.v1` gRPC contract. |
| `rex-config` | JSON config load/merge (`$REX_ROOT/config.json`). |
| `rex-sidecar-stub` / `rex-agent` | Harness and product sidecars ([SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md)). |

Architecture intent:

- Stable protocol boundary across clients.
- Default local UDS transport.
- Product logic migrates toward daemon-first economics ([CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md)).

## 2) Setup and working modes

### Local setup

Prerequisites:
- Rust toolchain
- `protoc` (see `docs/DEPENDENCIES.md`)

Core commands:

```bash
rex config init
# Edit $REX_ROOT/config.json — inference.openai_compat + sidecars.active=agent (binary rex-agent)
rex config validate
cargo build --workspace
make run        # rebuild web UI + rex, launch desktop (macOS)
# Or manually: cd apps/rex-web && npm install && npm run build; cargo build -p rex; rex
```

**Current focus:** Web UI program **W125–W129** Electron migration — [WEB_UI_DESIGN.md](WEB_UI_DESIGN.md), [WEB_UI_ROADMAP.md](WEB_UI_ROADMAP.md), [ADR 0043](architecture/decisions/0043-electron-shell-for-electric-alive-compositor.md). Running **`rex`** opens the desktop app and ensures the daemon (Tauri until Electron launch lands). Install with **`./scripts/install-cli.sh`**. Agent UI validation: **rex-ui-harness** MCP — `./scripts/setup_ui_probe_env.sh`, [fixtures/ui_probe/README.md](../fixtures/ui_probe/README.md).

The Phase 1 product path requires a **supervised sidecar** for assistant modes — [MVP_SPEC.md](MVP_SPEC.md), [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md). Configure **`sidecars`** and **`inference.openai_compat`** in JSON ([CONFIGURATION.md](CONFIGURATION.md)); legacy `REX_*` tuning env vars are ignored. CI may use `sidecars.harness: "direct"` (harness only).

### Multi-provider (Anthropic, OpenAI, Ollama) via Inference Gateway

**Default API:** OpenAI-compat toward LiteLLM. **Opt-in managed:** set `inference.gateway.mode: managed` in `$REX_ROOT/config.json` so Rex starts and controls the gateway (not a sidecar). Hub: [INFERENCE_GATEWAY.md](INFERENCE_GATEWAY.md).

**External gateway (today):** point `inference.openai_compat` at your LiteLLM URL in `$REX_ROOT/config.json`; keys in LiteLLM, not Rex.

```json
{
 "inference": {
 "runtime": "http-openai-compat",
 "openai_compat": {
 "base_url": "http://127.0.0.1:4000/v1",
 "model": "claude-sonnet-4-20250514"
 }
 },
 "sidecars": { "active": "stub", "required": true }
}
```

Product settings are **JSON only**; the sole product env var is **`REX_ROOT`** — [CONFIGURATION.md](CONFIGURATION.md).

**Managed gateway (`inference.gateway.mode: managed`):** `rex gateway init`, set keys in `$REX_ROOT/gateway/.env`, `rex gateway doctor`, then `rex` — [CONFIGURATION.md](CONFIGURATION.md#inference-gateway-design).

### Mac local MLX via managed oMLX

**Opt-in managed:** set `inference.omlx.mode: managed` on Apple Silicon so Rex starts oMLX during daemon boot (including R071 autostart). Hub: [OMLX_INFERENCE.md](OMLX_INFERENCE.md).

```json
{
 "inference": {
 "runtime": "http-openai-compat",
 "omlx": {
 "mode": "managed",
 "port": 8000,
 "model": "your-mlx-model-id"
 },
 "openai_compat": {
 "native_tools": "auto"
 }
 },
 "daemon": {
 "ready_timeout_secs": 45
 },
 "sidecars": { "active": "agent", "required": true }
}
```

**Operator path:** install oMLX → `rex omlx init` (merge `$REX_ROOT/omlx/config.snippet.json` into config) → `rex omlx doctor` → `rex` (desktop).

**Observability:** set `observability.enabled: true` and configure OTLP toward **LangFuse Cloud** when **LF-F01** lands — [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md), [CONFIGURATION.md](CONFIGURATION.md#observability). Rex-owned store and `rex obs` were **removed** (**LF-R01**). Only **`REX_ROOT`** is a bootstrap env var for layout.

### Working modes

Use lightweight modes instead of rigid process:

| Mode | Primary goal | Exit criterion |
|---|---|---|
| Exploration | Understand constraints and boundaries | You can point to exact files/contracts to change |
| Implementation | Make one focused behavior change | Build and tests pass for changed scope |
| Verification | Confirm behavior and regressions | Local checks and manual validation pass |

### Human and AI responsibilities

Human responsibilities:
- Set scope and accept/reject trade-offs.
- Approve contract changes and release-impacting decisions.
- Confirm final validation before merge.

AI responsibilities:
- Draft focused code/doc updates.
- Keep changes aligned with existing contracts and rules.
- Apply [KISS config surface](KISS_CONFIG_SURFACE.md) before adding JSON keys or dual product/harness modes.
- Produce explicit verification steps.
- Follow [ADR 0006](architecture/decisions/0006-gitignored-temp-tracking-sequential-handoffs.md) for repo-touching plans: keep a gitignored root `TEMP_*.md` record and, for sequential tasks, include **handoff instructions** (**purpose of the whole slice**, preferred response formats, and structure) for the next session.

### Common failures and recovery

| Symptom | Likely cause | Recovery |
|---|---|---|
| Desktop cannot connect | Daemon not running yet | Run `rex` again; daemon auto-starts |
| Bind fails on `/tmp/rex.sock` | Stale socket file | Stop daemon cleanly; remove stale socket and restart |
| Protobuf build error | `protoc` missing or incompatible | Reinstall prerequisites from `docs/DEPENDENCIES.md` |
| Installed command not found | PATH not updated after install | Re-run install script and reload shell |
| `requires-python >=3.10` on rex-agent install | macOS CLT Python 3.9 or wrong `pip` on PATH | `brew install python@3.12`; `./scripts/install-agent-sidecar.sh`; `rex sidecar doctor` |
| PEP 668 / externally-managed pip | System Python blocked global install | Use `$REX_ROOT/venv` via `./scripts/install-agent-sidecar.sh` |

## 3) Development workflow and quality gates

Required local checks before PR:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo audit
cargo test --workspace --all-targets --locked
```

Or run the full Rust CI path (includes audit):

```bash
./scripts/ci/run_rust_verify.sh
```

Use this review checklist before accepting AI-generated changes:
- Scope: one primary concern per PR.
- KISS: simpler config/runtime option stated before new keys ([KISS_CONFIG_SURFACE.md](KISS_CONFIG_SURFACE.md)).
- Contract safety: no accidental `rex.v1` breaking changes.
- Reliability: failure paths are explicit and testable; error messages follow [ERROR_HANDLING.md](ERROR_HANDLING.md).
- Documentation: setup/behavior changes are documented.
- Verification: command-level or test-level proof is present.

Optional before PR (error codes, fixtures, or guidelines scripts):

```bash
./scripts/ci/run_guidelines_verify.sh
```

Post-v1.0 gates: supply chain audit (**R023**) runs in `./scripts/ci/run_rust_verify.sh`; CodeQL (**R024**) runs in GitHub Actions only ([`.github/workflows/codeql.yml`](../.github/workflows/codeql.yml)); Ruff on `rex-agent` (**R025**) runs in `./scripts/ci/run_sidecar_verify.sh`. Remaining **R026** is tracked in [CI_QUALITY_GATES.md](CI_QUALITY_GATES.md). Optional local Ruff before PR:

```bash
python3 -m pip install "ruff>=0.8"
python3 -m ruff check sidecars/rex-agent/src sidecars/rex-agent/tests
```

### Agent context budget (AI assistants)

When using AI tools on this repo:

- Start from [AGENTS.md](AGENTS.md) or root [llms.txt](../llms.txt) — load **one task route**, not the full [README.md](README.md) index.
- Machine catalog: [manifest.yaml](manifest.yaml) (`role`, `status`, `summary_heading`, `advisory_bundle`).
- Cancelled programs live under [historical/](historical/) — exclude from default retrieval.
- Keep editor **alwaysApply** rules minimal; use agent-requestable rules for doc-authoring norms. Product policy lives in `docs/`, not duplicated in rules.

Daemon injection (when running `rex` in a workspace with `docs/manifest.yaml`):

- **R066** `context.paths[]` — bounded pre-inject paths; see [PROJECT_CONTEXT_PATHS.md](PROJECT_CONTEXT_PATHS.md).
- **R067** advisory bundle — short prompts matching advisory intent load manifest `advisory_bundle` paths.
- **KnowledgeRetrieval v1** — hub `## Summary` slices under 15% context budget; pointer in [AGENTS.md](AGENTS.md#bundle-pointer-knowledgeretrieval).

### Documentation conventions

Major product features use **one canonical hub doc** under `docs/` plus **pointers** in roadmaps and overviews—not parallel full design lists across files. Rules and hubs live in [DOCUMENTATION.md](DOCUMENTATION.md).

CI and PR expectations:
- Commit and PR title format: [CONTRIBUTING.md](../CONTRIBUTING.md) (Conventional Commits for release automation).
- CI gate behavior follows `docs/CI.md`.
- PR description follows `.github/pull_request_template.md` and stays about this repository and the change (no required coupling to external trackers for now).
- When documentation for a behavior change lives **only** in another open pull request, add **`Documentation:`** under **Summary** with a link to that PR (`#NNN` or full URL). Keep the docs checklist item accurate; auto-merge stays off until the checklist is complete. The same expectation is summarized in [DOCUMENTATION.md](DOCUMENTATION.md) under **Pull request descriptions**.
- **Pre-open slice gate:** before push or opening a PR, compare `git diff origin/main...HEAD` to the **current slice only**. Multi-PR plans use one branch per slice; do not open the next PR while a prior slice is still open. If the diff bundles multiple planned slices, split before opening or record an explicit user waiver in **Summary** (`PR slice: waived — <reason>`). For series work, keep gitignored `TEMP_*.md` **PR series** fields current ([ADR 0006](architecture/decisions/0006-gitignored-temp-tracking-sequential-handoffs.md)).

## 4) Extensibility developer track

**Stable today**

- [`proto/rex/v1/rex.proto`](../proto/rex/v1/rex.proto) unary + streaming RPC semantics.
- `InferenceRuntime` + capability negotiation ([ADAPTERS.md](ADAPTERS.md)).
- Daemon-owned L1/cache hooks ([CACHING.md](CACHING.md)).

**Near roadmap**

- Supervised sidecar **implemented** — [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md), [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md) (harness default **`rex-sidecar-stub`**; product **`rex-agent`** shipped).

**Later**

- Rich multi-plugin fleets, Wasm bridges—only after sidecar MVP evidence.

## 5) Contract and compatibility policy

Public API boundary for this project:
- `proto/rex/v1/rex.proto` and generated `rex.v1` surfaces

Allowed by default (same major version):
- Additive fields/RPCs with backward-compatible defaults.
- New optional behavior that does not break old clients.

Disallowed without major version change:
- Removing or renaming existing fields/RPCs.
- Reusing removed protobuf field numbers.
- Changing behavior in ways that break reasonable existing clients.

Deprecation policy:
- Mark APIs as deprecated before removal.
- Keep at least one transition period with clear migration direction.

References:
- [Semantic Versioning 2.0.0](https://semver.org/)
- [Google AIP-180 Backwards compatibility](https://google.aip.dev/180)
- [Protocol Buffers: Updating a Message Type](https://protobuf.dev/programming-guides/proto3/#updating)

## 6) Testing strategy by risk

| Change type | Minimum tests |
|---|---|
| CLI formatting or text-only output | Targeted unit tests or command verification |
| Daemon behavior/path changes | Unit tests + relevant integration path |
| Transport/stream lifecycle changes | Unit + E2E (`crates/rex-daemon/tests/uds_e2e.rs`) |
| Contract/proto evolution | Regenerate/build + compatibility-focused integration checks |

Testing principles:
- Prefer meaningful behavior checks over line-only coverage.
- Add E2E tests for startup race, daemon unavailable, and stream terminal behavior when affected.

## 7) Security and operational baseline

Security baseline:
- Keep transport local through UDS by default.
- Use filesystem/socket permissions to limit access.
- Avoid remote listeners unless explicitly introduced by spec.

Operational baseline:
- Keep daemon lifecycle deterministic (startup, shutdown, cleanup).
- Keep failure messages actionable for CLI and logs — see [ERROR_HANDLING.md](ERROR_HANDLING.md).
- Preserve CI observability conventions from `docs/CI.md`.

## 8) Roadmap to extensibility-ready DX

**Daemon-first prerequisites (higher leverage now)**

1. Router / metering fields landing per [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) matrix rows.
2. Deterministic **`InferenceRuntime`** tests + NDJSON conformance already in-tree.

**Optional sidecar-era artifacts (defer until isolation need is proven)**

- Plugin manifest (`runtime`, `entrypoint`, `capabilities`, contract semver).
- Health + restart policy + conformance harness (`PLUGIN_ROADMAP.md` snapshot).

Suggested reading for maintainers pitching integrations:
- [VS Code Proposed API model](https://code.visualstudio.com/api/advanced-topics/using-proposed-api)
- [Open Source Guides – Best Practices for maintainers](https://opensource.guide/best-practices/)
- [GitHub contributing guideline discoverability](https://docs.github.com/en/communities/setting-up-your-project-for-healthy-contributions/setting-guidelines-for-repository-contributors)
