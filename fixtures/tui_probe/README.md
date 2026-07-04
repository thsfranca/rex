# TUI probe fixture

Isolated **`REX_ROOT`** and workspace for **tuiwright** design acceptance (load the **`tui-probe`** agent skill).

## Layout

| Path | Role |
|------|------|
| `rex_root/config.json` | Mock inference, `harness: direct`, per-workspace sockets — no live sidecar or LLM required for S1–S6 |
| `rex_root/sockets/` | Runtime UDS paths (gitignored contents) |
| `workspace/` | Probe cwd — workspace root is this directory when `rex` runs |

The launch wrapper in `tuiwright.toml.example` `cd`s into `workspace/` and sets `REX_ROOT` to `../rex_root` so the repo-root `.rex/config.json` does not merge into probes.

## MCP setup

### Agent permissions (required for keystrokes)

Cursor **Auto-review** blocks `tui_send_keys` unless tuiwright is on the MCP allowlist. Copy the example permissions file so agents can run the full scenario matrix without manual approval:

```bash
./scripts/setup_tui_probe_env.sh
```

Or manually: `cursor-permissions.tui-probe.json.example` → `.cursor/permissions.json`.

Requires **Run Mode** enabled in Cursor Settings → Agents → Approvals & Execution (Auto-review or Allowlist). Cursor reloads `permissions.json` on change.

### Cursor `mcp.json` (user-local)

```json
"tuiwright": {
  "command": "/path/to/tuiwright",
  "args": ["--config", "/path/to/rex/tuiwright.toml"],
  "cwd": "/path/to/rex"
}
```

Copy `tuiwright.toml.example` → repo-root `tuiwright.toml` and set `launch.command` / wrapper if your debug binary path differs. **Restart tuiwright MCP** after edits.

## Baselines

Layout snapshots live in `.tuiwright/baselines/` (repo root). Names:

| Baseline | Scenario |
|----------|----------|
| `idle-wide` | S1 idle 120×40 |
| `help-expanded` | S2 after `?` |
| `breakpoint-90` | S3 standard width |
| `breakpoint-70` | S3 narrow (timeline unmounted) |
| `micro-55` | S3 micro overlay |

First acceptance run: `tui_diff` with `create_if_missing: true` per baseline. Refresh intentionally when layout changes — paths and version strings in baselines may drift; prefer stable region checks via `tui_assert` for leaks.

## Recordings

Motion review (S6): `tui_record_start` → interact → `tui_record_stop` → optional `tui_to_gif` (requires `agg`). Recordings go under `.tuiwright/recordings/` (gitignored).

## Monochrome pass (optional S8)

Uncomment `NO_COLOR = "1"` in `tuiwright.toml` `[launch.env]` for one full matrix run.
