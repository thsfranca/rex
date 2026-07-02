# REX configuration

This document is the **canonical** policy for how REX settings work: merged **JSON** under `$REX_ROOT`, optional project overrides, and partial CLI flags. See [ARCHITECTURE.md](ARCHITECTURE.md) for where the **daemon** applies inference and cache policy.

## Configuration surface

- **Product settings:** `$REX_ROOT/config.json` and `.rex/config.json` — not `REX_*` tuning environment variables.
- **Bootstrap env:** **`REX_ROOT`** only (optional). Points at the layout directory (`config.json`, protos, observability store path). Defaults to `~/.rex` when unset.
- **Secrets:** API keys may use env or OS keychain alongside JSON fields — see below. Observability and inference are **not** configured via `REX_OBS_*` or similar legacy names.

## Why this policy exists

- **Developer experience:** One merged config file per machine; `rex config show` for inspection.
- **Automation:** CI, scripts, and the CLI client set **`REX_ROOT`** and write JSON; legacy `REX_*` tuning vars are ignored with a startup warning.
- **One catalog:** JSON keys, bootstrap commands, and deprecated env tables for migration only.

## Precedence (implemented)

| Precedence (low to high) | Role |
|--------------------------|------|
| Built-in defaults | Used when a JSON field is unset. |
| `$REX_ROOT/config.json` | User defaults: daemon socket, sidecars, inference, workspace, broker, agent. |
| `.rex/config.json` | Optional project overrides (walked from cwd upward). |
| CLI flags (partial) | `rex complete` accepts `--model`, `--mode`, `--approval-id`, `--yes`, `--verbose`, `--trace-id`, `--active-file`, `--language-id`, `--selection-text` per invocation. `rex status` and `rex complete` accept **`--no-daemon-autostart`**. |

**Layout root:** `$REX_ROOT` defaults to `~/.rex` when unset. Run `rex config init` to create the layout and operator template `config.json` (**`sidecars.active: agent`**, **`rex-agent`** enabled, **`search.enabled: true`** with **`search.provider: mock`**). Operators typically edit only **`inference.openai_compat`** for a live backend. CI and harness tests use explicit stub configs or `RexConfig::defaults` in code — not the init template.

**Legacy environment variables:** Daemon startup ignores former `REX_INFERENCE_RUNTIME`, `REX_OPENAI_COMPAT_*`, `REX_SIDECAR_*`, `REX_DAEMON_SOCKET`, and `REX_WORKSPACE_ROOT` when present and prints a warning — use JSON instead. **`REX_AGENT_APPROVALS`** is **not read** (migration reference only; canonical key is `agent.approvals_enabled` in JSON — [V1_0.md](V1_0.md) RC-06). **`REX_ROOT`** remains the bootstrap override for layout location (tests, extension auto-start).

**Secret values:** Prefer environment or OS keychain for API keys and auth header values in JSON (`inference.openai_compat.api_key`, `inference.openai_compat.headers`). Do not commit secrets to the repository.

## JSON configuration keys (implemented)

Bootstrap: `rex config init|show|path|validate`, `rex sidecar list|init|doctor`, `rex proto install|path|doctor`.

| Section | Keys | Purpose |
|---------|------|---------|
| `daemon` | `socket`, `socket_scope`, `auto_start`, `ready_timeout_secs`, `idle_shutdown_secs`, `log_path` | Daemon UDS path; **`socket_scope`** (`per_workspace` default, or `global` for legacy single socket); **auto-start on by default** (**R071**); readiness poll budget; idle auto-shutdown (**default 300s**, **`0`** disables); detached daemon log file. Per-workspace scope derives `$REX_ROOT/sockets/ws-<hash>.sock` from `workspace.root` — [ADR 0036](architecture/decisions/0036-per-workspace-daemon-routing.md), idle shutdown — [ADR 0037](architecture/decisions/0037-daemon-idle-shutdown.md). |
| `sidecars` | `active`, `host`, `required`, `harness`, `list[]`, `capabilities[]` | Host sidecar (`list[]` entry named by `host` or `active`); optional capability fleet (`capabilities[]` with `provides`, `socket`, `binary`); `harness: "direct"` skips spawn (CI/tests). |
| `inference` | `runtime`, `openai_compat`, `cursor_cli` | Broker backend: `mock`, `http-openai-compat`, `cursor-cli`. |
| `workspace` | `root`, `indexer`, `allow_cwd_fallback` | Broker root and lexical indexer (`workspace` or `seeded`). Product path requires non-empty `root` (not `"."`). Harness/CI: `allow_cwd_fallback: true` or `REX_ALLOW_CWD_WORKSPACE=1`. |
| `context` | `max_prompt_tokens`, `max_context_tokens` | Context pipeline budgets. |
| `context` | `advisory_intent_enabled` | **Planned (R067):** when true (default), short advisory prompts bypass the ≤48 char retrieval skip. |
| `cache` | `bypass` | L1 / prefix cache bypass. |
| `broker` | `shell_allowlist`, `max_tool_result_bytes` | Allowed `exec.shell` programs; max bytes returned from `fs.read` and `exec.shell` stdout/stderr (default **8192**). Write upload cap remains **65536** bytes per request. |
| `agent` | `approvals_enabled`, `max_tools_per_step`, `compaction_suffix_fraction`, `read_pruning_enabled`, `deterministic_init_enabled` | Agent-mode approval gate; max batchable broker calls per LLM round (default **8**, **R057**); intra-turn compaction; goal-hint read pruning (**R031** Done, default off); pre-LLM ask init (**R060**). |
| `cli` | `stream_idle_timeout_secs_agent`, `stream_idle_timeout_secs_ask` | Per-chunk idle timeout for `rex complete` streams (defaults **120** for all modes). |
| `search` | `enabled`, `provider`, `max_results`, `api_key_path` | Ask-mode `web.search` broker (`provider: mock` for local demos). **R055** will migrate to capability sidecar — [WEB_SEARCH.md](WEB_SEARCH.md). |

**Capability sidecar entry (`capabilities[]`):** `name`, `binary`, `enabled`, `socket`, `provides` (capability ids, e.g. `web.search`), optional `required`. Daemon spawns enabled entries alongside the host; invoke routing is **R056-2** — [CAPABILITY_SIDECARS.md](CAPABILITY_SIDECARS.md).

**Planned (design — not implemented):** `broker.web_search` (enablement, timeouts, rate limits, cache TTL) — [WEB_SEARCH.md](WEB_SEARCH.md).
| `observability` | `enabled`, `service_name`, `custom_sidecar_metrics`, `otlp` | OTLP export + stdout economics — [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md), [Observability](#observability) |

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
 "workspace": { "root": "/absolute/path/to/your/project" },
 "observability": {
 "enabled": false,
 "service_name": "rex-daemon",
 "custom_sidecar_metrics": true,
 "otlp": {
 "endpoint": "http://127.0.0.1:4317",
 "protocol": "grpc"
 }
 }
}
```

**Workspace root (product path):** Set `workspace.root` to an absolute project path in `.rex/config.json` (extension auto-start writes this when a folder is open). Unset or `"."` without `allow_cwd_fallback` causes broker and `StreamInference` to fail closed. For harness tests only: `workspace.allow_cwd_fallback: true` in JSON or `REX_ALLOW_CWD_WORKSPACE=1` in the environment.

## Observability

**Status:** **LangFuse Cloud** is the product observability path ([LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md)). Rex-owned store, read API, Grafana plugin, and `rex obs` were removed in **LF-R01**.

When `observability.enabled` is `true`, the daemon emits economics on **stdout** and exports **OTLP metrics** when `observability.otlp.endpoint` is set. **LF-F01** wires the default endpoint toward LangFuse Cloud. When `false` or omitted, phase 0 **stdout grep** only.

| Key | Default | Purpose |
|-----|---------|---------|
| `observability.enabled` | `false` | Master switch for OTLP export path |
| `observability.service_name` | `rex-daemon` | OTel resource `service.name` |
| `observability.custom_sidecar_metrics` | `true` | Reserved for future sidecar OTLP ingest (**LF-F06**) |
| `observability.otlp.endpoint` | (none) | OTLP metrics URL (LangFuse Cloud when **LF-F01** lands) |
| `observability.otlp.protocol` | `grpc` | `grpc` or `http/protobuf` |

Legacy `store`, `read_api`, and `ui` keys in older config files are ignored at load time.

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
| `REX_AGENT_APPROVALS` | — | **Deprecated / not read.** Use `agent.approvals_enabled` in JSON instead ([ADR 0009](architecture/decisions/0009-centralized-agent-approvals-and-checkpoints.md), [V1_0.md](V1_0.md) RC-06). When enabled, pass `approval_id` on `StreamInference` (via `rex complete --approval-id`) after extension approval. |

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

**CLI flags:** `rex complete` accepts `--format`, `--model <id>`, `--mode <ask|plan|agent>`, `--approval-id <id>`, and `--trace-id <id>`. `rex status` and `rex complete` accept **`--no-daemon-autostart`**. Trace correlation uses **`--trace-id`** (not `REX_TRACE_ID`). Unset model uses daemon default; empty mode normalizes to **`ask`** on the server ([`MVP_SPEC.md`](MVP_SPEC.md), [`CACHING.md`](CACHING.md)).

The CLI reads merged JSON for UDS transport and daemon lifecycle (`rex_config::load_merged`).

### CLI daemon auto-start (**R071** — implemented)

Design hub: [CLI_OPERATOR_UX.md](CLI_OPERATOR_UX.md). Decision: [ADR 0035](architecture/decisions/0035-cli-operator-ux-daemon-lifecycle-and-terminal-ui.md).

| Key | Default | Purpose |
|-----|---------|---------|
| `daemon.auto_start` | **`true`** | CLI spawns detached `rex daemon` when socket is missing |
| `daemon.ready_timeout_secs` | `10` | Readiness poll budget after spawn |
| `daemon.idle_shutdown_secs` | **`300`** | Shutdown after this many seconds without work and without status contact; **`0`** disables |
| `daemon.log_path` | `$REX_ROOT/daemon.log` | Detached daemon stdout/stderr |

Opt out: `"auto_start": false` in merged JSON or **`--no-daemon-autostart`**. Extension **`rex.daemonAutoStart`** defaults **on** — [ROADMAP.md](ROADMAP.md).

When **`inference.omlx.mode: managed`** or **`inference.gateway.mode: managed`**, the autostarted daemon also starts and health-checks that managed inference child before binding the UDS socket. Raise **`daemon.ready_timeout_secs`** if the managed child startup budget (for example oMLX **`startup_timeout_secs`**, default 30) exceeds the default ready poll (**10s**).

### CLI operator UX (planned — R072–R074)

| Key | Default (design) | Purpose |
|-----|------------------|---------|
| `cli.ui.enabled` | `"auto"` | TUI on TTY: `auto` \| `true` \| `false` |
| `cli.ui.narrator` | `false` | Optional post-turn LLM summary (**R074**) |
| `cli.ui.sync_output` | `true` | Emit terminal `?2026` synchronized output when supported |
| `git.auto_commit_dirty` | `true` | Daemon broker auto-commits dirty files before **`fs.write`** (**R077**) |

CLI flags (planned): `--no-ui`.

Hub: [CLI_OPERATOR_UX.md](CLI_OPERATOR_UX.md), [TERMINAL_HARNESS_ARCHITECTURE.md](TERMINAL_HARNESS_ARCHITECTURE.md), [ADR 0039](architecture/decisions/0039-terminal-harness-presentation-and-daemon-intelligence.md).

**Module map:** Daemon: `settings`, `adapters`, `http_openai_compat`, `approvals`, `l1_cache`, stream service. CLI: `transport` (config socket), `runtime` (`--trace-id`).

## Operator quick start (daemon + brokered HTTP)

The product path requires a **supervised sidecar** ([MVP_SPEC.md](MVP_SPEC.md)). Bootstrap JSON; **`rex status`** / **`rex complete`** auto-start the daemon by default (**R071**):

```bash
rex config init
# Edit $REX_ROOT/config.json — set inference.openai_compat.base_url and model; enable sidecars.list[].enabled
rex config validate
rex status # starts detached daemon when needed
```

Foreground **`rex daemon`** remains for debugging. Opt out with **`daemon.auto_start: false`** or **`--no-daemon-autostart`**.

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

### Deprecated (R069 — ignored after implementation)

| Key | Former purpose |
|-----|----------------|
| `agent.max_tool_steps_ask` | Ask-mode tool loop hard cap |
| `agent.max_tool_steps_plan` | Plan-mode tool loop hard cap |
| `agent.max_tool_steps` | Agent-mode tool loop hard cap |
| `agent.soft_cap_enabled` | Soft pause before hard cap (**R063**, superseded by [ADR 0034](architecture/decisions/0034-remove-tool-step-caps.md)) |
| `agent.soft_cap_fraction` | Soft cap threshold fraction |
| `agent.soft_cap_step_extension` | Steps added on Continue |

### Planned (R067)

| Key | Default | Purpose |
|-----|---------|---------|
| `context.advisory_intent_enabled` | `true` | Advisory intent bypass for short-prompt retrieval — [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md#advisory-intent-retrieval-r067) |

CLI flags: `rex complete --verbose` (stderr status in text mode), `--yes` / `--approval-id` for agent approval automation.

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
