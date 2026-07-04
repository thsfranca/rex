# Rex web UI probe fixture

Validation harness for **rex-ui-harness** MCP.

## Modes

| Mode | When | What runs |
|------|------|-----------|
| **desktop** (default on macOS) | `ui_open` | Real `rex-desktop` (Tauri) + `REX_ROOT` probe config + mock daemon inference |
| **static** | Linux CI, token/layout-only | Headless Chromium on `static/index.html` mock scenarios |

Configure in `rex-ui-harness.toml`:

```toml
[launch]
mode = "desktop"   # or "static"
```

## Layout

```
fixtures/ui_probe/
  rex_root/config.json   # mock inference, sidecars.harness=direct
  workspace/             # cwd for daemon (global socket)
  static/                # HTML fixture (static mode only)
```

## Setup

```bash
./scripts/setup_ui_probe_env.sh
```

Builds `rex-web`, `rex-ui-harness`, and on macOS `rex-desktop --features e2e-testing`.

Register MCP in Cursor:

| Field | Value |
|-------|--------|
| command | `node` |
| args | `["<repo>/crates/rex-ui-harness/dist/index.js"]` |
| cwd | repo root |

## Desktop acceptance (real UI + daemon)

1. `ui_open` — spawns Tauri app, waits for `Ready`
2. `ui_assert_layout` — `[data-testid=shell]` display `grid`
3. `ui_send_keys` — composer: `"hello"` + Enter
4. `ui_wait_for` — text `"hello"` in transcript; status `Working` then `Ready`
5. `ui_assert_token` — `#status-dot` vs `--rex-status-success`
6. `ui_close`

## Static scenarios (`ui_goto_scenario`)

| Scenario | Validates |
|----------|-----------|
| `idle` | Default Ready state |
| `streaming` | Working status + motion hooks |
| `approval_required` | Modal (fixture only) |

Motion/clock tools require **static** mode.

## Baselines

PNG baselines live in `.rex-ui-harness/baselines/` (gitignored except `.gitkeep`).
