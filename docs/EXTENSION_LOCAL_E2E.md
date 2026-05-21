# Local end-to-end: REX daemon, CLI, and VS Code / Cursor extension

This guide walks from a clean clone to **REX ready** in the editor: built binaries, `rex-cli` visible to the extension host, `rex-daemon` on `/tmp/rex.sock`, and the extension installed.

## Prerequisites

- Rust toolchain (`cargo`, `rustc`). See [DEPENDENCIES.md](./DEPENDENCIES.md) for `protoc` (required to build `rex-proto`).
- Node.js **20+** and `npm` (for the extension build).
- **Cursor** or **VS Code** with the shell CLI on `PATH`, or set `REX_EXTENSION_EDITOR` to the full path of the `cursor` or `code` binary when using [scripts/install-extension.sh](../scripts/install-extension.sh).

**Preflight (automated):** From the repo root, run `chmod +x ./scripts/verify_mvp_local.sh && ./scripts/verify_mvp_local.sh`. That command builds the workspace, runs the same Rust gate as CI (`fmt`, `clippy`, tests), and runs the extension gate (`npm ci`, typecheck, lint, build, tests, VSIX package). It does **not** start the daemon. Use it before the manual editor checks in step 6 below.

## 1) Build the Rust workspace

From the repository root:

```bash
cargo build --workspace
```

This builds `rex-proto`, `rex-daemon`, and `rex-cli`. For a quicker smoke test you can use `cargo run -p rex-daemon` / `cargo run -p rex-cli` without installing binaries (see step 5 for editor settings if you stay on `cargo run` for the CLI only).

## 2) Put `rex-cli` and `rex-daemon` where the editor can run them

The extension spawns **`rex-cli` as a child process** using the extension host environment. That environment often has a **smaller `PATH` than your terminal** (common when you launch Cursor or VS Code from the macOS Dock).

Recommended: install both binaries into Cargo’s bin directory:

```bash
chmod +x ./scripts/install-cli.sh
./scripts/install-cli.sh
```

To print the standard install locations for **REX: Cli Path** and **REX: Daemon Binary Path** without re-installing, run `./scripts/install-cli.sh --print-bin-path`.

Optional: add `~/.cargo/bin` to your shell and restart the editor so GUI-launched apps inherit it:

```bash
./scripts/install-cli.sh --configure-shell
source ~/.zshrc
```

Then **fully quit and reopen** Cursor or VS Code (not only Reload Window).

## 3) Configure brokered HTTP and sidecar (MVP)

Phase 1 expects a **daemon-supervised sidecar agent** plus **brokered OpenAI-compatible HTTP** ([MVP_SPEC.md](./MVP_SPEC.md), [SIDECAR_RUNTIME.md](./SIDECAR_RUNTIME.md), [CONFIGURATION.md](./CONFIGURATION.md)). Example with **Ollama**:

```bash
export REX_OPENAI_COMPAT_BASE_URL="http://127.0.0.1:11434/v1"
export REX_OPENAI_COMPAT_MODEL="llama3.2"
export REX_INFERENCE_RUNTIME="http-openai-compat"
```

For automated preflight only (no live LLM), CI uses `REX_INFERENCE_RUNTIME=mock` and/or a stub sidecar — not the product MVP path.

**Sidecar (product path):** enable supervision and install the stub binary:

```bash
export REX_SIDECAR_ENABLED=1
export REX_SIDECAR_BINARY=rex-sidecar-stub   # or absolute path after cargo install
cargo build -p rex-sidecar-stub
# ensure target/debug or install path is on PATH when starting rex-daemon
```

See [CONFIGURATION.md](./CONFIGURATION.md) for `REX_SIDECAR_REQUIRED`, socket path, and harness `REX_SIDECAR_HARNESS=direct` (CI/tests only).

## 4) Run `rex-daemon`

**User-managed (default extension behavior)**

In a separate terminal from the repo root (with HTTP env from step 3):

```bash
cargo run -p rex-daemon
```

Or, after `install-cli.sh`:

```bash
rex-daemon
```

The daemon listens on **`/tmp/rex.sock`**.

**Extension-managed (opt-in)**

In editor settings, set `"rex.daemonAutoStart": true`. The extension spawns `rex.daemonBinaryPath` (default `rex-daemon`). If `rex-daemon` is not on the editor `PATH`, set `rex.daemonBinaryPath` to an **absolute path** (same idea as `rex.cliPath` below).

## 5) Verify from a terminal

```bash
rex-cli status
```

Or without install:

```bash
cargo run -p rex-cli -- status
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

**One-shot helper** (build Rust, install CLI into `~/.cargo/bin`, then install the VSIX; does **not** start the daemon):

```bash
chmod +x ./scripts/dev-rex-extension.sh
./scripts/dev-rex-extension.sh
```

Pass through extra flags to `install-extension.sh`, for example:

```bash
./scripts/dev-rex-extension.sh --verify
```

## 7) Verify in the editor

- Status bar shows **REX ready** (or **REX starting** briefly).
- Command **REX: Show Daemon Status** returns a snapshot.
- Output channel **REX** logs activation and probe/auto-start decisions.

Open **REX: Open Chat**, select **agent** or **plan** mode, send a short prompt, and confirm streaming completes **via the sidecar path** (daemon logs should show sidecar turn + broker). Exercise **cancel** and **Apply** on a code block (approval in non-ask modes). Verify a prompt that triggers brokered **`fs.read`** on a workspace file.

## Long-session stress (manual)

Use this checklist after the steps above when hardening chat reliability:

- [ ] Send **10+ prompts** in one session without reloading the window.
- [ ] **Cancel** mid-stream at least twice; confirm the composer returns to idle (no stuck “streaming” state).
- [ ] Switch **ask → plan → agent** between turns and send one prompt per mode.
- [ ] Stop `rex-daemon` while the extension is open; confirm the status bar shows **unavailable**, then returns to **ready** after restart.

## Terminal works, editor does not

If `rex-cli status` works in **Terminal.app** or an integrated terminal but the status bar shows **REX unavailable**:

1. Open **Settings**, search for `rex.cliPath`.
2. Set **REX: Cli Path** to the **absolute path** of the binary, for example:
   - `~/.cargo/bin/rex-cli` after `install-cli.sh`, or
   - `/path/to/rex/target/debug/rex-cli` for a debug build.
3. If you use daemon auto-start and spawn fails, set **REX: Daemon Binary Path** to the absolute `rex-daemon` path the same way.

Then reload the window.

## Related docs

- [EXTENSION_RELEASE.md](./EXTENSION_RELEASE.md) — install, auto-start, troubleshooting table, releases.
- [README.md](../README.md) — project quickstart and `rex-cli` / NDJSON checks.
- [extensions/rex-vscode/README.md](../extensions/rex-vscode/README.md) — extension-specific settings and dev commands.
