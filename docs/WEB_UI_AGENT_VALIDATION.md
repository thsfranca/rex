# Web UI agent validation

Agent playbook for **rex-ui-harness** MCP acceptance. Canonical design bars: [WEB_UI_DESIGN.md](WEB_UI_DESIGN.md).

## Setup

```bash
./scripts/setup_ui_probe_env.sh
```

Register MCP: `node crates/rex-ui-harness/dist/index.js` with cwd = repo root.

## Default: real desktop + daemon (macOS)

`ui_open` launches **Electron** `apps/rex-desktop` with production `apps/rex-web/dist` and the mock daemon under `REX_ROOT=fixtures/ui_probe/rex_root` (`sidecars.harness=direct`). Electron main owns UDS gRPC; the renderer never opens the socket.

Desktop CI gate: `./scripts/ci/run_ui_verify.sh --mode desktop` runs compositor proof (plus the separate `electron-compositor-proof` job). Full scenario suite (`node crates/rex-ui-harness/dist/run-ci.js --mode desktop`) is available locally for operator polish.

Use `ui_open { "mode": "build" }` for build-only validation (no desktop session).

## CI verify

Path-aware **`ui-verify`** in [`.github/workflows/ci.yml`](../.github/workflows/ci.yml) runs when Web UI paths change:

| Matrix leg | Runner | Command |
|------------|--------|---------|
| build | `ubuntu-latest` | `./scripts/ci/run_ui_verify.sh --mode build` |
| desktop | `macos-latest` | `./scripts/ci/run_ui_verify.sh --mode desktop` |

Failures surface in merge gate **`ci-checks`** as `UI_FAIL`. Local reproduction uses the same script. See [CI.md](CI.md#ui-verify-path-aware).

## Minimum scenario bundle (desktop)

1. `ui_open`
2. `ui_wait_for` — selector `[data-testid=shell]`
3. `ui_assert_token` — `#status-dot`, `--rex-status-success`, `background-color`
4. `ui_send_keys` — selector `[data-testid=composer-input]`, keys `hello{Enter}` (or fill + Enter via separate tools)
5. `ui_wait_for` — text `mock: hello` in assistant response (mock stream)
6. `ui_wait_for` — status label `Ready` (`#status-label`, not footer copy)
7. Narrow viewport layout assert + `⌘K` command palette smoke
8. `ui_goto_scenario` → `approval_required` — real mock tool approval via `__approval_probe__`
9. `ui_close`

## Component tests (build leg)

`apps/rex-web` runs Vitest + Testing Library via `npm test` in the `ui-verify` build matrix leg (primitives: Button, Modal, SegmentedControl).

## Observability (opt-in debug)

The in-app observability panel is **hidden unless Rex is started with `--debug`**:

```bash
rex --debug
```

The harness launches Electron via Playwright `_electron` (`ui_open`). Read `[data-testid=ui-observability]` or `window.__REX_UI_OBSERVABILITY__` for phase, stream events, and submit errors (panel visible when the app is started with `--debug`).

## Related

- [fixtures/ui_probe/README.md](../fixtures/ui_probe/README.md)
- [WEB_UI_ARCHITECTURE.md](WEB_UI_ARCHITECTURE.md)
