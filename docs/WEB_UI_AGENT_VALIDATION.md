# Web UI agent validation

Agent playbook for **rex-ui-harness** MCP acceptance. Canonical design bars: [WEB_UI_DESIGN.md](WEB_UI_DESIGN.md).

## Setup

```bash
./scripts/setup_ui_probe_env.sh
```

Register MCP: `node crates/rex-ui-harness/dist/index.js` with cwd = repo root.

## Minimum scenario bundle

1. `ui_open`
2. `ui_goto_scenario` → `idle`
3. `ui_assert_token` — `#status-dot`, `--rex-status-success`, `background-color`
4. `ui_assert_layout` — `[data-testid=shell]`, display `grid`
5. `ui_goto_scenario` → `streaming`
6. `ui_assert_motion` or `ui_assert_canvas`
7. `ui_close`

## Native WKWebView (W109)

When `tauri-plugin-playwright` is enabled, pass `{ "native": true }` to `ui_open` to target the desktop shell instead of the static fixture.

## Related

- [fixtures/ui_probe/README.md](../fixtures/ui_probe/README.md)
- [WEB_UI_ARCHITECTURE.md](WEB_UI_ARCHITECTURE.md)
