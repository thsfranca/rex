# REX for VS Code and Cursor

Local AI runtime client for VS Code and Cursor, backed by the [REX daemon](../..).

This extension is in early development. See [`docs/EXTENSION_ROADMAP.md`](../../docs/EXTENSION_ROADMAP.md) for the phased plan.

## Status

- PR 1: status bar, daemon detection via `rex status`, typed NDJSON stream client, capability detection.
- PR 2: activity-bar chat webview with streaming markdown, syntax-highlighted code blocks, Copy/Insert/Apply actions, native-diff Apply flow, editor context chip, and `REX: Explain/Fix/Refactor Selection` commands.
- PR 3: shipped — opt-in daemon auto-start, tag-driven release pipeline (validated VSIX + optional Open VSX / Marketplace publish), docs in [`docs/EXTENSION_RELEASE.md`](../../docs/EXTENSION_RELEASE.md).

## Using the chat view

1. Run `rex daemon` (user-managed by default) so the status bar reads `REX ready`.
2. Open the `REX` view container from the activity bar, or run `REX: Open Chat`.
3. Type a prompt and send (`Cmd/Ctrl+Enter`); the assistant response streams with live markdown.
4. Per code block you can `Copy`, `Insert at cursor`, or `Apply` (opens a diff and asks for confirmation before writing a `WorkspaceEdit`).
5. Select code in the editor and run `REX: Explain/Fix/Refactor Selection` to prefill the prompt with editor context.

The chat webview uses a strict Content Security Policy (nonce-scoped script, no remote resource loads). Syntax highlighting is lazy-loaded via `shiki` only after the first code block appears.

Stream reliability notes:

- The host enforces exactly one terminal stream transition per request (`done` or `error`).
- Cancellation is deterministic for a stream id (no duplicate terminal states on cancel/resend).
- Error events can include stable codes (for example `daemon_unavailable`, `stream_timeout`, `invalid_response`) to map UI behavior consistently.
- Each request is trace-correlated across extension host, `rex`, and daemon logs.

## Requirements

- `rex` available on `PATH` (or set `rex.cliPath` to an absolute path if the editor was not started from a shell that configures `PATH`, which is common on macOS).
- Daemon running locally on `/tmp/rex.sock` (user-managed: `rex daemon`; or set `rex.daemonAutoStart: true`). With auto-start, set `rex.daemonBinaryPath` to an absolute path if `rex` is not on the editor `PATH` (same binary as `rex.cliPath`).
- VS Code `^1.90` or Cursor with a compatible VS Code engine.

- For **plan/agent** modes with live models, configure JSON per [`docs/EXTENSION_LOCAL_E2E.md`](../../docs/EXTENSION_LOCAL_E2E.md) §3 (`rex-agent`, `inference.openai_compat`). Workspace bind merges the product sidecar into project `.rex/config.json`.

## Daemon auto-start (opt-in)

Flip `"rex.daemonAutoStart": true` in settings to let the extension spawn `rex daemon` automatically when the view activates. The extension:

1. Probes `rex status`; if the daemon is already running, nothing else happens.
2. Otherwise, spawns `rex daemon` via `rex.daemonBinaryPath` and polls status until it is ready (default timeout 10s).
3. Terminates the spawned child on `deactivate()` so the IDE never leaks daemon processes.

If the spawn or probe fails, the status bar moves to `REX unavailable` and the reason is logged to the `REX` output channel.

## Development

Fast loop from the repo root (Rust build + CLI install + VSIX install; does not start the daemon):

```bash
chmod +x ./scripts/dev-rex-extension.sh
./scripts/dev-rex-extension.sh
```

Extension-only install (when Rust binaries are already on your `PATH`):

```bash
chmod +x ./scripts/install-extension.sh
./scripts/install-extension.sh
```

Run checks before packaging when you want CI parity:

```bash
./scripts/install-extension.sh --verify
```

Manual steps:

```bash
cd extensions/rex-vscode
npm install
npm run typecheck
npm run lint
npm test
npm run package
```

`npm run package` produces `rex-vscode.vsix`.

## Design Mode preview

Preview the chat webview in Cursor’s integrated browser (mock host APIs + VS Code theme tokens). See [`dev/README.md`](./dev/README.md) for details.

```bash
cd extensions/rex-vscode
npm run dev:webview
```

1. Open **Agents Window** (`Cmd+Shift+P` → “Agents Window”).
2. Go to **http://127.0.0.1:3456/**
3. Run **Design Mode** from the command palette (`Cmd+Shift+P` → “Design Mode”).
4. Edit `webview/components/*.tsx` and `webview/theme/themeVars.css`; keep `npm run watch:webview` running and reload the extension window to verify in the real webview.

## Settings

| Key | Default | Purpose |
|---|---|---|
| `rex.cliPath` | `rex` | Resolvable path or name for `rex` (`status` / `complete`). |
| `rex.daemonBinaryPath` | `rex` | Same binary; spawned with `daemon` subcommand when auto-start is on. |
| `rex.daemonAutoStart` | `false` | Opt-in extension-managed daemon lifecycle. |
| `rex.modelId` | *(empty)* | When set, passed as `--model` on every `rex complete`. |

## License

See [LICENSE](./LICENSE).
