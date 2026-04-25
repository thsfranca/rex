# REX Extension: Install, Operate, Release

This document covers the end-to-end lifecycle for the `extensions/rex-vscode` package: how a user installs it, how the optional daemon auto-start behaves, how to troubleshoot common failure modes, and how the release pipeline turns a git tag into a published VSIX.

## Audience

- Users installing the extension in VS Code or Cursor.
- Maintainers preparing a release.

## Install

Prerequisites:

- VS Code `>= 1.90` (or a recent Cursor build with a compatible VS Code engine).
- `rex-cli` on `PATH` (or set `rex.cliPath`).
- `rex-daemon` reachable via `/tmp/rex.sock` or available as a binary if you want to enable auto-start.

### Option 1 — From a VSIX artifact (current path)

1. Download the VSIX attached to the GitHub Release (`rex-vscode-vX.Y.Z.vsix`).
2. In VS Code or Cursor, run `Extensions: Install from VSIX...` and choose the downloaded file.
3. Reload the editor if prompted.

### Developer install from this repository

From the repo root (requires Node.js 20+ and the **Cursor** or **VS Code** shell CLI on `PATH`, or set `REX_EXTENSION_EDITOR` to the full path of the CLI binary):

```bash
chmod +x ./scripts/install-extension.sh
./scripts/install-extension.sh
```

The script runs `npm install` and `npm run package` under `extensions/rex-vscode`, installs `rex-vscode.vsix` with `--force`, then runs `workbench.action.reloadWindow` on the last active editor window when the CLI supports it. When both `cursor` and `code` exist, the script prefers the host indicated by `TERM_PROGRAM` if you run it from an integrated terminal.

Useful flags:

- `--verify` — run lint, typecheck, and tests before packaging.
- `--editor cursor` or `--editor vscode` — pin the CLI when auto-detection is wrong.
- `--no-reload` — install only; reload the window yourself.
- `--only-install` — reuse an existing `extensions/rex-vscode/rex-vscode.vsix` without rebuilding.

### Option 2 — From Open VSX (once published)

- Cursor: search `REX` in the extensions view (Open VSX is the default marketplace).
- VS Code: point the editor at Open VSX, or install from a VSIX as above until the VS Code Marketplace listing is live.

### Verify the install

- The activity bar shows a new `REX` container.
- The status bar shows `REX ready`, `REX starting`, or `REX unavailable` within ~2 seconds of activation.
- The `REX: Open Chat` command focuses the chat view.

## First-run checklist

| Check | Expected |
|---|---|
| `rex-cli --version` resolves | Works from a terminal outside the editor. |
| Status bar | Transitions from `starting` to `ready` without intervention. |
| Output channel `REX` | Records activation events, daemon probe results, and auto-start decisions. |
| Send a test prompt | Streams markdown into the chat view without flicker. |

## Daemon lifecycle modes

| Mode | Setting | Behavior |
|---|---|---|
| User-managed (default) | `rex.daemonAutoStart: false` | Extension probes only; user starts / stops `rex-daemon` manually. |
| Extension-managed | `rex.daemonAutoStart: true` | Extension probes first, spawns `rex-daemon` if needed, polls status until ready, tears the child down on deactivate. |

Notes:

- Auto-start uses `rex.daemonBinaryPath` (default `rex-daemon`) to locate the binary.
- The readiness poll is time-bounded (10s by default); failures surface as `REX unavailable` with a reason in the output channel.
- Only the lifecycle instance that spawned the daemon will try to stop it; a pre-existing daemon is left alone.

## Settings

| Key | Default | Purpose |
|---|---|---|
| `rex.cliPath` | `rex-cli` | Executable path or name for `rex-cli`. |
| `rex.daemonBinaryPath` | `rex-daemon` | Executable path or name for `rex-daemon`. Used only when auto-start is enabled. |
| `rex.daemonAutoStart` | `false` | Opt-in extension-managed daemon lifecycle. |

## Troubleshooting

Open the `REX` output channel (`REX: Open Output Channel`) for details on every scenario below.

| Symptom | Likely cause | Fix |
|---|---|---|
| `REX unavailable` at activation | `rex-cli` not on `PATH`. | Set `rex.cliPath` or install `rex-cli`. |
| `REX unavailable: daemon did not become ready within Nms` | Daemon took too long to bind its socket. | Increase daemon warm-up time by starting it manually, or leave auto-start off. |
| `REX unavailable: rex-daemon exited with code N` | Daemon crashed on startup. | Inspect daemon logs; ensure only one instance runs on `/tmp/rex.sock`. |
| `Apply to file` opens diff but shows empty proposal | The proposed code block was empty or the target selection was cleared. | Re-run the action with a non-empty selection or accept a full-file replacement. |
| Chat view is blank / stuck on loading | Webview failed to load the bundled script. | Reload the window; if it persists, capture the webview devtools console and file an issue. |
| Streaming output flickers or stalls | Underlying `rex-cli complete` stream was interrupted. | Cancel and retry; confirm `rex-daemon` is healthy via `REX: Show Daemon Status`. |

## Release pipeline

The extension is released independently of the REX daemon. Tags prefixed with `rex-vscode-v` drive the workflow at `.github/workflows/extension-release.yml`.

### Release checklist

1. Decide the next semantic version `X.Y.Z` and update `extensions/rex-vscode/package.json` plus `extensions/rex-vscode/CHANGELOG.md` in a PR.
2. After the PR merges, tag the release commit: `git tag rex-vscode-vX.Y.Z && git push origin rex-vscode-vX.Y.Z`.
3. The workflow runs:
   - Lint, typecheck, and unit tests.
   - `vsce package --no-dependencies --out rex-vscode-<version>.vsix`.
   - `ovsx publish --dry-run` (always runs; validates the VSIX against Open VSX rules without publishing).
   - If `OVSX_TOKEN` secret is set, `ovsx publish --pat $OVSX_TOKEN`.
   - If `VSCE_PAT` secret is set, `vsce publish --packagePath <vsix> --pat $VSCE_PAT` (VS Code Marketplace).
   - Creates a GitHub Release with the VSIX attached when the trigger is a tag push.

### Manual dry-run

- `gh workflow run "Extension Release" -f dry_run=true` builds and packages the VSIX without attempting to publish or create a release; useful for verifying the pipeline on a non-release branch.

### Secrets

| Secret | Purpose | Required for |
|---|---|---|
| `OVSX_TOKEN` | Personal access token for Open VSX. | Publishing to Open VSX (Cursor distribution). |
| `VSCE_PAT` | Personal access token for the VS Code Marketplace. | Optional Marketplace publish. |

Without either secret the pipeline still builds, tests, packages, and dry-runs — safe for repeated use.

### Platform packaging

- Packaging target: universal (no native modules in the extension host or webview bundles).
- If a future change adds a platform-specific binary, revisit this section and adopt `vsce package --target` flows.

## Changelog discipline

- Every user-visible change adds an entry under `## [Unreleased]` in `extensions/rex-vscode/CHANGELOG.md`.
- On release, rename the `Unreleased` heading to the new version and the date, then start a fresh `Unreleased` block in the next PR.
