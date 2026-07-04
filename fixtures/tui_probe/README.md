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

## Stepped motion (deterministic harness)

When `rex` runs in the **tuiwright probe fixture** (launch wrapper sets `REX_ROOT` to `fixtures/tui_probe/rex_root` and cwd to `fixtures/tui_probe/workspace`), the compositor uses a **stepped animation clock**. No extra env vars — only `REX_ROOT` and process cwd.

| Action | Effect |
|--------|--------|
| **F12** in probe TUI | +16 ms animation time per press |
| Stable session id | Fixed `hs-probe` while in fixture context (also masked by `ignore_patterns`) |

Example acceptance sequence:

```
tui_open
tui_send_keys keys="F12,F12,F12"
tui_snapshot format=text
tui_diff baseline="connect-fade-mid"
```

Baselines `connect-fade-mid` and `approval-spring-mid` capture stepped frames; create with `create_if_missing: true` on first run.

## Baselines

Layout snapshots live in `.tuiwright/baselines/` (repo root). Names:

| Baseline | Scenario |
|----------|----------|
| `idle-wide` | S1 idle 120×40 |
| `help-expanded` | S2 after `?` |
| `breakpoint-90` | S3 standard width |
| `breakpoint-70` | S3 narrow (timeline unmounted) |
| `micro-55` | S3 micro overlay |
| `connect-fade-mid` | Stepped clock mid connect fade (F12 × N) |
| `approval-spring-mid` | Stepped clock mid approval spring |

First acceptance run: `tui_diff` with `create_if_missing: true` per baseline. Configure `[diff.ignore_patterns]` in `tuiwright.toml` (see example) so harness session ids and semver do not fail baselines. Help-expanded footer shows workspace **basename** only (not absolute path). Refresh baselines intentionally when layout changes; prefer `tui_assert` bundles for semantic leaks.

### S3 breakpoints (width)

rmux single-pane `tui_resize` may not change window dimensions on all hosts. When resize does not settle, use the reopen fallback per width:

```
tui_close
tui_open cols=90 rows=40
tui_wait_for text="Ready"
```

Repeat for 70 and 55, then restore `120×40` before later scenarios.

## Recordings

Motion review (S6 / hairline flux): `tui_record_start` → submit mock prompt → two `tui_snapshot format=text` while streaming → `tui_record_stop`. Text snapshots alone cannot verify Braille flux; review the `.cast` file or optional `tui_to_gif`. Recordings go under `.tuiwright/recordings/` (gitignored).

## Monochrome pass (optional S8)

Uncomment `NO_COLOR = "1"` in `tuiwright.toml` `[launch.env]` for one full matrix run.
