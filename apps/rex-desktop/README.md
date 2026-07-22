# rex-desktop

Electron shell for Rex ([ADR 0043](../../docs/architecture/decisions/0043-electron-shell-for-electric-alive-compositor.md)).

Loads the **sole** product UI from `apps/rex-web`. There is no parallel operator UI and no `proof/` mini-app.

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

Runs against **apps/rex-web** (header + composer + AmbientCanvas) for ≥5s. Intentional bury injects CSS in the test runner only.

```bash
cd apps/rex-web && npm run build
cd ../rex-desktop
npm run compositor-proof
npm run compositor-proof:bury-expect-fail
```

From repo root:

```bash
./scripts/ci/run_electron_compositor_proof.sh
```

## Daemon bridge

Electron **main** owns UDS gRPC. The renderer uses `window.rexDesktop` only.

```bash
export REX_ROOT="$PWD/fixtures/ui_probe/rex_root"
cargo build -p rex
cd apps/rex-web && npm run build
cd ../rex-desktop && npm start
```
