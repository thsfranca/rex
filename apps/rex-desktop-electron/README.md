# rex-desktop-electron

Electron shell scaffold for Rex (ADR 0043 / W126).

This slice ships a **compositor proof** page: dark shell chrome (header + composer) over a fullscreen WebGL clear. Chromium must keep chrome painted and hit-testable for ≥5 seconds while WebGL runs.

## Prerequisites

- macOS Apple Silicon (v1 product host)
- Node ≥ 20

## Setup

```bash
cd apps/rex-desktop-electron
npm install
```

## Run proof window

```bash
npm run start:proof
```

Intentional bury (WebGL above chrome — must fail the automated proof):

```bash
npm run start:bury
```

## Automated compositor proof

Samples at t=0, 1s, 3s, 5s. Each sample:

- Hit-test: `elementFromPoint` on `#proof-header` and `#proof-composer`
- Luminance: chrome screenshot must not be WebGL-clear-only

```bash
npm run compositor-proof
npm run compositor-proof:bury-expect-fail
```

From repo root:

```bash
./scripts/ci/run_electron_compositor_proof.sh
```

## Next slices

- W127: daemon UDS in main + load `apps/rex-web`
- W128: remount Electric Alive effects
- W129: harness desktop on Electron; retire Tauri product path
