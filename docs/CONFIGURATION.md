# REX configuration

This document is the **canonical** policy for how REX settings work: what applies today, how precedence will work when you add new layers, and where each `REX_*` variable is read. See [ARCHITECTURE.md](ARCHITECTURE.md) for where the **daemon** applies inference and cache policy.

## Why this policy exists

- **Developer experience:** You can repeat the same run without retyping long `export` lines; future CLI flags and optional files will map to the same **names** as environment variables.
- **Automation:** CI, scripts, and the editor extension inject settings through the environment. That pattern stays first-class.
- **One catalog:** This file lists the variables the **Rust** binaries and the **core** tool flow use. Related scripts may define extra `REX_*` names; you find those next to the script in `docs/` or the script header.

## Precedence (target model)

Rex does **not** implement all layers below yet. **Phase 1 (today):** only **defaults** and **environment** apply.

| Precedence (low to high) | Role |
|--------------------------|------|
| Built-in defaults | Used when a setting is unset. |
| User persistent file (not implemented) | Optional file under the [XDG Base Directory](https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html) convention, e.g. `$XDG_CONFIG_HOME/rex` with default `$HOME/.config/rex` when `XDG_CONFIG_HOME` is unset. |
| Project-local file (not implemented) | Optional repo-local file (for example `.rex.toml`); use `.gitignore` for machine-specific files; do **not** commit secrets. |
| Environment variables | **Primary** for parents (extension, tests, CI) and for overrides today. |
| CLI flags (not implemented) | **Per-invocation** overrides; **document** 1:1 with env keys so shell history and `--help` stay consistent. |

**Secret values:** Do not place API keys in committed project files. Prefer environment, OS keychain, or a **local** file with correct permissions. If you add a user config file later, document redaction in logs the same way you would for env.

**Boundaries:** `rex-daemon` applies **inference and cache** policy from the effective config it reads. `rex-cli` remains **thin**; it may set metadata (for example trace id) and in the future can accept **global** flags. Avoid inventing a second set of ad hoc names: extend this catalog when you add a new knob.

## Phase 1: environment variables (implemented)

The following are read in **Rust** code paths for the **daemon** or **CLI** as indicated.

### `rex-daemon` (inference and cache)

| Variable | Default (if unset) | Purpose |
|----------|--------------------|---------|
| `REX_INFERENCE_RUNTIME` | `mock` (any value other than `cursor` / `cursor-cli` selects mock) | Selects the inference adapter: `mock` or Cursor CLI. |
| `REX_CURSOR_CLI_PATH` | `cursor-agent` | Executable used when the runtime is `cursor` / `cursor-cli`. |
| `REX_CURSOR_CLI_COMMAND` | (none) | Optional full shell command template; `{prompt}` is substituted. |
| `REX_CURSOR_CLI_TIMEOUT_SECS` | `20` | Bound for the adapter subprocess, in seconds. |
| `REX_CACHE_BYPASS` | off | Set to `1` or `true` to bypass the **L1** response cache and the context **prefix** cache (diagnostics). See [`CACHING.md`](CACHING.md). |
| `REX_WORKSPACE_ROOT` | (default fingerprint) | Optional absolute path to the current workspace. When set, it scopes the in-memory L1 **exact** response key so a cache entry from one checkout does not match another. If unset, the daemon uses a stable default (same process always matches). |

### `rex-cli` (client metadata)

| Variable | Default (if unset) | Purpose |
|----------|--------------------|---------|
| `REX_TRACE_ID` | (none) | If set, the CLI propagates it for request correlation. The extension sets this when it runs `rex-cli`; see [`EXTENSION_MVP.md`](EXTENSION_MVP.md). |

**CLI flags (not environment):** `rex-cli complete` accepts optional `--model <id>` and `--mode <ask|plan|agent>` in any order with `--format`. Empty values behave like the defaults documented in the protobuf ([`MVP_SPEC.md`](MVP_SPEC.md)): unset model leaves model selection to the daemon default; empty mode is treated as **ask** on the server for inference policy (L1 only caches **ask**; see `CACHING.md`).

### Related project scripts (not `rex-daemon` / `rex-cli` binaries)

| Variable | Where it matters |
|----------|------------------|
| `REX_EXTENSION_EDITOR` | [install-extension.sh](../scripts/install-extension.sh) and [EXTENSION_LOCAL_E2E.md](EXTENSION_LOCAL_E2E.md): path to the `cursor` or `code` CLI when the shell `PATH` is wrong. |
| `REX_TEST_STATUS_STATE_FILE` | Test fixtures only: controls scripted `rex-cli status` success/fail sequences in the extension tests. |

**Code map (audit):** Daemon keys are read in `crates/rex-daemon/src/adapters.rs` (`REX_INFERENCE_RUNTIME`, Cursor CLI trio), `crates/rex-daemon/src/service.rs` (`REX_CACHE_BYPASS`), and `crates/rex-daemon/src/l1_cache.rs` (`REX_WORKSPACE_ROOT`). The CLI reads `REX_TRACE_ID` in `crates/rex-cli/src/runtime.rs`. If you add a new `REX_*` for a binary, extend this catalog and the table above in the same change.

Deeper treatment of the Cursor adapter path: [`PLUGIN_ROADMAP.md`](PLUGIN_ROADMAP.md), [`DEPENDENCIES.md`](DEPENDENCIES.md), [`ADAPTERS.md`](ADAPTERS.md).

**Cursor CLI:** On non-zero exit, the daemon may include stderr in the terminal gRPC `unavailable` message. The adapter **truncates** overlong stderr (marker ` [rex: cursor stderr truncated] `) so error payloads stay bounded. Run `cursor-agent` in a shell if you need the full transcript.

## Not implemented yet (roadmap)

- **Persistent user config** on disk (TOML or JSON under XDG paths).
- **`rex` / `rex-daemon` global flags** that mirror the table above (same semantics as env, higher precedence than env when both are set).
- **`rex config` subcommands** (for example get, set, path) to edit the user file without hand-editing.
- **Project-local** `.rex.toml` (or similar) and merge rules with user + env.

If you add any of the above, update this file **and** the precedence table in the same change.

## See also

- [ARCHITECTURE.md](ARCHITECTURE.md) — system shape and data flow.
- [MVP_SPEC.md](MVP_SPEC.md) — which configuration **documentation** is in scope for Phase 1 versus which **implementation** ships later.
- [`CACHING.md`](CACHING.md) — `REX_CACHE_BYPASS` and cache behavior.
- [`EXTENSION_MVP.md`](EXTENSION_MVP.md) — `REX_TRACE_ID` from the editor.
