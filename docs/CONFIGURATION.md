# REX configuration

This document is the **canonical** policy for how REX settings work: what applies today, how precedence will work when you add new layers, and where each `REX_*` variable is read. See [ARCHITECTURE.md](ARCHITECTURE.md) for where the **daemon** applies inference and cache policy.

## Why this policy exists

- **Developer experience:** Repeat the same run without retyping long `export` lines; future CLI flags and optional files will map to the same **names** as environment variables.
- **Automation:** CI, scripts, and the editor extension set **`REX_ROOT`** (optional) and use JSON config; legacy `REX_*` tuning vars are ignored with a startup warning.
- **One catalog:** Documents JSON keys, bootstrap commands, and remaining environment variables.

## Precedence (implemented)

| Precedence (low to high) | Role |
|--------------------------|------|
| Built-in defaults | Used when a JSON field is unset. |
| `$REX_ROOT/config.json` | User defaults: daemon socket, sidecars, inference, workspace, broker, agent. |
| `.rex/config.json` | Optional project overrides (walked from cwd upward). |
| CLI flags (partial) | `rex complete` accepts `--model`, `--mode`, `--approval-id`, `--trace-id` per invocation. |

**Layout root:** `$REX_ROOT` defaults to `~/.rex` when unset. Run `rex config init` to create the layout and template `config.json`.

**Legacy environment variables:** Daemon startup ignores former `REX_INFERENCE_RUNTIME`, `REX_OPENAI_COMPAT_*`, `REX_SIDECAR_*`, `REX_DAEMON_SOCKET`, and `REX_WORKSPACE_ROOT` when present and prints a warning — use JSON instead. **`REX_ROOT`** remains the bootstrap override for layout location (tests, extension auto-start).

**Secret values:** Prefer environment or OS keychain for API keys in JSON (`inference.openai_compat.api_key`). Do not commit secrets to the repository.

## JSON configuration keys (implemented)

Bootstrap: `rex config init|show|path|validate`, `rex sidecar list|init|doctor`, `rex proto install|path|doctor`.

| Section | Keys | Purpose |
|---------|------|---------|
| `daemon` | `socket` | Daemon UDS path (default `/tmp/rex.sock`). |
| `sidecars` | `active`, `required`, `harness`, `list[]` | Supervised sidecar; `harness: "direct"` skips spawn (CI/tests). |
| `inference` | `runtime`, `openai_compat`, `cursor_cli` | Broker backend: `mock`, `http-openai-compat`, `cursor-cli`. |
| `workspace` | `root`, `indexer` | Broker root and lexical indexer (`workspace` or `seeded`). |
| `context` | `max_prompt_tokens`, `max_context_tokens` | Context pipeline budgets. |
| `cache` | `bypass` | L1 / prefix cache bypass. |
| `broker` | `shell_allowlist` | Allowed `exec.shell` programs. |
| `agent` | `approvals_enabled`, `max_tool_steps` | Agent-mode approval gate. |

Minimal example:

```json
{
  "version": 1,
  "daemon": { "socket": "/tmp/rex.sock" },
  "sidecars": {
    "active": "stub",
    "required": true,
    "list": [
      { "name": "stub", "binary": "rex-sidecar-stub", "enabled": true, "socket": "/tmp/rex-sidecar.sock" }
    ]
  },
  "inference": {
    "runtime": "http-openai-compat",
    "openai_compat": {
      "base_url": "http://127.0.0.1:11434/v1",
      "model": "llama3.2"
    }
  },
  "workspace": { "root": "." }
}
```

## Legacy environment variables (deprecated)

The tables below document **former** env-based tuning. **Do not use** for new setups — they are ignored by the daemon (warning only). Tests and the extension should set **`REX_ROOT`** and write or merge JSON instead.

### Former `rex-daemon` variables

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

### `rex` CLI (client)

| Variable | Default (if unset) | Purpose |
|----------|--------------------|---------|
| `REX_ROOT` | `~/.rex` | Layout root for `config.json`, proto paths, and daemon auto-start from the extension. |

**CLI flags:** `rex complete` accepts `--format`, `--model <id>`, `--mode <ask|plan|agent>`, `--approval-id <id>`, and `--trace-id <id>`. Trace correlation uses **`--trace-id`** (not `REX_TRACE_ID`). Unset model uses daemon default; empty mode normalizes to **`ask`** on the server ([`MVP_SPEC.md`](MVP_SPEC.md), [`CACHING.md`](CACHING.md)).

The CLI reads `daemon.socket` from merged JSON for UDS transport (`rex_config::load_merged`).

### Related project scripts

| Variable | Where it matters |
|----------|------------------|
| `REX_EXTENSION_EDITOR` | [EXTENSION_LOCAL_E2E.md](EXTENSION_LOCAL_E2E.md), install scripts. |
| `REX_TEST_STATUS_STATE_FILE` | Extension test fixtures only. |

**Module map:** Daemon: `settings`, `adapters`, `http_openai_compat`, `approvals`, `l1_cache`, stream service. CLI: `transport` (config socket), `runtime` (`--trace-id`).

## Operator quick start (daemon + brokered HTTP)

The product path requires a **supervised sidecar** ([MVP_SPEC.md](MVP_SPEC.md)). Bootstrap JSON, then start the daemon:

```bash
rex config init
# Edit $REX_ROOT/config.json — set inference.openai_compat.base_url and model; enable sidecars.list[].enabled
rex config validate
rex daemon
```

For **Anthropic and other cloud providers** via one broker URL, use the [LiteLLM operator profile](#operator-profile-litellm-anthropic-and-other-providers) below.

Example HTTP backend (Ollama) in `$REX_ROOT/config.json`:

```json
"inference": {
  "runtime": "http-openai-compat",
  "openai_compat": {
    "base_url": "http://127.0.0.1:11434/v1",
    "model": "llama3.2"
  }
}
```

CI and unit tests set `REX_ROOT` to a temp dir and write `config.json` with `inference.runtime: "mock"` and `sidecars.harness: "direct"` — see [CI.md](CI.md).

## Operator profile: LiteLLM (Anthropic and other providers)

**Status:** operator-ready on existing `http-openai-compat` runtime — no Rex code change. Design: [ADAPTERS.md](ADAPTERS.md#multi-provider-gateway-via-litellm-recommended), [ADR 0018](architecture/decisions/0018-gateway-first-multi-provider-inference.md).

Run LiteLLM (or your deployment) with Anthropic and OpenAI keys in **LiteLLM’s** config. Rex only needs the OpenAI-compat surface LiteLLM exposes.

### Configuration

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
  },
  "sidecars": {
    "active": "stub",
    "required": true,
    "list": [
      { "name": "stub", "binary": "rex-sidecar-stub", "enabled": true, "socket": "/tmp/rex-sidecar.sock" }
    ]
  }
}
```

`rex complete --model <id>` overrides the model sent on each request (LiteLLM uses it for routing).

### Verification

1. Confirm LiteLLM responds: `curl` against `{base_url}/chat/completions` per LiteLLM docs.
2. Start daemon with sidecar enabled; `rex complete "hello" --format ndjson --model <litellm-model>`.
3. On failure, see broker error intent in [ADAPTERS.md](ADAPTERS.md#broker-provider-errors-intent).

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
- Default cap: 25% of `context.max_context_tokens` for assembled prompts (see capabilities hub budget table).

### Cross-links

- [DEVELOPMENT_ASSISTANCE_CAPABILITIES.md](DEVELOPMENT_ASSISTANCE_CAPABILITIES.md)
- [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) — economics matrix row

## Not implemented yet (roadmap)

- Global CLI flags mirroring all JSON keys — partial today (`rex complete` flags only).
- Layered prompt assemblies — see **Layered prompts** below.

## See also

- [ARCHITECTURE.md](ARCHITECTURE.md)
- [MVP_SPEC.md](MVP_SPEC.md)
- [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md)
- [ADAPTERS.md](ADAPTERS.md)
- [CACHING.md](CACHING.md)
- [EXTENSION.md](EXTENSION.md)
