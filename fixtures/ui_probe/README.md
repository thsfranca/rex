# Rex web UI probe fixture

Isolated static HTML harness for **rex-ui-harness** MCP validation before the Tauri app lands.

## Layout

```
fixtures/ui_probe/
  static/
    index.html    # Wide-profile shell wireframe + scenario hooks
    tokens.css    # --rex-* semantic tokens
```

## Setup

```bash
./scripts/setup_ui_probe_env.sh
```

Register MCP in Cursor:

| Field | Value |
|-------|--------|
| command | `node` |
| args | `["<repo>/crates/rex-ui-harness/dist/index.js"]` |
| cwd | repo root |

Copy `rex-ui-harness.toml.example` → `rex-ui-harness.toml` (script does this).

## Scenarios (`ui_goto_scenario`)

| Scenario | Validates |
|----------|-----------|
| `idle` | Default Ready state, timeline visible |
| `streaming` | Working status dot + ambient motion |
| `approval_required` | Modal backdrop + spring target |
| `error` | Semantic error color |

## Agent acceptance minimum

1. `ui_open`
2. `ui_goto_scenario` → `idle`
3. `ui_assert_token` on `#status-dot` vs `--rex-status-success`
4. `ui_assert_layout` on `[data-testid=shell]` display grid
5. `ui_close`

## Baselines

PNG baselines live in `.rex-ui-harness/baselines/` (gitignored except `.gitkeep`).
