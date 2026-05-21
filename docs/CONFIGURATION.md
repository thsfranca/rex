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
| CLI flags (partial) | `rex-cli complete` accepts `--model` and `--mode` per invocation. |

**Secret values:** Prefer environment or OS keychain for API keys. Do not commit secrets to the repository.

## Phase 1: environment variables (implemented)

### `rex-daemon` (inference and cache)

| Variable | Default (if unset) | Purpose |
|----------|--------------------|---------|
| `REX_INFERENCE_RUNTIME` | `http-openai-compat` | Broker backend when sidecar requests inference: **`http-openai-compat`**, **`mock`** (tests/harness), **`cursor-cli`** (legacy). Direct daemon HTTP without sidecar is **harness only** for MVP acceptance. |
| `REX_OPENAI_COMPAT_BASE_URL` | (none — **required** for HTTP runtime) | Base URL for OpenAI-compatible API (for example `http://127.0.0.1:11434/v1` for Ollama). |
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
| `REX_AGENT_APPROVALS` | off | `1` or `true` enforces daemon `ApprovalGate` for `agent` mode ([ADR 0009](architecture/decisions/0009-centralized-agent-approvals-and-checkpoints.md)). Pass `approval_id` on `StreamInference` (via `rex-cli --approval-id`) after extension approval. |

### Sidecar supervision and harness

| Variable | Default (if unset) | Purpose |
|----------|--------------------|---------|
| `REX_SIDECAR_ENABLED` | off | `1`/`true` enables spawn; product path uses sidecar when enabled |
| `REX_SIDECAR_REQUIRED` | on when enabled | `0` makes sidecar optional (daemon starts without hard fail) |
| `REX_SIDECAR_BINARY` | `rex-sidecar-stub` on `PATH` | Sidecar executable for supervision |
| `REX_SIDECAR_SOCKET` | `/tmp/rex-sidecar.sock` | UDS for `rex.sidecar.v1` — [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md) |
| `REX_SIDECAR_HARNESS` | (none) | `direct` forces in-process inference (CI/tests); not MVP product acceptance |
| `REX_DAEMON_SOCKET` | `/tmp/rex.sock` | Daemon UDS for stub `BrokerReadFile` during tool turns |

### `rex-cli` (client metadata)

| Variable | Default (if unset) | Purpose |
|----------|--------------------|---------|
| `REX_TRACE_ID` | (none) | Request correlation; extension sets when spawning `rex-cli` — [`EXTENSION.md`](EXTENSION.md). |

**CLI flags:** `rex-cli complete` accepts `--format`, `--model <id>`, `--mode <ask|plan|agent>`, and `--approval-id <id>`. Unset model uses daemon default; empty mode normalizes to **`ask`** on the server ([`MVP_SPEC.md`](MVP_SPEC.md), [`CACHING.md`](CACHING.md)).

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
cargo run -p rex-daemon
```

CI and unit tests set `REX_INFERENCE_RUNTIME=mock` and clear `REX_OPENAI_COMPAT_BASE_URL` — see [CI.md](CI.md).

## Not implemented yet (roadmap)

- Persistent user config on disk.
- Global `rex-daemon` / `rex-cli` flags mirroring env keys.
- `rex config` subcommands.
- Project-local `.rex.toml`.

## See also

- [ARCHITECTURE.md](ARCHITECTURE.md)
- [MVP_SPEC.md](MVP_SPEC.md)
- [ADAPTERS.md](ADAPTERS.md)
- [CACHING.md](CACHING.md)
- [EXTENSION.md](EXTENSION.md)
