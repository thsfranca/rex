# Local end-to-end: REX daemon, CLI, and VS Code / Cursor extension

This guide walks from a clean clone to **REX ready** in the editor: built binaries, `rex` visible to the extension host, daemon on `/tmp/rex.sock`, and the extension installed.

## Prerequisites

Full version matrix: [DEPENDENCIES.md](./DEPENDENCIES.md#operator-install-prerequisites-local-e2e). Quick summary:

| Requirement | Minimum | Install / check |
|---|---|---|
| Rust + `cargo` | stable | `cargo --version` |
| `protoc` | 3.x | `brew install protobuf` |
| Python (`rex-agent`) | **3.10+** | `./scripts/install-agent-sidecar.sh` (venv at `$REX_ROOT/venv`) |
| Node.js + `npm` | **20+** | Extension build only |
| Editor | VS Code engine **^1.120.0** | `cursor` or `code` on `PATH`; see [EXTENSION_RELEASE.md](./EXTENSION_RELEASE.md) |

**Operator preflight:** `./scripts/install-preflight.sh` prints pass/fail rows for the above (non-destructive). Use `--strict` before `./scripts/reinstall-dev.sh` when debugging install issues.

**CI preflight (maintainers):** `chmod +x ./scripts/verify_mvp_local.sh && ./scripts/verify_mvp_local.sh` — builds the workspace, runs Rust verify, sidecar verify, `mvp_product_path`, and extension checks ([CI.md](./CI.md)). Does **not** start the daemon. Use before manual editor checks in step 7 below.

## 1) Build the Rust workspace

From the repository root:

```bash
cargo build --workspace
```

This builds the full workspace (`rex-proto`, `rex-config`, `rex`, `rex-daemon`, `rex-cli`, `rex-sidecar-stub`, and related crates). For a quicker smoke test you can use `cargo run -p rex -- daemon` / `cargo run -p rex -- status` without installing binaries (see step 5 for editor settings if you stay on `cargo run`).

## 2) Put `rex` where the editor can run it

The extension spawns **`rex status`** and **`rex complete`** as child processes using the extension host environment. That environment often has a **smaller `PATH` than your terminal** (common when you launch Cursor or VS Code from the macOS Dock).

Recommended: install the unified binary into Cargo’s bin directory:

```bash
chmod +x ./scripts/install-cli.sh
./scripts/install-cli.sh
```

The script configures shell PATH for `~/.cargo/bin` by default, runs `rex config init` (operator template uses **`rex-agent`**), and updates the current shell session PATH.

To print absolute paths for manual editor settings, run `./scripts/install-cli.sh --print-bin-path`.

The extension **auto-discovers** `~/.cargo/bin/rex` when `rex.cliPath` and `rex.daemonBinaryPath` are left at the default `rex`. If the status bar still shows unavailable after install, set those settings explicitly or **fully quit and reopen** Cursor or VS Code (not only Reload Window).

## 3) Configure JSON (product path: `rex-agent` + live HTTP)

Phase 1 expects a **daemon-supervised sidecar** plus **brokered OpenAI-compatible HTTP** ([MVP_SPEC.md](./MVP_SPEC.md), [CONFIGURATION.md](./CONFIGURATION.md)). Use JSON under `$REX_ROOT` and project `.rex/config.json` — legacy `REX_*` tuning env vars are ignored.

From the repo root (with [Ollama](https://ollama.com/) or another OpenAI-compatible server on port 11434):

```bash
./scripts/reinstall-dev.sh
# or: ./scripts/install-cli.sh && ./scripts/install-agent-sidecar.sh
rex config show
```

Fresh `rex config init` (also run automatically by `install-cli.sh`) writes **`sidecars.active: "agent"`** with **`rex-agent`** enabled. Edit `$REX_ROOT/config.json` or create `.rex/config.json` in your project for inference and workspace:

```json
{
  "version": 1,
  "sidecars": {
    "active": "agent",
    "required": true,
    "list": [
      {
        "name": "agent",
        "binary": "rex-agent",
        "enabled": true,
        "socket": "/tmp/rex-sidecar.sock"
      }
    ]
  },
  "inference": {
    "runtime": "http-openai-compat",
    "openai_compat": {
      "base_url": "http://127.0.0.1:11434/v1",
      "model": "llama3.2"
    }
  },
  "workspace": { "root": "/absolute/path/to/your/project" }
}
```

Install scripts attempt the product sidecar automatically (`./scripts/install-cli.sh` or `./scripts/reinstall-dev.sh` call `scripts/install-agent-sidecar.sh`). The agent installer creates **`$REX_ROOT/venv`** and a **`~/.cargo/bin/rex-agent`** wrapper — it does **not** use macOS CLT Python 3.9 or system `pip` directly.

If rex-agent install fails, fix Python and re-run:

```bash
./scripts/install-preflight.sh
./scripts/install-agent-sidecar.sh
rex sidecar doctor
```

Manual maintainer path (editable into your own venv): `rex proto install` then `pip install -e sidecars/rex-agent` with Python **>= 3.10** — see [sidecars/rex-agent/README.md](../sidecars/rex-agent/README.md).

Ensure `rex-agent` is on `PATH` when the daemon starts (`rex sidecar doctor` checks this).

For automated preflight only (no live LLM), CI uses `inference.runtime: mock` and/or `sidecars.harness: "direct"` — not the operator acceptance path below.

The extension always merges `sidecars.active: agent` and `agent.approvals_enabled: true` into project `.rex/config.json` when binding the workspace.

## 4) Run `rex daemon`

**User-managed (default extension behavior)**

In a separate terminal from the repo root (with JSON from step 3):

```bash
cargo run -p rex -- daemon
```

Or, after `install-cli.sh`:

```bash
rex daemon
```

The daemon listens on **`/tmp/rex.sock`**.

**Extension-managed (opt-in)**

In editor settings, set `"rex.daemonAutoStart": true`. The extension writes `workspace.root` into project `.rex/config.json`, spawns `rex daemon` with `cwd` set to the primary workspace folder, and passes `REX_ROOT` when configured. If `rex` is not on the editor `PATH`, set `rex.daemonBinaryPath` and `rex.cliPath` to the **absolute path** to the same binary. Multi-root workspaces log `workspace.warning=multi_root` and bind the **primary** folder only.

## 5) Verify from a terminal

```bash
rex status
```

Or without install:

```bash
cargo run -p rex -- status
```

You should see daemon fields (version, uptime, model id). If this fails, fix the daemon or socket before opening the extension.

## 6) Install the extension

From the repo root:

```bash
chmod +x ./scripts/install-extension.sh
./scripts/install-extension.sh
```

See [EXTENSION_RELEASE.md](./EXTENSION_RELEASE.md) for flags (`--verify`, `--editor vscode`, `--no-reload`, `--only-install`).

Reload the window when prompted (or run **Developer: Reload Window**).

**One-shot helper** (install CLI + sidecar stub into `~/.cargo/bin`, then install the VSIX; does **not** start the daemon):

```bash
chmod +x ./scripts/reinstall-dev.sh
./scripts/reinstall-dev.sh
```

Pass through extra flags to `install-extension.sh`, for example:

```bash
./scripts/reinstall-dev.sh --verify
./scripts/reinstall-dev.sh --extension-only --only-install
```

[`scripts/dev-rex-extension.sh`](../scripts/dev-rex-extension.sh) delegates to `reinstall-dev.sh`.

## 7) Verify in the editor

- Status bar shows **REX ready** (or **REX starting** briefly).
- Command **REX: Show Daemon Status** returns a snapshot.
- Output channel **REX** logs activation and probe/auto-start decisions.

Open **REX: Open Chat**, select **agent** or **plan** mode, send a short prompt, and confirm streaming completes **via the sidecar path** (daemon logs should show sidecar turn + broker). Exercise **cancel** and **Apply** on a code block (approval in non-ask modes). Verify a prompt that triggers brokered **`fs.read`** on a workspace file.

## Automated vs manual verification

| Area | CI / unit tests | Manual checklist |
|------|-----------------|------------------|
| NDJSON terminal contract | `rex` / NDJSON conformance tests; extension `ndjson_contract_fixture` and `streamClient` tests | — |
| Cancel → idle | `streamClient.test.ts` | Optional smoke in chat |
| Daemon probe **ready → unavailable** | `daemonLifecycle.test.ts` (fixture `cli_status_ok_then_fail.sh`) | Stop daemon while extension open |
| Daemon probe **unavailable → ready** | `daemonLifecycle.test.ts` (fixture `cli_status_fail_then_ok.sh`) | Restart daemon after stop |
| `--approval-id` on `complete` | `cliBridgeArgs.test.ts` | Agent send with `agent.approvals_enabled: true` in JSON |
| Long multi-turn session | — | Checklist below |

## 8) R019 acceptance — live model (operator; not CI)

After `./scripts/verify_mvp_local.sh` passes, validate the **product path** with **`rex-agent`** and **live** OpenAI-compatible HTTP (Ollama, LiteLLM, etc.). CI uses mock/stub harness config; this checklist is the integration acceptance gate for **R019**. Reliable **plan/agent** tool loops on a live model use **R038** native broker tool calling — [NATIVE_TOOL_CALLING.md](NATIVE_TOOL_CALLING.md). Automated live smoke (`ask` + brokered read/policy subset) is planned as **R039** — [ECONOMICS_VALIDATION.md](ECONOMICS_VALIDATION.md).

### 8a) Automated native tool loop (R038; opt-in)

After MVP preflight and with Ollama serving a **tool-capable** model (default `qwen2.5-coder:7b`):

```bash
pip install -e sidecars/rex-agent
rex proto install
./scripts/verify_native_tools_live.sh
```

Requires Ollama at `http://127.0.0.1:11434/v1` with a tool-capable model (default in script: `qwen2.5-coder:7b`). Not run in PR CI (**RC-10**).

**What it automates vs §8 checklist below:**

| Check | `verify_native_tools_live.sh` | Manual §8 |
|-------|------------------------------|-----------|
| Plan-mode read via native `tools[]` / `tool_calls` | Yes — fixture marker in NDJSON chunks; daemon log `protocol=1` (native), no `protocol=3` on plan turn | Optional |
| Agent allowed read + `.env` deny | Yes | Optional (`__rex_read:` prompts) |
| Extension UI, cancel, client hints, multi-turn | No | Yes |
| ask mode live turn | No (**R039**) | Yes |

Fixture workspace: [`fixtures/native_tools_e2e/workspace/`](../fixtures/native_tools_e2e/workspace/).

### 8b) Automated native tool loop on oMLX (planned; opt-in)

**Status:** `planned` — implementation slice after [OMLX_INFERENCE.md](OMLX_INFERENCE.md) PR 5. Mirror §8a for **managed or direct oMLX** at `http://127.0.0.1:8000/v1` (`inference.omlx.mode: managed` or explicit `inference.openai_compat.base_url`) with a tool-capable MLX model. Script name TBD (for example `./scripts/verify_omlx_native_tools_live.sh`). Not run in PR CI (**RC-10**).

Prerequisites for §8 and §8a: HTTP server running (example: `ollama serve`), JSON from step 3 on the **same** daemon process with **direct Ollama** `inference.openai_compat.base_url` `http://127.0.0.1:11434/v1` (gateway opt-in only for multi-provider), workspace folder open in the editor with `rex.daemonAutoStart: true` (or manual `rex daemon` started from that project directory).

**oMLX path (when implemented):** prefer `inference.omlx.mode: managed` or direct `http://127.0.0.1:8000/v1`; see [OMLX_INFERENCE.md](OMLX_INFERENCE.md).

- [ ] Daemon listen log includes `workspace.root=<absolute path>` (not `workspace.error=not_configured`).
- [ ] Extension output shows project `.rex/config.json` merge when auto-start runs; multi-root logs `workspace.warning=multi_root` when applicable.
- [ ] **ask**, **plan**, and **agent** modes each complete a turn against the **live model** (not stub echo-only text).
- [ ] With **Attach editor context** enabled, daemon log shows `client_hints.active_file=...`; prompt does not duplicate large `File:`/`Selection:` blocks (hints on wire).
- [ ] **Cancel** mid-stream twice; composer returns to idle.
- [ ] Brokered **`__rex_read:<workspace-file>`** succeeds under workspace root; **`__rex_read:.env`** is denied.
- [ ] Stop `rex daemon`; status bar shows unavailable until restart.

## Long-session stress (manual)

Use this checklist after the steps above when hardening chat reliability:

- [ ] Send **10+ prompts** in one session without reloading the window.
- [ ] **Cancel** mid-stream at least twice; confirm the composer returns to idle (no stuck “streaming” state).
- [ ] Switch **ask → plan → agent** between turns and send one prompt per mode.
- [ ] Stop `rex daemon` while the extension is open; confirm the status bar shows **unavailable**, then returns to **ready** after restart (also covered by automated probe recovery tests when using fixtures).

## Install troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| `requires-python >=3.10` / pip editable failure | macOS CLT **Python 3.9** or old **pip 21.x** on system Python | `brew install python@3.12`; `./scripts/install-agent-sidecar.sh` |
| PEP 668 externally-managed-environment | Homebrew/system Python blocks global `pip install` | Use operator script (venv at `$REX_ROOT/venv`) — do not `pip install` into system Python |
| `rex-agent` on PATH but import fails | Missing proto gen or wrong Python | `rex proto install`; `rex sidecar doctor` |
| VSIX not compatible with VS Code engine | Cursor/VS Code older than `engines.vscode` (**^1.120.0**) | Upgrade editor; or install a VSIX from an older release tag |
| `reinstall-dev.sh` skipped extension | Earlier failure aborted before extension step (older scripts) | Re-run `./scripts/reinstall-dev.sh` — agent failure no longer blocks extension install |

## Terminal works, editor does not

If `rex status` works in **Terminal.app** or an integrated terminal but the status bar shows **REX unavailable**:

1. Open **Settings**, search for `rex.cliPath`.
2. Set **REX: Cli Path** to the **absolute path** of the binary, for example:
   - `~/.cargo/bin/rex` after `install-cli.sh`, or
   - `/path/to/rex/target/debug/rex` for a debug build.
3. If you use daemon auto-start and spawn fails, set **REX: Daemon Binary Path** to the same **`rex`** binary path (unified CLI).

Then reload the window.

## Related docs

- [EXTENSION_RELEASE.md](./EXTENSION_RELEASE.md) — install, auto-start, troubleshooting table, releases.
- [README.md](../README.md) — project quickstart and `rex` / NDJSON checks.
- [extensions/rex-vscode/README.md](../extensions/rex-vscode/README.md) — extension-specific settings and dev commands.
