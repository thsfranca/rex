# Inference adapters

This document defines the design contract for pluggable inference backends in REX. The daemon remains transport- and stream-correct; adapters own how text is produced for a given effective prompt, mode, and model policy.

## Purpose

- Keep REX **agnostic** to a single vendor or engine (mock, local MLX, Cursor CLI, or a future gRPC sidecar).
- Make **capability negotiation** explicit so context shaping, token budgets, and caching do not run blindly for every adapter.
- **MVP** includes **Cursor CLI** as an **enableable default inference plugin**: when turned on, it **forwards prompts** to the CLI for AI-assisted development; **mock** stays the default for CI and offline runs. Treat **Cursor CLI** as a **frontier-model gateway**: account-bound models and an agentic runtime, not a dumb pass-through. Cursor runs its own loop; REX still owns policy, optional layered cache, and the final gRPC stream contract.

## `InferenceRequest` (design contract)

Requests carry at least the following fields (concrete types live in the implementation layer):

| Field | Purpose |
|---|---|
| `prompt` | User-visible task text (after REX may attach structured context or directives). |
| `mode` | Optional `ask`, `plan`, or `agent` (or future values). Affects tool use and **cacheability** (see `docs/CACHING.md`). |
| `model_hint` | Optional model id or `auto` when the adapter supports it. |
| `trace_id` | Correlates logs and metrics across the stack. |
| `metadata` | Opaque per-adapter hints (workspace root, thread id, etc.) without coupling the public proto to one vendor. |

**Invariants:** the daemon must still deliver exactly one terminal stream outcome (`done` or `error`) to clients, regardless of adapter.

## Streaming response contract

- Adapters return a **server-side stream** of `StreamInference`-style chunks (text + index + `done` flag in the public API).
- The stream **ends with exactly one** terminal success (`done = true` with empty text) or a terminal error mapping to the gRPC contract.
- Clients and the CLI NDJSON path remain valid; see `MVP_SPEC.md` for the extension consumer contract.

## `AdapterCapabilities` (design contract)

Adapters declare what REX may do before invocation. Examples:

| Capability | Meaning |
|---|---|
| `wants_context_injection` | If false, REX should not prepend heavy `[context]` from the lexical pipeline (Cursor does its own retrieval). If true, REX may attach daemon-built context. |
| `cacheable_modes` | Which `mode` values are safe to cache (never include `agent` if it can write files). |
| `max_prompt_tokens` / `max_context_tokens` | Optional override; `0` or unset means use daemon defaults. |
| `default_timeout` | Subprocess or RPC budget; Cursor CLI spawns are subject to hang risk and should always use a bounded wait + kill. |
| `supported_modes` | Modes the adapter can run; others should fail fast with a clear error. |

The daemon consults these flags so **TokenBudgetGate**, **Indexer/Retriever/Compressor**, **prefix-style cache in the context pipeline**, and **behavioral prefilter** are applied per adapter. See `docs/CONTEXT_EFFICIENCY.md` for the default policy for the **Cursor adapter profile**.

## Cursor CLI adapter (design profile)

Intended use: shell out to Cursor’s CLI (for example `cursor-agent`) in **non-interactive** mode with structured output (for example `-p`, `--print`, and `--output-format json` as documented for your Cursor version).

| Aspect | Design default |
|---|---|
| Role | Frontier-model and agent runtime, not the sole context source. |
| First slice | `ask` mode only; `plan` and `agent` unlock after proto/CLI can pass mode and policy safely. |
| Context | **No** REX full lexical context injection on top of the user prompt; Cursor’s workspace tools handle retrieval. Behavioral prefilter may still run. |
| Session | **Fresh** invocation per request: REX owns long-lived conversation and routing; Cursor is stateless at the REX boundary unless you add explicit session ids later. |
| Lifecycle | Mandatory **timeout** and process teardown on expiry; map hangs to a clear terminal `error` for the client. Non-zero exits embed stderr in the gRPC `unavailable` text with a bounded length; see [CONFIGURATION.md](CONFIGURATION.md) (Cursor CLI). |
| Model | Pass through `model_hint` including `auto` when the CLI supports it. |

Risks to document in operations: some environments report `print` / headless mode not exiting; defensive timeouts and logging are required.

### Local verification (Cursor CLI)

- **Default CI** does not install Cursor. It keeps the mock runtime. The UDS integration test `cursor_runtime_streams_chunks_over_uds` in `crates/rex-daemon/tests/uds_e2e.rs` sets `REX_INFERENCE_RUNTIME=cursor-cli` and `REX_CURSOR_CLI_COMMAND` to a `printf` shell stub that prints one JSON line, so the adapter path is covered without a real `cursor-agent` process.
- **Real CLI (optional, local):** install the Cursor CLI on your machine, set `REX_INFERENCE_RUNTIME=cursor-cli`, and optionally set `REX_CURSOR_CLI_PATH` or `REX_CURSOR_CLI_COMMAND` per [CONFIGURATION.md](CONFIGURATION.md). Start `rex-daemon`, then run `rex-cli complete "hello" --format ndjson` to exercise the end-to-end path. Expect bounded stderr in error `Status` messages (see [CONFIGURATION.md](CONFIGURATION.md) — Cursor CLI).

## Promotion: in-process adapter to gRPC sidecar

The same logical contract (capabilities + stream semantics) can be implemented in a **separate process** that speaks gRPC, matching `docs/PLUGIN_ROADMAP.md` (plugin lifecycle baseline). REX’s clients and `rex.v1` do not need to change if the sidecar is a drop-in backend behind the `InferenceRuntime` seam.

## Checklist: add a new adapter

1. Define `AdapterCapabilities` and supported modes; document cache safety for `agent`.
2. Map REX’s stream chunks and terminal `done`/`error` to your backend with no double terminals.
3. If spawning a process, set `cwd` to the workspace, env from config, and enforce timeout + signal handling.
4. If using another vendor API, key cache entries and metrics by adapter id and model (see `docs/CACHING.md`).
5. Add a short “adapter profile” subsection in this file when the design stabilizes.
6. Extend `docs/PLUGIN_ROADMAP.md` if the work implies a new phased track or dependencies on the sidecar gate.

## Related

- [ARCHITECTURE.md](ARCHITECTURE.md) — high-level data flow and inference adapters overview.
- [CACHING.md](CACHING.md) — layered cache keys, bypass, and mode safety.
- [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) — per-adapter pipeline policy.
- [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md) — Cursor adapter phased track and sidecar-first gate.
