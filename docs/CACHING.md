# Caching architecture

This document describes how REX layers **application-level** caching in front of inference. Provider-native prompt or KV cache inside a model vendor (when you do not own the runtime) is out of scope here; that behavior is a bonus, not a dependency for REX correctness.

**Economics context:** see canonical lever matrix in [`CONTEXT_EFFICIENCY.md`](CONTEXT_EFFICIENCY.md) and [ADR 0003](architecture/decisions/0003-layered-cache-agent-mode-policy.md).

## Goals

- Reduce **latency and cost** on repeated or near-identical work.
- Preserve **safety** for **agent** mode: never reuse a past completion if it could imply or reflect filesystem edits.
- Key entries so upgrades (schema or model) do not silently return stale text.

## Layered model

| Layer | Mechanism | Bypasses the model? | Typical use |
|---|---|---|---|
| L1 | **Exact match** on a normalized key | Yes, when the key matches | Idempotent, safe modes |
| L2 (optional) | **Semantic** nearest-neighbor on embeddings | Yes, when similarity exceeds a threshold and guards pass | Paraphrased **ask** prompts (high care) |
| Provider | **Prompt / prefix cache** in the model API | No (faster prefill) | Long static system prompts; optional |

REX’s **context pipeline** in `crates/rex-daemon/src/plugins.rs` also uses a small **prefix cache** for retrieved context. The **layered cache** described here is the design target for a single **response cache in front of the `InferenceRuntime` adapter**, with keys that include adapter identity, mode, and model. See [ADAPTERS.md](ADAPTERS.md) for capability flags.

**Implementation status:** the daemon now runs an in-memory, LRU-bounded **L1 exact** response cache (module `l1_cache`, constant `L1_CACHE_SCHEMA` in the Rust crate) for **ask** mode only; **`plan` and `agent` do not** read or write L1. Daemon logs show `l1_cache=miss` or `l1_cache=hit` when L1 is applicable. The prompt portion of the key hashes **effective** post-pipeline text; workspace scoping uses `REX_WORKSPACE_ROOT` when set (see [CONFIGURATION.md](CONFIGURATION.md)). The daemon also wires a `LayeredCache` (`policy::LayeredCache::default_l1_l2`) with **L1** followed by an **L2 null slot** (`policy::NullL2`) so a future semantic L2 implementation plugs in behind the same `ResponseCache` trait without changing `service.rs`. L2 semantic behavior itself remains design-only.

## L1 exact-match keys

A robust key should include, at minimum:

- **Adapter id** (for example `mock`, `cursor-cli`, `mlx`).
- **Model id** (or a sentinel for `default` / `auto` resolution).
- **Mode** (`ask`, `plan`, `agent`).
- **Normalized prompt** (whitespace- and line-ending-normalized) or a **hash** of it.
- **REX cache schema version** (bump when the pipeline or chunk format changes and old entries are invalid).
- **Workspace signature** (for example a hash of repo root path + optional commit hash) so a cache from another checkout does not apply.

**`agent` mode: never store or serve from L1/L2** in the default policy: the remote side may have applied edits, so replay is unsafe.

**`plan` mode:** only cache if the adapter declares it safe; plans can include stale assumptions if the tree changed, so default to off or to short TTL with workspace signature strictness.

**`ask` mode:** the usual candidate for L1; still respect bypass.

## Normalization

Before hashing the prompt for the key:

- Unicode normalize and trim.
- Unify newlines to `\n`.
- Optionally strip volatile directives in a documented way, or keep them in the key if they change semantics (prefer keeping them in the key).

## Bypass

- **Per-request:** `[[cache:bypass]]` in the prompt (see `docs/CONTEXT_EFFICIENCY.md`).
- **Global env:** `REX_CACHE_BYPASS=1` or `true` for diagnostics.

When bypass is on, the cache layer should record `bypass` in metrics, not a miss.

## TTL, eviction, and invalidation

- **TTL** per entry or per layer: typical short TTL for L1 in dev (minutes) and longer in stable ask-only flows, if you enable it.
- **Max entries** with LRU or FIFO under a size cap to avoid unbounded memory.
- **Schema version** bump: drop or ignore all prior entries when the pipeline or terminal stream contract changes.

## L2 semantic cache (optional, ask mode)

Use when you want hits across **paraphrase**, not just identical strings.

| Concern | Mitigation |
|---|---|
| False positive (wrong answer) | **Similarity threshold** (often 0.85–0.95) plus optional second checks: entity overlap, output length bounds, or a small classifier for “safe to reuse” |
| Stale file context | Tighten key with **workspace signature**; prefer L2 for **read-only** questions |
| Cost of embeddings | Off by default; use a small, local model if policy allows |

**Scope default:** L2 is **ask-only** until you have evidence that your semantic gate is safe for `plan`.

## Metrics and observability

The daemon emits one `cache_decision=` field per request on stdout:

- `cache_decision=hit` | `miss_stored` | `bypass` | `uncacheable_mode` | `miss_semantic` (if L2 exists)

The legacy `l1_cache=hit|miss` line is also emitted for cacheable lookups so existing grep-based triage keeps working; new dashboards should prefer `cache_decision=` because it covers all four states (including `bypass` and `uncacheable_mode`, which never produced an `l1_cache=` line).

## Related

- [ADAPTERS.md](ADAPTERS.md) — which modes and adapters may cache.
- [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) — context pipeline, directives, and behavioral prefilter.
- [ARCHITECTURE.md](ARCHITECTURE.md) — data flow and layered cache in the system picture.
