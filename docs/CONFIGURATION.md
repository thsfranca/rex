# REX configuration

This document is the **canonical** policy for how REX settings work: what applies today, how precedence will work when you add new layers, and where each `REX_*` variable is read. See [ARCHITECTURE.md](ARCHITECTURE.md) for where the **daemon** applies inference and cache policy.

## Why this policy exists

- **Developer experience:** Repeat the same run without retyping long `export` lines; future CLI flags and optional files will map to the same **names** as environment variables.
- **Automation:** CI, scripts, and the editor extension inject settings through the environment.
- **One catalog:** Lists variables the **Rust** binaries and core tool flow use.

## Precedence (target model)

Rex does **not** implement all layers below yet. **Phase 1 (today):** only **defaults** and **environment** apply.

| Precedence (low to high) | Role |
|--------------------------|------|
| Built-in defaults | Used when a setting is unset. |
| User persistent file (not implemented) | Optional file under the [XDG Base Directory](https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html) convention. |
| Project-local file (not implemented) | Optional repo-local file (for example `.rex.toml`); do **not** commit secrets. |
| Environment variables | **Primary** for parents (extension, tests, CI) and overrides today. |
| CLI flags (partial) | `rex complete` accepts `--model` and `--mode` per invocation. |

**Secret values:** Prefer environment or OS keychain for API keys. Do not commit secrets to the repository.

## Phase 1: environment variables (implemented)

### `rex-daemon` (inference and cache)

| Variable | Default (if unset) | Purpose |
|----------|--------------------|---------|
| `REX_INFERENCE_RUNTIME` | `http-openai-compat` | Broker backend when sidecar requests inference: **`http-openai-compat`**, **`mock`** (tests/harness), **`cursor-cli`** (legacy). Direct daemon HTTP without sidecar is **harness only** for MVP acceptance. |
| `REX_OPENAI_COMPAT_BASE_URL` | (none — **required** for HTTP runtime) | Base URL for **OpenAI-compatible** chat/completions (protocol name — not OpenAI-only). Examples: Ollama, LiteLLM gateway, OpenAI API — [ADAPTERS.md](ADAPTERS.md#terminology-protocol-vs-vendor). |
| `REX_OPENAI_COMPAT_API_KEY` | (none) | Optional `Bearer` token for remote APIs. |
| `REX_OPENAI_COMPAT_MODEL` | `gpt-4o-mini` | Model id sent in chat/completions requests; reported on `GetSystemStatus` when HTTP runtime is active. |
| `REX_OPENAI_COMPAT_TIMEOUT_SECS` | `120` | Upper bound for a single HTTP completion request. |
| `REX_CURSOR_CLI_PATH` | `cursor-agent` | Executable when runtime is `cursor-cli` (non-MVP). |
| `REX_CURSOR_CLI_COMMAND` | (none) | Optional shell template; `{prompt}` substituted (non-MVP). |
| `REX_CURSOR_CLI_TIMEOUT_SECS` | `20` | Subprocess bound for Cursor CLI adapter. |
| `REX_CACHE_BYPASS` | off | `1` or `true` bypasses L1 and context prefix cache — [`CACHING.md`](CACHING.md). |
| `REX_WORKSPACE_ROOT` | (cwd) | L1 fingerprint scope, brokered `fs.read`, and workspace lexical indexer root ([CACHING.md](CACHING.md), [AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md)). |
| `REX_INDEXER` | `workspace` | `workspace` walks the workspace (bounded); `seeded` uses deterministic in-memory docs (CI/tests). |
| `REX_MAX_PROMPT_TOKENS` | `512` | Context pipeline max prompt tokens (char heuristic ÷4). |
| `REX_MAX_CONTEXT_TOKENS` | `192` | Context pipeline max injected context tokens. |
| `REX_BROKER_SHELL_ALLOWLIST` | `echo,printf,true` | Comma-separated programs `exec.shell` broker may run (workspace cwd). |
| `REX_AGENT_APPROVALS` | off | `1` or `true` enforces daemon `ApprovalGate` for `agent` mode ([ADR 0009](architecture/decisions/0009-centralized-agent-approvals-and-checkpoints.md)). Pass `approval_id` on `StreamInference` (via `rex complete --approval-id`) after extension approval. |

### Sidecar supervision and harness

| Variable | Default (if unset) | Purpose |
|----------|--------------------|---------|
| `REX_SIDECAR_ENABLED` | off | `1`/`true` enables spawn; product path uses sidecar when enabled |
| `REX_SIDECAR_REQUIRED` | on when enabled | `0` makes sidecar optional (daemon starts without hard fail) |
| `REX_SIDECAR_BINARY` | `rex-sidecar-stub` on `PATH` | Sidecar executable for supervision |
| `REX_SIDECAR_SOCKET` | `/tmp/rex-sidecar.sock` | UDS for `rex.sidecar.v1` — [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md) |
| `REX_SIDECAR_HARNESS` | (none) | `direct` forces in-process inference (CI/tests); not MVP product acceptance |
| `REX_DAEMON_SOCKET` | `/tmp/rex.sock` | Daemon UDS for sidecar `BrokerInference` and `BrokerReadFile` during `RunTurn` |

### `rex` CLI (client metadata)

| Variable | Default (if unset) | Purpose |
|----------|--------------------|---------|
| `REX_TRACE_ID` | (none) | Request correlation; extension sets when spawning `rex` — [`EXTENSION.md`](EXTENSION.md). |

**CLI flags:** `rex complete` accepts `--format`, `--model <id>`, `--mode <ask|plan|agent>`, and `--approval-id <id>`. Unset model uses daemon default; empty mode normalizes to **`ask`** on the server ([`MVP_SPEC.md`](MVP_SPEC.md), [`CACHING.md`](CACHING.md)).

### Related project scripts

| Variable | Where it matters |
|----------|------------------|
| `REX_EXTENSION_EDITOR` | [EXTENSION_LOCAL_E2E.md](EXTENSION_LOCAL_E2E.md), install scripts. |
| `REX_TEST_STATUS_STATE_FILE` | Extension test fixtures only. |

**Module map:** Daemon: `adapters`, `http_openai_compat`, `approvals`, `l1_cache`, stream service. CLI: `runtime` (`REX_TRACE_ID`).

## Operator quick start (daemon + brokered HTTP)

The Phase 1 product path requires a **supervised sidecar** ([MVP_SPEC.md](MVP_SPEC.md)). Enable with `REX_SIDECAR_ENABLED=1` and related vars in the table above ([SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md)). HTTP env configures the **broker** the daemon uses for sidecar inference requests.

```bash
export REX_OPENAI_COMPAT_BASE_URL="http://127.0.0.1:11434/v1"   # Ollama example
export REX_OPENAI_COMPAT_MODEL="llama3.2"
export REX_INFERENCE_RUNTIME="http-openai-compat"
cargo run -p rex -- daemon
```

For **Anthropic and other cloud providers** via one broker URL, use the [LiteLLM operator profile](#operator-profile-litellm-anthropic-and-other-providers) below.

CI and unit tests set `REX_INFERENCE_RUNTIME=mock` and clear `REX_OPENAI_COMPAT_BASE_URL` — see [CI.md](CI.md).

## Operator profile: LiteLLM (Anthropic and other providers)

**Status:** operator-ready on existing `http-openai-compat` runtime — no Rex code change. Design: [ADAPTERS.md](ADAPTERS.md#multi-provider-gateway-via-litellm-recommended), [ADR 0018](architecture/decisions/0018-gateway-first-multi-provider-inference.md).

Run LiteLLM (or your deployment) with Anthropic and OpenAI keys in **LiteLLM’s** config. Rex only needs the OpenAI-compat surface LiteLLM exposes.

### Environment (Phase 1)

```bash
export REX_INFERENCE_RUNTIME="http-openai-compat"
export REX_OPENAI_COMPAT_BASE_URL="http://127.0.0.1:4000/v1"   # LiteLLM default listen example
export REX_OPENAI_COMPAT_MODEL="claude-sonnet-4-20250514"      # must match LiteLLM model_name / alias
# Optional: LiteLLM master key if your proxy requires it
# export REX_OPENAI_COMPAT_API_KEY="sk-..."
export REX_SIDECAR_ENABLED=1
```

`rex complete --model <id>` overrides the model sent on each request (LiteLLM uses it for routing).

### JSON fragment (R015 target)

Do not commit secrets. Anthropic API keys belong in LiteLLM configuration, not Rex `config.json`.

```json
{
  "inference": {
    "runtime": "http-openai-compat",
    "openai_compat": {
      "base_url": "http://127.0.0.1:4000/v1",
      "model": "claude-sonnet-4-20250514",
      "timeout_secs": 120
    }
  }
}
```

### Verification

1. Confirm LiteLLM responds: `curl` against `{base_url}/chat/completions` per LiteLLM docs.
2. Start daemon with sidecar enabled; `rex complete "hello" --format ndjson --model <litellm-model>`.
3. On failure, see broker error intent in [ADAPTERS.md](ADAPTERS.md#broker-provider-errors-intent).

## Planned: JSON configuration (R015)

**Not shipped.** Target model is documented in [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md):

| Layer | Role |
|-------|------|
| `$REX_HOME/config.json` | User defaults: sidecars, inference, `proto.gen_root` |
| `.rex/config.json` | Optional project overrides |
| Environment | **CI override** — same variable names as today |
| **`rex config`** / **`rex proto install`** | Operator bootstrap (**R014**–**R015**) |

Precedence target (low → high): built-in defaults → user JSON → project JSON → environment → CLI flags. Until **R015** lands, **environment remains primary** (table above).

## Layered prompts (design accepted)

**Status:** **design accepted** — not shipped. [ADR 0012](architecture/decisions/0012-layered-prompt-assemblies.md). Hub: [DEVELOPMENT_ASSISTANCE_CAPABILITIES.md](DEVELOPMENT_ASSISTANCE_CAPABILITIES.md). Implementation: **R015**+.

### Purpose

Versioned **system / project prompt assemblies** assembled in the daemon so clients and sidecars do not duplicate long rule blocks on every request.

### Scope

| In (design stage) | Out (design stage) |
|---|---|
| Assembly versioning and merge order (system → project → mode) | Full prompt authoring UI in the extension |
| Config keys or JSON section for assembly paths | Client-side prompt templating replacing daemon policy |
| Daemon responsibility for final prompt sent to broker | Replacing [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) retrieval/compression pipeline |

### Boundaries

- **Policy / assembly:** daemon owns merge order and version bumps.
- **Mechanism:** existing context pipeline and broker adapters unchanged until scheduled.
- See [ARCHITECTURE_GUIDELINES.md](ARCHITECTURE_GUIDELINES.md) for doc precedence.

### Interfaces (intent)

- R015 JSON section `prompts`: `system`, `project` (path or glob-scoped files), `mode` overlays.
- `prompt_assembly_revision` in L1 cache key with `context_revision` when retrieval ran ([CACHING.md](CACHING.md), [ADR 0012](architecture/decisions/0012-layered-prompt-assemblies.md)).
- Default cap: 25% of `REX_MAX_CONTEXT_TOKENS` for assembled prompts (see capabilities hub budget table).

### Cross-links

- [DEVELOPMENT_ASSISTANCE_CAPABILITIES.md](DEVELOPMENT_ASSISTANCE_CAPABILITIES.md)
- [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) — economics matrix row

## Not implemented yet (roadmap)

- Persistent user config on disk — see **Planned: JSON configuration** and **R015**.
- Global CLI flags mirroring env keys — deferred beyond **R014** (unified binary shipped).
- `rex config` subcommands — **R015**.
- Project-local `.rex/config.json` — **R015** (not `.rex.toml`).

## See also

- [ARCHITECTURE.md](ARCHITECTURE.md)
- [MVP_SPEC.md](MVP_SPEC.md)
- [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md)
- [ADAPTERS.md](ADAPTERS.md)
- [CACHING.md](CACHING.md)
- [EXTENSION.md](EXTENSION.md)
