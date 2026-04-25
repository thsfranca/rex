# Plugin Roadmap (Sidecar-First)

This roadmap defines how REX should grow plugin capabilities after MVP while keeping the daemon lightweight.

## Current purpose

- Deliver a local completion product that extensions can consume reliably.
- Keep `rex-daemon` focused on core runtime responsibilities.
- Add user-facing value through sidecars before promoting features into the daemon.

## Related implementation docs

- Read `docs/CONTEXT_EFFICIENCY.md` for current token budget contracts, plugin boundaries, and local telemetry defaults.

## Principles

- Prefer sidecars/plugins by default.
- Keep built-in scope small until evidence justifies expansion.
- Ship in small, reviewable PRs with clear acceptance criteria.
- Keep transport and contracts stable while iterating feature logic.

## Sidecar-first decision gate

Use this gate for every new feature.

| Question | If yes | If no |
|---|---|---|
| Does it require deep model runtime control (KV cache, memory scheduler, tokenizer internals)? | Consider built-in | Keep as plugin |
| Must it be non-bypassable for compliance or safety? | Consider built-in | Keep as plugin |
| Can teams benefit from fast iteration and replacement? | Keep as plugin | Consider built-in later |
| Can it fail independently without breaking core completion flow? | Keep as plugin | Consider built-in |

Default outcome: keep the feature as a sidecar/plugin.

## Candidate features from common local-AI pains

### 1) Prompt/context cache

- **User pain:** repeated prompts waste time and cost.
- **Feature:** exact + semantic cache for repeated context blocks.
- **Default placement:** sidecar.
- **Why now:** fast user-visible latency win with low daemon complexity.
- **Promote to built-in when:** cache must integrate tightly with model internals.

### 2) Request observability

- **User pain:** teams cannot debug slow or low-quality responses quickly.
- **Feature:** request traces, latency, token/cost counters, error taxonomy.
- **Default placement:** sidecar.
- **Why now:** enables quality and reliability iteration without daemon bloat.
- **Promote to built-in when:** always-on telemetry must be guaranteed with minimal dependency surface.

### 3) Context shaping

- **User pain:** context slop causes irrelevant or unstable responses.
- **Feature:** filter, rank, and trim context before inference.
- **Default placement:** sidecar.
- **Why now:** experimentation-heavy logic that benefits from rapid iteration.
- **Promote to built-in when:** product requires deterministic context policy everywhere.

### 4) Hybrid local/cloud routing

- **User pain:** local models are cheap/private but weaker on hard tasks.
- **Feature:** local-first routing with fallback/escalation policies.
- **Default placement:** sidecar.
- **Why now:** unlocks quality gains without expanding daemon core.
- **Promote to built-in when:** routing guarantees become core product behavior.

### 5) Guardrails and policy

- **User pain:** risk of leaking secrets or generating unsafe code.
- **Feature:** PII redaction, policy checks, and response filtering.
- **Default placement:** sidecar.
- **Why now:** policy changes often and varies by environment.
- **Promote to built-in when:** enforcement must be mandatory and non-bypassable.

## Roadmap sequence (small PRs)

## Goal

Add plugin value in low-risk slices while preserving daemon simplicity.

### PR 1: Plugin contract and lifecycle baseline

- **Outcome:** daemon can load and supervise one sidecar plugin reliably.
- **Scope:**
  - Define plugin metadata contract (runtime, version, entrypoint, capabilities).
  - Add health-check and readiness handshake.
  - Add timeout/restart/shutdown policy for one plugin process.
- **Acceptance criteria:**
  - Daemon starts with no plugin configured.
  - Daemon starts and routes with one healthy plugin.
  - Daemon surfaces clear error when plugin fails readiness.

### PR 2: Observability plugin

- **Outcome:** request-level insight for debugging and tuning.
- **Scope:**
  - Add sidecar that records request/response timing and terminal status.
  - Emit structured events that CLI and extension flows can correlate.
- **Acceptance criteria:**
  - Each completion request has a trace id.
  - Latency and terminal status are visible for success and failure paths.
  - Failure events include actionable reason categories.

### PR 3: Prompt/context cache plugin

- **Outcome:** lower latency for repeated context-heavy requests.
- **Scope:**
  - Add sidecar cache with configurable key strategy and TTL.
  - Support cache bypass flag for diagnostics.
- **Acceptance criteria:**
  - Repeated prompt segments can produce cache hits.
  - Cache hit/miss status appears in observability output.
  - Cache bypass reliably forces model path.

### PR 4: Context shaping plugin

- **Outcome:** improve completion quality by reducing irrelevant context.
- **Scope:**
  - Add plugin that scores and trims candidate context blocks.
  - Keep fallback to pass-through mode.
- **Acceptance criteria:**
  - Plugin can run in enforce and observe-only modes.
  - Pass-through mode preserves MVP behavior.
  - Extension consumer still receives valid stream terminal events.

### PR 5: Hybrid routing plugin (optional)

- **Outcome:** balance local cost/privacy with quality on hard tasks.
- **Scope:**
  - Add policy-driven local-first routing with explicit fallback triggers.
  - Add routing decision metadata to observability output.
- **Acceptance criteria:**
  - Local-first path remains default.
  - Fallback is explicit, logged, and testable.
  - Failures preserve clear terminal error semantics.

## What stays built-in for now

- Socket lifecycle and local transport ownership.
- Core request validation and stream terminal semantics.
- Minimal scheduling and cancellation control.
- Typed errors and reliability guarantees for daemon startup/shutdown.

## What to defer

- Wasm plugin runtime host.
- Full multi-plugin orchestration complexity.
- Heavy policy engines inside daemon code.
- Feature expansion that does not improve completion quality, reliability, or developer feedback loop.

## Success metrics for roadmap execution

- Median completion latency improves on repeated requests.
- Stream terminal correctness remains stable (`done` or `error`, exactly one).
- Plugin failures do not crash the daemon.
- Extension consumer path stays backwards compatible.
- Daemon code growth stays controlled relative to plugin code growth.
