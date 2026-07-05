# Web UI agent validation

Agent playbook for **rex-ui-harness** MCP acceptance. Canonical design bars: [WEB_UI_DESIGN.md](WEB_UI_DESIGN.md).

## Setup

```bash
./scripts/setup_ui_probe_env.sh
```

Register MCP: `node crates/rex-ui-harness/dist/index.js` with cwd = repo root.

## Default: real desktop + daemon (macOS)

`ui_open` with no arguments launches **rex-desktop** with:

- `REX_ROOT=fixtures/ui_probe/rex_root` (mock inference, `sidecars.harness=direct`)
- **`apps/rex-web/dist`** served via `vite preview` (same bundle as Tauri release builds)
- Real Tauri IPC → UDS `StreamInference` → streaming transcript

Use `ui_open { "mode": "static" }` for the HTML fixture only (CI / layout tokens without daemon).

## CI verify

Path-aware **`ui-verify`** in [`.github/workflows/ci.yml`](../.github/workflows/ci.yml) runs when Web UI paths change:

| Matrix leg | Runner | Command |
|------------|--------|---------|
| static | `ubuntu-latest` | `./scripts/ci/run_ui_verify.sh --mode static` |
| desktop | `macos-latest` | `./scripts/ci/run_ui_verify.sh --mode desktop` |

Failures surface in merge gate **`ci-checks`** as `UI_FAIL`. Local reproduction uses the same script. See [CI.md](CI.md#ui-verify-path-aware).

## Minimum scenario bundle (desktop)

1. `ui_open`
2. `ui_wait_for` — selector `[data-testid=shell]`
3. `ui_assert_token` — `#status-dot`, `--rex-status-success`, `background-color`
4. `ui_send_keys` — selector `[data-testid=composer-input]`, keys `hello{Enter}` (or fill + Enter via separate tools)
5. `ui_wait_for` — text in assistant response (mock stream)
6. `ui_close`

## Static fixture bundle

1. `ui_open` with `{ "mode": "static" }`
2. `ui_goto_scenario` → `idle`
3. Token + layout asserts
4. `ui_goto_scenario` → `streaming` (fixture motion)
5. `ui_close`

## Related

- [fixtures/ui_probe/README.md](../fixtures/ui_probe/README.md)
- [WEB_UI_ARCHITECTURE.md](WEB_UI_ARCHITECTURE.md)
