# Context Efficiency Architecture

This guide defines how REX reduces token usage and local compute for coding workflows.

**Hub:** Canonical **economics lever matrix** ([below](#economics-lever-matrix-rex-vs-product-techniques)) + system views in [ARCHITECTURE.md](ARCHITECTURE.md). **ADR** rationale: [architecture/decisions/](architecture/decisions/).

**Inference adapters:** pipeline stages are **not** one-size-fits-all. Each adapter (mock, local MLX, Cursor CLI, or a future sidecar) declares `AdapterCapabilities` in `docs/ADAPTERS.md` so the daemon can skip or apply indexer, compressor, token budget, cache, and behavioral prefilter. **Cursor adapter profile (design default):** skip heavy lexical **context injection** and **token-budget truncation** of the user prompt; keep the **behavioral prefilter**; **mode-gated** response cache per `docs/CACHING.md`.

**Local Cursor testing:** you can set `REX_INFERENCE_RUNTIME=cursor-cli` to exercise the in-process adapter while keeping the transport and context pipeline in place. Use `REX_CURSOR_CLI_PATH`, `REX_CURSOR_CLI_COMMAND`, and `REX_CURSOR_CLI_TIMEOUT_SECS` to control invocation and time bounds (see `docs/PLUGIN_ROADMAP.md`).

## Scope

- Add token budget controls before inference.
- Select and compress context primarily **inside `rex-daemon`**; delegate heavy experimentation to optional sidecars only when isolation dictates — [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md).
- Keep `rex-daemon` responsible for transport and stream correctness.
- Keep generic byte compression as storage-only optimization.

## Economics lever matrix (REX vs product techniques)

Single authoritative mapping. **`Status`** reflects code or documented design intent.

| Technique | REX responsibility | Module / seam | Primary doc anchor | Status |
|-----------|-------------------|---------------|--------------------|--------|
| Model routing / escalation cascade | Daemon chooses backend + model hint before adapter | Planned `InferenceRouter`; today env selects `InferenceRuntime` | [ADR 0004](architecture/decisions/0004-routing-daemon-first-optional-http-gateway.md), [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md) | **planned** |
| Context compaction — verbatim-safe packing | Daemon pipeline trims redundancy without inventing symbols | `ContextPipeline`, future packers | Responsibility map below | **partial** |
| Context compaction — learned / small-model | Optional compressor stage or sidecar ML | Compressor hooks in `plugins.rs` | Evidence-informed defaults | **planned** |
| Layered response cache — L1 exact | In-process LRU keyed by adapter, model, mode, schema, workspace | `l1_cache.rs`, `service.rs` | [CACHING.md](CACHING.md) | implemented (ask) |
| Layered cache — L2 semantic | Embedding similarity **ask-only** guarded | Planned | [CACHING.md](CACHING.md) | **planned** |
| Prefix / shared context reuse | TTL prefix cache segments in pipeline | `PrefixCache` in `plugins.rs` | Responsibility map below | **partial** |
| Vendor KV / prompt cache hints | Depends on outbound API owning runtime | Adapter metadata future | [CACHING.md](CACHING.md) | **planned** |
| Layered prompts (system/project stack) | Versioned assemblies to avoid duplicate client rules | Config + daemon assembly | [CONFIGURATION.md](CONFIGURATION.md) | **planned** |
| Batching / async doc jobs | Lower priority vs interactive latency | Future RPC/job | [ROADMAP.md](ROADMAP.md) | **planned** |
| Project memory — decisions + repo fingerprints | Reduce chat-history token pressure | Planned store (`sqlite`/files) alongside daemon | [LONG_TERM_MEMORY.md](LONG_TERM_MEMORY.md) | **planned** |
| MCP / standard tool interoperability | Daemon or bridge as MCP **client** | Integration TBD | [ARCHITECTURE.md](ARCHITECTURE.md) interoperability | **planned** |
| Human approvals + sandbox for tools | Extension modes today; sandbox future | Extension + daemon policy future | [EXTENSION.md](EXTENSION.md), [ARCHITECTURE.md](ARCHITECTURE.md) security | **partial** |

## Evidence-informed defaults

- Treat context as a scarce budget and avoid over-packing prompts; long-context quality can degrade when relevant evidence is buried ([Lost in the Middle](https://aclanthology.org/2024.tacl-1.9/)).
- Prefer retrieve-on-demand over fixed retrieval for every query when adapter capabilities allow it ([Self-RAG](https://arxiv.org/abs/2310.11511)).
- Use query-aware compression so local models can handle more tasks within bounded token budgets ([LLMLingua](https://arxiv.org/abs/2310.05736)).

## Architecture flow

```mermaid
flowchart LR
  editorClient[EditorOrCLI] --> daemonCore[rexDaemon]
  daemonCore --> layeredCache[LayeredResponseCache]
  layeredCache -->|miss| budgetGate[TokenBudgetGate]
  budgetGate --> retrieverPlugin[RetrieverPlugin]
  retrieverPlugin --> indexerPlugin[IndexerPlugin]
  retrieverPlugin --> compressorPlugin[CompressorPlugin]
  budgetGate --> contextPrefixCache[ContextPrefixCache]
  budgetGate --> behaviorPlugin[BehaviorPrefilterPlugin]
  compressorPlugin --> runtimeAdapter[InferenceRuntime]
  contextPrefixCache --> runtimeAdapter
  behaviorPlugin --> runtimeAdapter
  layeredCache -->|hit| streamOut[StreamChunkDoneOrError]
  runtimeAdapter --> streamOut
  streamOut --> editorClient
```

The cache and each pipeline stage can be **skipped** when the active adapter’s capabilities say so (for example, Cursor: skip most indexer or compressor context attached to the prompt, but run prefilter).

## Responsibility map

| Component | Responsibility |
|---|---|
| `rex-daemon` | Owns UDS/gRPC transport, lifecycle, final stream contract, and orchestration. |
| `LayeredResponseCache` (design) | L1 exact (and optional L2 semantic) cache in front of the inference adapter; see `docs/CACHING.md`. |
| `TokenBudgetGate` | Enforces prompt/context limits when the adapter opts in to REX context shaping. |
| `IndexerPlugin` | Maintains workspace-aware lexical index and ignore rules. |
| `RetrieverPlugin` | Selects top candidate context chunks deterministically. |
| `CompressorPlugin` | Applies extractive compression and token-budget packing. |
| `ContextPrefixCache` | Reuses stable context segments inside the REX context pipeline (today `PrefixCache` in the daemon) with TTL and bypass. |
| `BehaviorPrefilterPlugin` | Optionally suppresses low-value invocations using local behavior snapshots. |
| `InferenceRuntime` (adapter) | Mock, MLX, **Cursor CLI**, or future gRPC process; see `docs/ADAPTERS.md`. |

| Adapter (design) | REX context pipeline (default) |
|---|---|
| Mock / local MLX (future) | Full: budget gate, indexer, compressor, prefix cache, prefilter, then inference. |
| **Cursor CLI** (design) | **Prefilter on**; do **not** add heavy `[context]` from the lexical path on top of the user text; do **not** **truncate** the user prompt in the REX path before the CLI; response cache per mode in `docs/CACHING.md`. |

## Coding-first features

| Feature | Current behavior | Boundary |
|---|---|---|
| Workspace-scoped index | Uses lexical index with deterministic ranking and ignore filtering. | Sidecar-like plugin |
| Diff/hunk-aware packing | Supports compact context packing by selecting only relevant chunks. | Sidecar-like plugin |
| Symbol/structure chunking | Supports chunk-oriented retrieval contract; can evolve to AST-aware chunks later. | Sidecar-like plugin |
| Build/test diagnostics hint | Accepts diagnostics hint directives in prompt metadata. | Client input + sidecar-like plugin |
| Task-scoped context bundle | Supports bounded prompt context envelope (`prompt + [context]`). | Daemon orchestration |

## Current plugin contract

The daemon uses these contracts internally as sidecar seams:

- `TokenBudget`: max prompt tokens and max context tokens.
- `ContextRequest`: prompt, diagnostics hint, cache bypass flag, behavior snapshot.
- `PipelineResult`: effective prompt plus per-request metrics.
- `PipelineMetrics`: prompt tokens, context tokens, candidate/selected counts, truncation, cache status, behavior decision.

This contract lives in `crates/rex-daemon/src/plugins.rs`.

## Configuration examples

### Cache bypass

- Global bypass through environment variable:
  - `REX_CACHE_BYPASS=1`
- Per-request bypass directive inside prompt:
  - `[[cache:bypass]]`

### Diagnostics hint

- Add a diagnostics line to improve retrieval focus:
  - `[[diag: cargo test failed in runtime module]]`

### Behavior snapshot hint

- Add a focused typing hint to test behavioral prefilter path:
  - `[[behavior:focused]]`

## Local behavior telemetry defaults

### Defaults

- Keep behavior telemetry local.
- Do not persist raw code.
- Do not persist raw prompts.
- Emit coarse event categories only.

### Suggested event schema

| Field | Type | Example | Notes |
|---|---|---|---|
| `ts` | RFC3339 string | `2026-04-25T16:00:00Z` | Event timestamp |
| `typing_cadence_cpm` | integer | `280` | Characters per minute |
| `pause_events_last_minute` | integer | `2` | Coarse cognitive rhythm |
| `suggestion_requests_last_minute` | integer | `4` | Request pressure |
| `suppressed` | boolean | `false` | Prefilter result |
| `reason_code` | string | `focused-typing-window` | Stable categorical reason |

### Retention policy

- Use capped local storage (ring buffer or capped SQLite table).
- Rotate old entries automatically.
- Allow explicit user export for diagnostics.

## Multi-agent setup

When more than one agent can change this repository, follow the project **multi-agent collaboration** policy, plus any **global** multi-agent guardrails you keep in your own environment. Apply them at task start, before branch or stash actions, and at handoff.

## Verification checklist

- [ ] `cargo test -p rex-daemon` passes.
- [ ] Stream still ends with exactly one terminal event (`done` or `error`).
- [ ] Daemon logs include `stream.metrics` line per request.
- [ ] Cache reports `hit`, `miss_stored`, or `bypass`.
- [ ] Behavior prefilter path can be exercised with prompt directive.

## Out of scope for this phase

- Wasm plugin runtime hosting.
- Cross-process plugin supervision.
- ML-trained behavior model.
- Semantic retrieval reranking in production path.
