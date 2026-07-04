# REX configuration

This document is the **canonical** policy for how REX settings work: merged **JSON** under `$REX_ROOT`, optional project overrides, and partial CLI flags. See [ARCHITECTURE.md](ARCHITECTURE.md) for where the **daemon** applies inference and cache policy.

## Configuration surface

**Rex has one product environment variable: `REX_ROOT`.** All product settings are JSON. Do not configure Rex with other `REX_*` environment variables.

| Surface | Role |
|---------|------|
| **Product settings** | `$REX_ROOT/config.json` and optional `.rex/config.json` (walked from cwd upward) |
| **Product env** | **`REX_ROOT` only** (optional). Layout directory for `config.json`, protos, and related paths. Defaults to `~/.rex` when unset. |
| **CLI flags (partial)** | Per-invocation overrides on `rex complete` / `rex status` (see [Precedence](#precedence-implemented)) |
| **Secrets** | Prefer OS keychain or a non-`REX_*` secret store for API key *values* referenced by JSON fields (`inference.openai_compat.api_key`, `inference.openai_compat.headers`). Do not commit secrets. |

**Not product configuration:** The daemon may inject socket paths into **child processes it spawns** (internal process plumbing). CI and contributor scripts may use test gates — see [CI.md](CI.md). Neither is an operator config catalog.

Former tuning names (if present in the environment) are **ignored** with a startup warning. Use JSON instead. There is **no** migration catalog of those names.

## Why this policy exists

- **Developer experience:** One merged config file per machine; `rex config show` for inspection.
- **Automation:** CI, scripts, and clients set **`REX_ROOT`** and write JSON.
- **One catalog:** JSON keys and bootstrap commands only.

## Precedence (implemented)

| Precedence (low to high) | Role |
|--------------------------|------|
| Built-in defaults | Used when a JSON field is unset. |
| `$REX_ROOT/config.json` | User defaults: daemon socket, sidecars, inference, workspace, broker, agent. |
| `.rex/config.json` | Optional project overrides (walked from cwd upward). |
| CLI flags (partial) | Product entry is bare **`rex`** (no per-invocation daemon opt-out). |

**Layout root:** `$REX_ROOT` defaults to `~/.rex` when unset. Run `rex config init` to create the layout and operator template `config.json` (**`sidecars.active: agent`**, **`rex-agent`** enabled, **`search.enabled: true`** with **`search.provider: mock`**). Operators typically edit only **`inference.openai_compat`** for a live backend. CI and harness tests use explicit stub configs or `RexConfig::defaults` in code — not the init template.

The CLI reads merged JSON for UDS transport and daemon lifecycle (`rex_config::load_merged`).

## JSON configuration keys (implemented)

Bootstrap: `rex config init|show|path|validate`, `rex sidecar list|init|doctor`, `rex proto install|path|doctor`.

| Section | Keys | Purpose |
|---------|------|---------|
| `daemon` | `socket`, `socket_scope`, `ready_timeout_secs`, `idle_shutdown_secs`, `log_path` | Daemon UDS path; **`socket_scope`** (`per_workspace` default, or `global` for legacy single socket); CLI always ensures the daemon (**R071**); readiness poll budget; idle auto-shutdown (**default 300s**, **`0`** disables); detached daemon log file. Per-workspace scope derives `$REX_ROOT/sockets/ws-<hash>.sock` from canonical process **cwd** — [ADR 0036](architecture/decisions/0036-per-workspace-daemon-routing.md), idle shutdown — [ADR 0037](architecture/decisions/0037-daemon-idle-shutdown.md). |
| `sidecars` | `active`, `host`, `required`, `harness`, `list[]`, `capabilities[]` | Host sidecar (`list[]` entry named by `host` or `active`); optional capability fleet (`capabilities[]` with `provides`, `socket`, `binary`); `harness: "direct"` skips spawn (CI/tests). |
| `inference` | `runtime`, `openai_compat`, `gateway`, `omlx`, `cursor_cli` | Broker backend: `mock`, `http-openai-compat`, `cursor-cli`. Managed gateway/oMLX inject `openai_compat.base_url` — see below. |
| `workspace` | `indexer` | Lexical indexer mode (`workspace` or `seeded`). **Workspace root** is always canonical process **cwd** — not configurable ([ADR 0011](architecture/decisions/0011-workspace-binding-and-turn-context-authority.md)). |
| `context` | `max_prompt_tokens`, `max_context_tokens` | Context pipeline budgets. |
| `cache` | `bypass` | L1 / prefix cache bypass. |
| `broker` | `shell_allowlist`, `max_tool_result_bytes` | Allowed `exec.shell` programs; max bytes returned from `fs.read` and `exec.shell` stdout/stderr (default **8192**). Write upload cap remains **65536** bytes per request. |
| `agent` | `approvals_enabled`, `tool_approvals_enabled`, `max_tools_per_step`, `deterministic_init_enabled`, `compaction_enabled`, `compaction_suffix_fraction`, `read_pruning_enabled` | Agent-mode approval gates; max batchable broker calls per LLM round (default **8**, **R057**); pre-LLM ask init (**R060**); intra-turn compaction (**R029**, default off); compaction threshold fraction (default **0.25**); goal-hint read pruning (**R031**, default off). |
| `cli` | `stream_idle_timeout_secs_agent`, `stream_idle_timeout_secs_ask`, `ui` | Per-chunk idle timeouts (defaults **120**); `cli.ui.enabled` / `sync_output` for TUI (**R073**). |
| `search` | `enabled`, `provider`, `max_results`, `api_key_path` | Ask-mode `web.search` broker (`provider: mock` for local demos). **R055** will migrate to capability sidecar — [WEB_SEARCH.md](WEB_SEARCH.md). |
| `observability` | `enabled`, `service_name`, `otlp` | OTLP export + stdout economics — [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md), [Observability](#observability) |

**Capability sidecar entry (`capabilities[]`):** `name`, `binary`, `enabled`, `socket`, `provides` (capability ids, e.g. `web.search`), optional `required`. Daemon spawns enabled entries alongside the host; invoke routing is **R056-2** — [CAPABILITY_SIDECARS.md](CAPABILITY_SIDECARS.md).

**Not implemented (do not set):** `context.advisory_intent_enabled` (**R067**), `broker.web_search`, `git.auto_commit_dirty` (**R077**), `cli.ui.narrator` (**R074**) — design only in linked hubs.

**Removed (R069 / ADR 0034 / R082):** `agent.max_tool_steps`, `agent.max_tool_steps_ask`, `agent.max_tool_steps_plan`, `agent.soft_cap_enabled`, `agent.soft_cap_fraction`, `agent.soft_cap_step_extension`, `observability.custom_sidecar_metrics` — ignored if present in older files.

**Removed (cwd-only workspace binding):** `workspace.root`, `workspace.allow_cwd_fallback` — ignored if present in older files. Run `rex` from the project directory; broker and daemon scope to that cwd.

Minimal example:

```json
{
 "version": 1,
 "daemon": { "socket": "/tmp/rex.sock" },
 "sidecars": {
 "active": "agent",
 "required": true,
 "list": [
 { "name": "agent", "binary": "rex-agent", "enabled": true, "socket": "/tmp/rex-sidecar.sock" }
 ]
 },
 "inference": {
 "runtime": "http-openai-compat",
 "openai_compat": {
 "base_url": "http://127.0.0.1:11434/v1",
 "model": "llama3.2",
 "native_tools": "auto"
 }
 },
 "observability": {
 "enabled": false,
 "service_name": "rex-daemon",
 "otlp": {
 "endpoint": "http://127.0.0.1:4317",
 "protocol": "grpc"
 }
 }
}
```

**Workspace root (product path):** Run `rex` from the project directory. The daemon resolves workspace as canonical **cwd**; per-workspace sockets and broker sandbox derive from that path. Optional `workspace.indexer` in project `.rex/config.json` selects lexical vs seeded indexer mode only.

## Observability

**Status:** **LangFuse Cloud** is the product observability path ([LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md)). Rex-owned store, read API, Grafana plugin, and `rex obs` were removed in **LF-R01**.

When `observability.enabled` is `true`, the daemon emits economics on **stdout** and exports **OTLP metrics** when `observability.otlp.endpoint` is set. **LF-F01** wires the default endpoint toward LangFuse Cloud. When `false` or omitted, phase 0 **stdout grep** only.

| Key | Default | Purpose |
|-----|---------|---------|
| `observability.enabled` | `false` | Master switch for OTLP export path |
| `observability.service_name` | `rex-daemon` | OTel resource `service.name` |
| `observability.otlp.endpoint` | (none) | OTLP metrics URL (LangFuse Cloud when **LF-F01** lands) |
| `observability.otlp.protocol` | `grpc` | `grpc` or `http/protobuf` |

Legacy `store`, `read_api`, `ui`, and `custom_sidecar_metrics` keys in older config files are ignored at load time.

## CLI daemon ensure (**R071** — implemented)

Design hub: [CLI_OPERATOR_UX.md](CLI_OPERATOR_UX.md). Decision: [ADR 0035](architecture/decisions/0035-cli-operator-ux-daemon-lifecycle-and-terminal-ui.md).

Running **`rex`** always ensures a daemon: probe UDS, spawn a detached process when needed, poll **`GetSystemStatus`** until ready.

| Key | Default | Purpose |
|-----|---------|---------|
| `daemon.ready_timeout_secs` | `10` | Readiness poll budget after spawn |
| `daemon.idle_shutdown_secs` | **`300`** | Shutdown after this many seconds without work and without status contact; **`0`** disables |
| `daemon.log_path` | `$REX_ROOT/daemon.log` | Detached daemon stdout/stderr |

When **`inference.omlx.mode: managed`** or **`inference.gateway.mode: managed`**, the ensured daemon also starts and health-checks that managed inference child before binding the UDS socket. Raise **`daemon.ready_timeout_secs`** if the managed child startup budget (for example oMLX **`startup_timeout_secs`**, default 30) exceeds the default ready poll (**10s**).

### CLI operator UX (`cli.ui` — implemented)

| Key | Default | Purpose |
|-----|---------|---------|
| `cli.ui.sync_output` | `true` | Emit terminal `?2026` synchronized output when supported |
| `cli.ui.session_title_refresh_turns` | `3` | Daemon KEEP-or-rename title fallback cadence (completed operator turns) when the agent did not call `session.set_title` |

Hub: [CLI_OPERATOR_UX.md](CLI_OPERATOR_UX.md), [TERMINAL_HARNESS_ARCHITECTURE.md](TERMINAL_HARNESS_ARCHITECTURE.md), [ADR 0039](architecture/decisions/0039-terminal-harness-presentation-and-daemon-intelligence.md).

**Module map:** Daemon: `settings`, `adapters`, `http_openai_compat`, `approvals`, `l1_cache`, stream service. CLI: `transport` (config socket), TUI ensure path.

## Operator quick start (daemon + brokered HTTP)

The product path requires a **supervised sidecar** ([MVP_SPEC.md](MVP_SPEC.md)). Bootstrap JSON; **`rex`** ensures the daemon (**R071**):

```bash
rex config init
# Edit $REX_ROOT/config.json — set inference.openai_compat.base_url and model; enable sidecars.list[].enabled
rex config validate
rex
```


For **Anthropic, OpenAI, and local Ollama** via one broker URL, use the [Inference Gateway](#inference-gateway-design) (`managed` or `external`) or the [LiteLLM operator profile](#operator-profile-litellm-anthropic-and-other-providers) below.

Example HTTP backend (Ollama) in `$REX_ROOT/config.json`:

```json
"inference": {
 "runtime": "http-openai-compat",
 "openai_compat": {
 "base_url": "http://127.0.0.1:11434/v1",
 "model": "llama3.2",
 "native_tools": "auto"
 }
}
```

**`inference.openai_compat.native_tools`** (**R038**): tri-state `auto` \| `true` \| `false`; schema default **`auto`** (omit field → `auto`). Controls whether daemon forwards OpenAI `tools[]` on `BrokerInference` vs interim JSON-in-text. **`auto`** probes Ollama `/api/show` for `tools` capability; direct Ollama is the reference path for agent tool calling — [NATIVE_TOOL_CALLING.md](NATIVE_TOOL_CALLING.md). `mock` / `cursor-cli` runtimes always use interim regardless of config.

### `inference.openai_compat` keys

| Key | Default | Purpose |
|-----|---------|---------|
| `base_url` | (none) | OpenAI-compat API root (for example `http://127.0.0.1:11434/v1`). Required when `runtime` is `http-openai-compat` unless a managed child (gateway or oMLX) injects URL — see [Effective `openai_compat.base_url`](#effective-openai_compatbase_url). |
| `api_key` | (none) | Optional `Authorization: Bearer` token when `headers` does not already set `Authorization`. |
| `model` | `gpt-4o-mini` | Default model id on chat/completions requests. |
| `timeout_secs` | `120` | Upper bound for a single HTTP completion request. |
| `native_tools` | `auto` | Tri-state native tool forwarding — see above. |
| `headers` | `{}` | Optional map of extra HTTP request headers (for example `X-Api-Key`) sent on every broker inference POST. Project `.rex/config.json` overlay merges keys into global headers. Invalid names or values fail `rex config validate`. Rex always sets `Content-Type: application/json` after configured headers. |

Example with custom auth header:

```json
"openai_compat": {
 "base_url": "https://my-gateway.example/v1",
 "model": "my-model",
 "headers": {
 "X-Api-Key": "secret-token"
 }
}
```

CI and unit tests set `REX_ROOT` to a temp dir and write `config.json` with `inference.runtime: "mock"` and `sidecars.harness: "direct"` — see [CI.md](CI.md).

## Inference Gateway (design)

**Status:** **implemented** — [INFERENCE_GATEWAY.md](INFERENCE_GATEWAY.md), [ADR 0019](architecture/decisions/0019-inference-gateway-opt-in-litellm.md). Bootstrap with `rex gateway init`; validate with `rex gateway doctor`.

### Purpose

Opt-in **`inference.gateway.mode: managed`** so `rex-daemon` spawns and controls a local LiteLLM proxy; **`external`** keeps an operator-run URL; **`disabled`** leaves gateway off (direct `openai_compat.base_url` or `mock`).

### Example (`managed` — design intent)

```json
{
 "inference": {
 "runtime": "http-openai-compat",
 "gateway": {
 "mode": "managed",
 "port": 4000,
 "ollama": { "enabled": true, "api_base": "http://127.0.0.1:11434" }
 },
 "openai_compat": {
 "model": "claude-sonnet-4-20250514",
 "timeout_secs": 120
 }
 }
}
```

Effective `openai_compat.base_url` becomes `http://127.0.0.1:4000/v1` when managed (unless override allowed). Secrets: `$REX_ROOT/gateway/.env` (gitignored). See hub for full field table and Ollama discovery template.

## Inference oMLX

**Status:** **implemented** — config schema, daemon supervisor, `rex omlx init|doctor`, opt-in live E2E — [OMLX_INFERENCE.md](OMLX_INFERENCE.md), [ADR 0033](architecture/decisions/0033-omlx-managed-local-inference.md).

### Purpose

Opt-in **`inference.omlx.mode: managed`** so `rex-daemon` spawns and controls a local oMLX server on Apple Silicon; **`external`** keeps an operator-run URL; **`disabled`** leaves oMLX lifecycle off (direct `openai_compat.base_url` or `mock`).

Managed oMLX uses the **same broker API** as every other OpenAI-compat backend: Rex injects `inference.openai_compat.base_url` and calls `POST …/chat/completions` only. oMLX’s Anthropic Messages and other upstream APIs are **not** Rex surfaces.

When `mode: managed`, the daemon supervisor spawns `omlx serve --port {port}` and optional `--model-dir {model_dir}` from **`inference.omlx.*` in JSON** — Rex does not configure oMLX via `OMLX_*` environment variables.

### Example (`managed` — design intent)

```json
{
 "inference": {
 "runtime": "http-openai-compat",
 "omlx": {
 "mode": "managed",
 "port": 8000,
 "model": "qwen2.5-coder-32b"
 },
 "openai_compat": {
 "native_tools": "auto"
 }
 }
}
```

### `inference.omlx` keys (intent)

| Key | Default | Purpose |
|-----|---------|---------|
| `mode` | `disabled` | `disabled` \| `external` \| `managed` |
| `port` | `8000` | Loopback port when `managed` |
| `command` | `omlx` on PATH | Spawn command override |
| `model_dir` | (none) | MLX weights directory |
| `model` | (none) | Default model id when `openai_compat.model` unset |
| `health_path` | `/v1/models` | Readiness probe path |
| `discovery_on_ready` | `true` | Optional `GET /v1/models` after health |
| `startup_timeout_secs` | `30` | Health wait budget |
| `required` | `true` when `managed` | Daemon ready blocked if child fails |
| `allow_url_override` | `false` | Allow non-empty `openai_compat.base_url` to override injection |

Hub: [OMLX_INFERENCE.md](OMLX_INFERENCE.md).

## Effective `openai_compat.base_url`

Rex resolves **one** broker URL for `http_openai_compat`. Managed gateway and managed oMLX are **lifecycle helpers** that inject into `openai_compat.base_url` — not separate Rex APIs.

| Priority | Condition | Effective URL |
|----------|-----------|---------------|
| 1 | Non-empty `openai_compat.base_url` and managed `allow_url_override` on active managed child | Configured URL |
| 2 | `inference.omlx.mode: managed` | `http://127.0.0.1:{omlx.port}/v1` |
| 3 | `inference.gateway.mode: managed` | `http://127.0.0.1:{gateway.port}/v1` |
| 4 | Otherwise | Configured `openai_compat.base_url` or broker error at request time |

**Mutual exclusion:** `rex config validate` **fails** if both `inference.omlx.mode: managed` and `inference.gateway.mode: managed`. Enable at most one managed injector.

Canonical table: [OMLX_INFERENCE.md](OMLX_INFERENCE.md#effective-openai_compatbase_url).

## Operator profile: LiteLLM (Anthropic and other providers)

**Status:** operator-ready on existing `http-openai-compat` runtime when gateway is **external**. Design: [INFERENCE_GATEWAY.md](INFERENCE_GATEWAY.md), [ADAPTERS.md](ADAPTERS.md#multi-provider-gateway-via-litellm-default-api), [ADR 0018](architecture/decisions/0018-gateway-first-multi-provider-inference.md), [ADR 0019](architecture/decisions/0019-inference-gateway-opt-in-litellm.md).

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
 "active": "agent",
 "required": true,
 "list": [
 { "name": "agent", "binary": "rex-agent", "enabled": true, "socket": "/tmp/rex-sidecar.sock" }
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

## CLI operation feedback

Keys under `cli` and `search` control stream idle timeouts and ask-mode `web.search` ([OPERATION_FEEDBACK.md](OPERATION_FEEDBACK.md)):

| Key | Default | Purpose |
|-----|---------|---------|
| `cli.stream_idle_timeout_secs_agent` | `120` | Per-chunk idle timeout for `agent` NDJSON streams |
| `cli.stream_idle_timeout_secs_ask` | `120` | Per-chunk idle timeout for `ask` / `plan` NDJSON streams |
| `search.enabled` | `false` (operator init: `true`) | Enables broker `web.search` (ask mode only) |
| `search.provider` | — (operator init: `mock`) | `mock` for local demos |
| `search.max_results` | `5` | Max hits returned per query |
| `agent.max_tools_per_step` | `8` | Max batchable read/list/search broker calls per LLM round (**R057**) |
| `agent.deterministic_init_enabled` | `true` | Pre-LLM ask init (`fs.read` README + `fs.list`) before first inference (**R060**) |
| `agent.compaction_enabled` | `false` | Intra-turn suffix compaction node (**R029**, **R062** — off by default to preserve prefix cache) |
| `agent.compaction_suffix_fraction` | `0.25` | Compaction threshold as a fraction of `broker.max_tool_result_bytes` |
| `agent.read_pruning_enabled` | `false` | Goal-hint read pruning (**R031**) |

CLI flags: `rex complete --verbose` (stderr status in text mode), `--yes` / `--approval-id` for agent approval automation.

## CI and harness (not operator product config)

These keys exist for automated tests and local harnesses. Prefer `RexConfig::defaults` in code or an explicit test `config.json` under a temp `REX_ROOT`. Do **not** copy them into operator installs from `rex config init`.

| Key | Typical harness value | Purpose |
|-----|----------------------|---------|
| `inference.runtime` | `"mock"` | Deterministic broker without network |
| `sidecars.active` / `list[]` | `stub` / `rex-sidecar-stub` | CI sidecar without Python agent |
| `sidecars.harness` | `"direct"` | Skip sidecar spawn; in-process inference |
| `workspace.indexer` | `"seeded"` | In-memory docs for tests |
| `cache.bypass` | `true` | Disable L1 / prefix cache for diagnostics |

Product operators use **`rex-agent`**, a live `http-openai-compat` backend, and run `rex` from the project directory. See [CI.md](CI.md).

## Not implemented yet (roadmap)

- Global CLI flags mirroring all JSON keys — partial today (`rex complete` flags only).
- Layered prompt assemblies — see **Layered prompts** below.

## See also

- [ARCHITECTURE.md](ARCHITECTURE.md)
- [MVP_SPEC.md](MVP_SPEC.md)
- [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md)
- [ADAPTERS.md](ADAPTERS.md)
- [CACHING.md](CACHING.md)
- [NDJSON_STREAM.md](NDJSON_STREAM.md)
- [OPERATION_FEEDBACK.md](OPERATION_FEEDBACK.md)
- [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md)
- [ECONOMICS_VALIDATION.md](ECONOMICS_VALIDATION.md)
