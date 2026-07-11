# rex-desktop

Electron shell for Rex ([ADR 0043](../../docs/architecture/decisions/0043-electron-shell-for-electric-alive-compositor.md)).

Loads the single product UI from `apps/rex-web`. There is no second operator UI.

## Prerequisites

- macOS Apple Silicon (v1 product host)
- Node ≥ 20
- Built web UI: `cd apps/rex-web && npm run build`

## Setup

```bash
cd apps/rex-desktop
npm install
```

## Run (product UI)

```bash
npm start
```

Or from the repo: bare `rex` on macOS spawns this app.

## Compositor proof (CI / local gate)

CI-only proof page under `proof/` — not a product surface. Samples chrome + fullscreen WebGL co-visibility for ≥5s.

```bash
npm run compositor-proof
npm run compositor-proof:bury-expect-fail
```

From repo root:

```bash
./scripts/ci/run_electron_compositor_proof.sh
```

## Daemon bridge

Electron **main** owns UDS gRPC. The renderer uses `window.rexDesktop` only.

Autostart uses `rex __rex_internal_daemon` with `cwd` set to the OS temp dir so a project overlay under `$HOME/.rex` does not override `$REX_ROOT/config.json` when the app is launched from a path under the home directory.

```bash
export REX_ROOT="$PWD/fixtures/ui_probe/rex_root"
cargo build -p rex
cd apps/rex-web && npm run build
cd ../rex-desktop && npm start
```
