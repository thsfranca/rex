# REX for VS Code and Cursor

Local AI runtime client for VS Code and Cursor, backed by the [REX daemon](../..).

This extension is in early development. See [`docs/EXTENSION_ROADMAP.md`](../../docs/EXTENSION_ROADMAP.md) for the phased plan.

## Status

- PR 1: status bar, daemon detection via `rex-cli status`, typed NDJSON stream client, capability detection.
- PR 2: activity-bar chat webview with streaming markdown, syntax-highlighted code blocks, Copy/Insert/Apply actions, native-diff Apply flow, editor context chip, and `REX: Explain/Fix/Refactor Selection` commands.
- PR 3: shipped — opt-in daemon auto-start, tag-driven release pipeline (validated VSIX + optional Open VSX / Marketplace publish), docs in [`docs/EXTENSION_RELEASE.md`](../../docs/EXTENSION_RELEASE.md).

## Using the chat view

1. Start `rex-daemon` (user-managed by default) so the status bar reads `REX ready`.
2. Open the `REX` view container from the activity bar, or run `REX: Open Chat`.
3. Type a prompt and send (`Cmd/Ctrl+Enter`); the assistant response streams with live markdown.
4. Per code block you can `Copy`, `Insert at cursor`, or `Apply` (opens a diff and asks for confirmation before writing a `WorkspaceEdit`).
5. Select code in the editor and run `REX: Explain/Fix/Refactor Selection` to prefill the prompt with editor context.

The chat webview uses a strict Content Security Policy (nonce-scoped script, no remote resource loads). Syntax highlighting is lazy-loaded via `shiki` only after the first code block appears.

Stream reliability notes:

- The host enforces exactly one terminal stream transition per request (`done` or `error`).
- Cancellation is deterministic for a stream id (no duplicate terminal states on cancel/resend).
- Error events can include stable codes (for example `daemon_unavailable`, `stream_timeout`, `invalid_response`) to map UI behavior consistently.
- Each request is trace-correlated across extension host, `rex-cli`, and daemon logs.

## Requirements

- `rex-cli` available on `PATH` (or set `rex.cliPath` to an absolute path if the editor was not started from a shell that configures `PATH`, which is common on macOS).
- `rex-daemon` running locally on `/tmp/rex.sock` (user-managed by default, or set `rex.daemonAutoStart: true`). With auto-start, set `rex.daemonBinaryPath` to an absolute path if `rex-daemon` is not on the editor `PATH`.
- VS Code `^1.90` or Cursor with a compatible VS Code engine.

**End-to-end from this repository:** follow [`docs/EXTENSION_LOCAL_E2E.md`](../../docs/EXTENSION_LOCAL_E2E.md) (build, install binaries, daemon, VSIX, verification). Quick combined script from repo root: `./scripts/dev-rex-extension.sh` (then start the daemon or enable auto-start as described in that doc).

## Daemon auto-start (opt-in)

Flip `"rex.daemonAutoStart": true` in settings to let the extension spawn `rex-daemon` automatically when the view activates. The extension:

1. Probes `rex-cli status`; if the daemon is already running, nothing else happens.
2. Otherwise, spawns the binary at `rex.daemonBinaryPath` and polls status until it is ready (default timeout 10s).
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

## Settings

| Key | Default | Purpose |
|---|---|---|
| `rex.cliPath` | `rex-cli` | Resolvable path or name for `rex-cli`. |
| `rex.daemonBinaryPath` | `rex-daemon` | Resolvable path or name for `rex-daemon`. |
| `rex.daemonAutoStart` | `false` | Opt-in extension-managed daemon lifecycle. |

## License

See [LICENSE](./LICENSE).
