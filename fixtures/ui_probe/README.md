# Rex web UI probe fixture

Validation harness for **rex-ui-harness** MCP.

## Modes

| Mode | When | What runs |
|------|------|-----------|
| **desktop** (default on macOS) | compositor proof / future `ui_open` | Electron `apps/rex-desktop` + `apps/rex-web/dist` + mock daemon (full `ui_open` after W127/W129) |
| **build** | Linux CI | `npm ci && npm run build` in `apps/rex-web` (same bundle artifact) |

Configure in `rex-ui-harness.toml`:

```toml
[launch]
mode = "desktop"   # or "build"
```

## Layout

```
fixtures/ui_probe/
  rex_root/config.json   # mock inference, sidecars.harness=direct
  workspace/             # cwd for daemon (global socket)
```

## Setup

```bash
./scripts/setup_ui_probe_env.sh
```

Builds `rex-web`, `rex-ui-harness`, and installs Electron `apps/rex-desktop` (macOS also builds `rex` CLI).

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
4. `ui_wait_for` — text `mock: hello` in transcript; `#status-label` `Ready`
5. `ui_assert_layout` — hairline flux, transcript hairline, edge glow, status orbit visible during streaming
6. `ui_assert_token` — `#status-dot` vs `--rex-status-success`
7. `ui_goto_scenario` → `approval_required` — modal via `__approval_probe__` mock stream
8. `ui_close`

## Desktop scenarios (`ui_goto_scenario`)

| Scenario | Validates |
|----------|-----------|
| `idle` | Default Ready state |
| `streaming` | Working status + motion tier on `#status-dot`; ambient/particle canvas tier `cinematic`; regl particle renderer; canvas pixel drift |
| `approval_required` | Approval modal on production UI (mock daemon probe) |

## Observability

Harness desktop sessions launch with `--debug`. Inspect `[data-testid=ui-observability]` or `window.__REX_UI_OBSERVABILITY__` for phase, stream events, and submit errors when a scenario fails.

## Baselines

PNG baselines live in `.rex-ui-harness/baselines/` (gitignored except `.gitkeep`).
