# Rex operator UX (web desktop)


> Role: explanation | Status: active | Audience: operators | Read when: daily Rex usage
> Prefer: ## Product entry

## Product entry

Bare **`rex`** opens the **Rex desktop app** (Tauri + React). The daemon is ensured automatically on launch.

Design and acceptance: [WEB_UI_DESIGN.md](WEB_UI_DESIGN.md). Architecture: [WEB_UI_ARCHITECTURE.md](WEB_UI_ARCHITECTURE.md).

## Layout (wide profile)

- **Header** — workspace title, mode, status dot (Ready / Working).
- **Transcript** — conversation and streaming markdown (primary surface).
- **Timeline** — background tasks in human phrasing.
- **Composer** — prompt input; Enter to send, Shift+Enter for newline.
- **Footer** — status line and shortcut hints.

## Session flags

| Flag | Behavior |
|------|----------|
| `rex` | New session |
| `rex --continue` | Resume closed session (carousel — W107) |
| `rex --last` | Resume most recent closed session |

## Menu bar (macOS)

| Menu | Items |
|------|-------|
| Rex | About, Hide, Quit |
| Session | New, Continue…, Last |
| View | Reload |
| Help | About Rex |

## Keyboard

| Key | Action |
|-----|--------|
| Enter | Send message |
| Shift+Enter | New line in composer |
| ⌘K | Command palette (planned) |

## Setup subcommands

Unchanged: `rex config`, `rex proto`, `rex sidecar`, `rex gateway`, `rex omlx`.

## Related

- [CLI_OPERATOR_UX.md](historical/CLI_OPERATOR_UX.md) — superseded terminal UX reference
- [CONFIGURATION_OPERATOR.md](CONFIGURATION_OPERATOR.md)
