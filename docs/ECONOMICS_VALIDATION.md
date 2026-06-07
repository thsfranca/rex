# Economics validation program (design hub)

**Diátaxis role:** explanation — how to prove Rex reduces cost without unacceptable quality loss.

**Status:** **design documented** — live-LLM smoke harness (**R039–R042**) and powered statistical gates are **planned**; no automated benchmark runner ships in this repository yet.

**Related:** [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md) (telemetry) · [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) (lever matrix) · [CONFIGURATION.md](CONFIGURATION.md) (`observability` JSON) · [CI.md](CI.md) (no live LLM on PRs by default)

Portable background: research theme on LLM observability and validation (GenAI semconv, benchmarks, cost–quality statistics, local store patterns) in the techythings repository.

## Purpose

- Define **scenarios**, **benchmarks**, and **stop/go criteria** for baseline vs Rex-enabled runs.
- Support **paid API** and **local open-source** backends via `inference` JSON ([CONFIGURATION.md](CONFIGURATION.md)).
- Attribute failures to **retrieval**, **compression**, or **generation** before changing models or levers.

## Scope

**In:**

- Scenario matrix with benchmark mapping.
- Statistical gates (TOST / non-inferiority, MDE sample sizes).
- Run manifest schema; cadence tiers (CI smoke, nightly, release).
- Local OSS effectiveness framework (hardware tiers, task-class routing — design bets).

**Out:**

- **Plan-mode JSON tool-loop live test** — separate track (**R038** — [NATIVE_TOOL_CALLING.md](NATIVE_TOOL_CALLING.md); not **R039–R042**).
- Powered SWE-bench / release-gate harness — later cadence tier (see [Cadence](#cadence)).
- Production export of benchmark pass rates as OTel metrics.
- Proprietary repository content in CI fixtures.

## Boundaries

| Concern | Owner |
|---------|--------|
| Telemetry during runs | [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md), `rex-obs-store` when `observability.enabled: true` (default **`sqlite`**; **`mmap`** opt-in on macOS — [ADR 0025](architecture/decisions/0025-dual-economics-store-engines.md)) |
| Lever definitions | [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) |
| Benchmark execution | Operator / scheduled CI (containers, pinned models) |

## Scenario matrix

| Scenario | Baseline | Rex-enabled | Primary benchmark | Economics signals |
|----------|----------|-------------|-------------------|-------------------|
| **Short ask** | Adapter only; retrieval off or N/A | Adaptive retrieval + cache | LiveCodeBench (subset) | `prompt_tokens`, TTFT, cache |
| **Code-context ask** | Full prompt without compaction | Extractive compaction + prefix cache | Aider Polyglot | `compression_strategy`, context tokens |
| **Agent turn** | Sidecar loop without prefix/vendor cache | Phase 1 bundle: prefix lock + raw results + microcompaction | SWE-bench Lite–style subset (~50 tasks) | `tokens_in_total`, `cached_tokens`, `prefix_hash`, `parse_retries` |
| **Paid API** | Remote OpenAI-compat | Same + pipeline stages | Per scenario above | Store + OTLP rollups |
| **Paid API via LiteLLM** | Gateway to Anthropic/OpenAI | Same Rex signals | Per scenario | LiteLLM logs for spend — [INFERENCE_GATEWAY.md](INFERENCE_GATEWAY.md) |
| **Local OSS** | Ollama / LM Studio | Same; emphasize compute | Per scenario | `load_duration`, store by `config_snapshot_id` |

## Benchmark roles

| Benchmark | Role | Notes |
|-----------|------|--------|
| **SWE-bench Verified** | Primary for multi-step agent / repo fixes | Docker required |
| **Aider Polyglot** | Primary for diff quality | Strict format adherence |
| **LiveCodeBench** | Secondary for short Q&A | Lower contamination risk |
| **Nugget recall** | Primary for retrieval isolation | LLM-assisted grading |
| **HumanEval / MBPP** | Smoke only | Not representative of repo work |

## Retrieval and compression validation

**Retrieval:** Nugget checklist per task; compute recall on injected context. High recall + low pass rate → model/routing. Low recall → pipeline issue.

**Compression:** Paired verbose vs compressed runs tied to `compression_strategy` in `stream.metrics`. Non-inferiority on pass rate required for token savings to count.

## Statistical gates

| Parameter | Proposed default |
|-----------|------------------|
| Non-inferiority margin δ | 2.5 percentage points on binary pass/fail |
| Powered sample size | n ≥ 400 per arm for ~5pp difference (80% power, α=0.05) |

**GO:** 95% CI lower bound for (experimental − baseline) pass rate ≥ −δ **and** significant token or `elapsed_ms` reduction.

**STOP:** CI lower bound &lt; −δ, or lever overhead exceeds savings.

## Live LLM validation program

**Purpose:** Bridge mock CI ([`mvp_product_path`](../crates/rex-daemon/tests/mvp_product_path.rs), RC-10) and manual operator acceptance ([EXTENSION_LOCAL_E2E.md](EXTENSION_LOCAL_E2E.md) §8 / **R019**) with an **opt-in** automated path against a real local model — without requiring live LLM on every PR.

**Status:** `planned` — implementation phased **R039–R042** on [ROADMAP.md](ROADMAP.md).

### Scenarios (**R039**)

| Scenario | What it proves | Mirrors |
|----------|----------------|---------|
| **`ask` NDJSON smoke** | Streaming + terminal `done` against live HTTP | CLI/extension NDJSON contract |
| **Brokered read + policy** | `__rex_read:<allowed>` succeeds; `__rex_read:.env` denied | [`mvp_product_path`](../crates/rex-daemon/tests/mvp_product_path.rs) with live `http_openai_compat` |

**Explicit out:** live **plan-mode** JSON tool-loop validation (user-owned separate track).

### Pinned models (operator / harness defaults)

| Role | Model | Notes |
|------|-------|-------|
| **Primary agent smoke** | `qwen2.5-coder:7b` | JSON-in-text protocol; ≤16GB guidance |
| **Fast streaming smoke** | `llama3.2` | Matches [EXTENSION_LOCAL_E2E.md](EXTENSION_LOCAL_E2E.md) §3 example |

### Interfaces (intent)

| Surface | Contract |
|---------|----------|
| Opt-in gate | Operator invokes script manually; not default PR CI (see `./scripts/verify_native_tools_live.sh`) |
| Harness entry | Future `scripts/verify_ollama_live.sh` or `#[ignore]` integration test |
| Inference | Direct Ollama `inference.openai_compat.base_url` `http://127.0.0.1:11434/v1` — [CONFIGURATION.md](CONFIGURATION.md) |
| Sidecar | Product path `rex-agent` (not stub) — [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md) |
| Gateway variant (**R041**) | Same scenarios via managed/external gateway — [INFERENCE_GATEWAY.md](INFERENCE_GATEWAY.md) |
| Run manifest (**R042**) | Fields in [Run manifest schema](#run-manifest-schema) written per harness run |

### Boundaries

- Harness **orchestrates** daemon and CLI; it does **not** embed or ship Ollama.
- Inference stays **broker-only** through daemon `http_openai_compat` — [ADAPTERS.md](ADAPTERS.md).
- RC-10 preserved: required PR checks stay mock/loopback; live tier is opt-in or scheduled informational.

### Roadmap linkage

| ID | MoSCoW | Outcome |
|----|--------|---------|
| **R039** | Should | Direct-Ollama live smoke harness (`ask` + brokered read/policy) |
| **R040** | Should | Scheduled non-blocking nightly workflow |
| **R041** | Could | Gateway-path live smoke (same scenarios) |
| **R042** | Could | Harness writes run manifest to economics store |

## Cadence

| Tier | When | Live LLM | Purpose |
|------|------|----------|---------|
| **Per-PR smoke** | Every PR | No (mock / replay) | Daemon latency, export wiring — RC-10 |
| **Opt-in local + nightly** | Operator or scheduled (**R039**, **R040**) | Yes (pinned model; small scenario set) | Product-path drift detection — not powered equivalence |
| **Release gate** | Model/lever changes | Yes (powered n) | TOST / non-inferiority — 50–100+ tasks |

## Local open-source effectiveness (design bets)

| Hardware tier | Guidance |
|---------------|----------|
| ≤16GB unified memory | Cap at ~7B–8B Q4/Q5 |
| 32GB+ | 30B+ class for harder tasks; measure P95 TTFT |

| Task class | Routing bet |
|------------|-------------|
| `short_ask`, `small_edit` | Local smaller model when observability shows stable TTFT |
| `multi_file`, `agent_loop` | Remote API or largest local model hardware allows |

**Threshold language (example):** Local configuration is *practically usable* when pass rate is non-inferior to remote baseline within δ on Aider Polyglot **and** P95 time-to-first-chunk &lt; 1.5s on the declared hardware profile.

## Run manifest schema

Stored in `rex-obs-store` `runs` / `run_tasks` when `observability.enabled: true` (either engine; harness should use a shared **store trait** and parity-check sqlite vs mmap on macOS — design intent, not shipped):

| Field | Purpose |
|-------|---------|
| `run_id` | Unique id |
| `config_snapshot_id` | Content hash FK — [ADR 0021](architecture/decisions/0021-rex-owned-economics-store-byot-visualization.md) |
| `git_sha` | Code under test |
| `model_revision` | Pinned LLM |
| `quant` | Quantization label |
| `hardware_profile` | Operator-declared class |
| `lever_toggles` | Pipeline flags from merged config snapshot |
| `benchmark_suite` | e.g. `swe-bench-verified-subset` |
| `pass_rate` | Aggregate |
| `metrics_snapshot` | Token/latency summaries |

## Agent-turn A/B protocol (design)

Golden set: ~50 local repo tasks (SWE-bench Lite–style subset). **Control:** current JSON tool results + variable system blurbs. **Experimental:** Phase 1 bundle per [AGENT_GRAPH_ARCHITECTURE.md](AGENT_GRAPH_ARCHITECTURE.md) (prefix immutability, **R034** delimiters, microcompaction tier).

| Metric | Target (experimental vs control) |
|--------|----------------------------------|
| `tokens_in_total` | >40% reduction (Phase 1 bundle) |
| `cached_tokens` / total input | >85% on steps 2–12 when vendor cache enabled |
| `parse_retries` | 0.0 post-**R038** native tools — [NATIVE_TOOL_CALLING.md](NATIVE_TOOL_CALLING.md) |
| `tool_steps` | No significant increase |
| `task_success_rate` | Non-inferiority δ = 2.5pp (gates below) |
| `prefix_hash` (SHA-256) | Identical across steps 1–12 per turn |

**Prefix-hash CI spec (planned):** harness records SHA-256 of static prefix bytes before each `BrokerInference` in a fixture turn; fail build if hash differs between step 1 and step N (N ≤ `max_tool_steps`). Wire fields: [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md).

**Cadence:** per-PR smoke uses mock/replay (no live LLM); powered agent-turn A/B on **release** or lever-change gates only — see [Cadence](#cadence).

## CHCE store benchmarks (mmap engine promotion)

When `observability.store.engine=mmap` (CHCE) ships, these gates apply before default-engine promotion ([OBS_STORE_MMAP_FORMAT.md](OBS_STORE_MMAP_FORMAT.md), [ADR 0027](architecture/decisions/0027-chce-columnar-mmap-engine.md)):

| Benchmark | Target | Verification |
|-----------|--------|--------------|
| 1M synthetic `append_stream` | &lt;100 ms queue drain | Harness / criterion bench |
| Hot-path `append_stream` | &lt;1 ms on Apple Silicon | Daemon micro-bench |
| Crash recovery | No panic; truncate torn page | `kill -9` mid-write fixture |
| Disk footprint | &lt;8 MB after 1M streams | `du` on `store.rexobs` |
| Engine parity | Aggregates within tolerance vs SQLite | Same scenario on both engines |
| Linux CI alignment | ELF page size 16384 when mmap crate builds | `check_elf_alignment.sh` |

Harness should use a shared **store trait** and parity-check sqlite vs CHCE on macOS — design intent in [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md).

## Gaps (explicit)

- Human preference and maintainability not captured by benchmarks.
- Multi-day session degradation under-tested.
- Open-source benchmarks may overfit; no customer code in CI.
- Local TTFT varies with thermal load — use rolling observability, not static labels alone.

## Cross-links

| Doc | Relationship |
|-----|----------------|
| [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md) | Signal catalog, store |
| [OBSERVABILITY_INTEGRATIONS.md](OBSERVABILITY_INTEGRATIONS.md) | Bundled Grafana + Rex read API |
| [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) | Economics harness row |
| [ROADMAP.md](ROADMAP.md) | **R039–R042** live smoke phasing; **R038** native tools — [NATIVE_TOOL_CALLING.md](NATIVE_TOOL_CALLING.md) |
| [CI.md](CI.md) | Three-tier validation model; RC-10 |
| [EXTENSION_LOCAL_E2E.md](EXTENSION_LOCAL_E2E.md) | Manual R019 acceptance; **R039** automated smoke follow-up |
| [INFERENCE_GATEWAY.md](INFERENCE_GATEWAY.md) | **R041** gateway-path smoke |
| [NATIVE_TOOL_CALLING.md](NATIVE_TOOL_CALLING.md) | **R038** plan-mode tool-loop E2E |
