# REX configuration


> Role: reference | Status: active | Audience: contributors | Read when: JSON config keys
> Prefer: ## JSON configuration keys


## Summary

Canonical JSON config catalog under `$REX_ROOT`: precedence, key tables, and deprecated env policy. Operator walkthroughs live in CONFIGURATION_OPERATOR.md.

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
| `context` | `max_prompt_tokens`, `max_context_tokens`, `advisory_intent_enabled`, `paths[]` | Context pipeline budgets; advisory bundle toggle (**R067**); pre-inject paths (**R066**). |
| `cache` | `bypass` | L1 / prefix cache bypass. |
| `broker` | `shell_allowlist`, `max_tool_result_bytes` | Allowed `exec.shell` programs; max bytes returned from `fs.read` and `exec.shell` stdout/stderr (default **8192**). Write upload cap remains **65536** bytes per request. |
| `agent` | `approvals_enabled`, `tool_approvals_enabled`, `max_tools_per_step`, `deterministic_init_enabled`, `compaction_enabled`, `compaction_suffix_fraction`, `read_pruning_enabled` | Agent-mode approval gates; max batchable broker calls per LLM round (default **8**, **R057**); pre-LLM ask init (**R060**); intra-turn compaction (**R029**, default off); compaction threshold fraction (default **0.25**); goal-hint read pruning (**R031**, default off). |
| `cli` | `stream_idle_timeout_secs_agent`, `stream_idle_timeout_secs_ask`, `ui` | Per-chunk idle timeouts (defaults **120**); `cli.ui.enabled` / `sync_output` for TUI (**R073**). |
| `search` | `enabled`, `provider`, `max_results`, `api_key_path` | Ask-mode `web.search` broker (`provider: mock` for local demos). **R055** will migrate to capability sidecar — [WEB_SEARCH.md](WEB_SEARCH.md). |
| `observability` | `enabled`, `service_name`, `otlp` | OTLP export + stdout economics — [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md), [Observability](#observability) |

**Capability sidecar entry (`capabilities[]`):** `name`, `binary`, `enabled`, `socket`, `provides` (capability ids, e.g. `web.search`), optional `required`. Daemon spawns enabled entries alongside the host; invoke routing is **R056-2** — [CAPABILITY_SIDECARS.md](CAPABILITY_SIDECARS.md).

**Not implemented (do not set):** `broker.web_search`, `git.auto_commit_dirty` (**R077**), `cli.ui.narrator` (**R074**) — design only in linked hubs.

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

Operator walkthroughs (bootstrap, gateway, oMLX, LiteLLM): [CONFIGURATION_OPERATOR.md](CONFIGURATION_OPERATOR.md).

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
- Layered prompt assemblies — [CONFIGURATION_OPERATOR.md](CONFIGURATION_OPERATOR.md#layered-prompts-design-accepted).

## See also

- [ARCHITECTURE.md](ARCHITECTURE.md)
- [MVP_SPEC.md](MVP_SPEC.md)
- [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md)
- [ADAPTERS.md](ADAPTERS.md)
- [CACHING.md](CACHING.md)
- [NDJSON_STREAM.md](NDJSON_STREAM.md)
- [OPERATION_FEEDBACK.md](OPERATION_FEEDBACK.md)
- [CONFIGURATION_OPERATOR.md](CONFIGURATION_OPERATOR.md)
- [ECONOMICS_VALIDATION.md](ECONOMICS_VALIDATION.md)
