# REX for VS Code and Cursor

Local AI runtime client for VS Code and Cursor, backed by the [REX daemon](../..).

This extension is in early development. PR 1 ships foundations (status bar, daemon detection, headless runtime modules). Chat UI arrives in PR 2. See [`docs/EXTENSION_ROADMAP.md`](../../docs/EXTENSION_ROADMAP.md).

## Status

- PR 1 (current): status bar, daemon detection via `rex-cli status`, typed NDJSON stream client, capability detection. No chat UI yet.
- PR 2: streaming chat webview, syntax-highlighted code blocks, Apply-to-file via native diff.
- PR 3: opt-in daemon auto-start, Open VSX publish pipeline, docs polish.

## Requirements

- `rex-cli` available on `PATH` (or set `rex.cliPath`).
- `rex-daemon` running locally on `/tmp/rex.sock` (user-managed by default).
- VS Code `^1.90` or Cursor with a compatible VS Code engine.

## Development

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
