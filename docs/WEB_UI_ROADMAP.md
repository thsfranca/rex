# Web UI implementation roadmap


> Role: explanation | Status: active | Audience: contributors | Read when: web UI phased delivery
> Prefer: ## Phases

## Summary

Phased delivery for the **Electron + React** desktop harness ([ADR 0043](architecture/decisions/0043-electron-shell-for-electric-alive-compositor.md)). W100–W124 shipped on Tauri; **W125+** migrates the product shell to Electron so Electric Alive WebGL and visible chrome coexist. Supersedes cancelled **R090–R096** TUI visual identity program.

**Design hub:** [WEB_UI_DESIGN.md](WEB_UI_DESIGN.md). **Architecture:** [WEB_UI_ARCHITECTURE.md](WEB_UI_ARCHITECTURE.md).

## Phases

| Phase | ID | Concern | Status |
|-------|-----|---------|--------|
| 0 | **W100** | rex-ui-harness MCP + static probe | **Done** |
| 1 | **W101** | Tauri shell + `--rex-*` tokens | **Done** (legacy host) |
| 2 | **W102** | UDS bridge + simple streaming chat | **Done** |
| 3 | **W103** | `rex` launches desktop + menu bar | **Done** |
| 4 | **W104** | TUI removal | **Done** |
| 5 | **W105** | Motion v1 (Framer Motion tiers) | **Done** |
| 6 | **W106** | Approval modal + diff scrubber | **Done** |
| 7 | **W107** | Session picker carousel | **Done** |
| 8 | **W108** | Canvas/WebGL cinematic tier (regl) | **Done** |
| 9 | **W109** | tauri-plugin-playwright native harness | **Done** (superseded host → Electron) |
| 10 | **W110** | macOS signing + auto-update CI | In progress (release-desktop workflow stub) |
| 11 | **W111** | path-aware `ui-verify` CI matrix | **Done** |
| 12 | **W112** | In-app design system module + token lint | **Done** |
| 13 | **W113** | Responsive ShellGrid breakpoints | **Done** |
| 14 | **W114** | Motion tiers and presets | **Done** |
| 15 | **W115** | Chat surfaces + composer mode | **Done** |
| 16 | **W116** | Approval modal polish | **Done** |
| 17 | **W117** | Session list IPC + picker | **Done** |
| 18 | **W118** | Command palette + status + errors + test expansion | **Done** |
| 19 | **W119** | Electric Alive default tokens + aurora gradients | **Done** |
| 20 | **W120** | RK4 spring physics + motion orchestrator | **Done** |
| 21 | **W121** | AmbientCanvas v2 + ParticleField (regl) | **Done** (remount on Electron in W128) |
| 22 | **W122** | HairlineFlux + EdgeGlow region canvases | **Done** |
| 23 | **W123** | Component choreography (effect graph) | **Done** |
| 24 | **W124** | Harness baselines + motion scenario refresh | **Done** |
| 25 | **W125** | ADR 0043 Electron shell decision + compositor proof spec | **Done** |
| 26 | **W126** | Electron scaffold + compositor proof CI | **In progress** |
| 27 | **W127** | Electron main UDS/stream + load `apps/rex-web` | Planned |
| 28 | **W128** | Remount Electric Alive on Electron; `rex` launches Electron | Planned |
| 29 | **W129** | Harness on Electron; retire Tauri product path | Planned |

## Phase detail

### W101 — Shell MVP

- Electron monorepo (`apps/rex-desktop`, `apps/rex-web`)
- Wide-profile wireframe: header, transcript, timeline, composer, footer
- Harness token asserts on idle shell

### W102 — Chat MVP

- UDS gRPC proxy + `tauri::ipc::Channel` streaming
- react-virtuoso transcript with `skipAnimationFrameInResizeObserver`
- Zustand store; human-first copy from stream events

### W103 — CLI integration

- Bare `rex` spawns/focuses desktop window
- Native menu: Session, View, Help
- `--continue` / `--last` routed to desktop

### W104 — TUI removal

- Delete ratatui compositor and tuiwright fixtures
- No retrocompatibility

### W105 — Motion v1

- Framer Motion slide-ins, timeline stagger
- Tiered budget: Idle / Ambient / Active

### W106 — Approval gate

- Human-first modal; RK4 spring enter/exit
- Horizontal unified diff scrubber

### W107 — Session picker

- Scroll-snap carousel; preview snippets on hover

### W108 — Cinematic tier

- regl ambient shaders; battery-aware throttle via `navigator.getBattery()`

### W109 — Native harness

- tauri-plugin-playwright; WKWebView baselines

### W110 — Deployment

- Developer ID signing, notarytool, `tauri-plugin-updater`

### W111 — CI verify

- Path-aware `ui-verify` job: build gate on Linux, desktop harness on macOS
- Wired into `ci-checks` with `UI_FAIL` / skip semantics — [CI.md](CI.md)

### W112 — Design system module

- `apps/rex-web/src/design-system/` token scales + React primitives
- `./scripts/ci/lint_ui_tokens.sh` enforces token exclusivity on build leg

### W113 — Responsive shell

- `ShellGrid` + breakpoint CSS (Wide / Standard / Narrow / Micro)

### W114 — Motion system

- `design-system/motion/` presets wired to Framer wrappers

### W115 — Chat surfaces

- Transcript, Timeline, Composer on design system; mode segmented control

### W116 — Approval UX

- Modal spring presets; diff scrubber on tokens

### W117 — Sessions

- `list_closed_sessions` Tauri IPC; carousel populated from workspace history

### W118 — Operator chrome + tests

- Command palette (⌘K), status panel, error banner
- Vitest RTL for primitives; expanded harness CI scenarios

### W124 — Harness motion refresh

- Electric Alive token lint; streaming canvas flux + region motion asserts unchanged contract
- Follow-up note: WKWebView fullscreen regl buries chrome (#417); product host moves to Electron (W125+)

### W125 — Electron shell decision

- [ADR 0043](architecture/decisions/0043-electron-shell-for-electric-alive-compositor.md): Electron replaces Tauri/WKWebView for product shell
- Compositor proof spec in [WEB_UI_ARCHITECTURE.md](WEB_UI_ARCHITECTURE.md): chrome + WebGL co-visible ≥5s

### W126 — Electron compositor proof

- `apps/rex-desktop` Electron scaffold; loads `apps/rex-web`; compositor proof page under `proof/` (CI only)
- CI/local script fails on bury: `./scripts/ci/run_electron_compositor_proof.sh`
- Tauri `crates/rex-desktop` removed (no parallel shell)

### W127 — Electron daemon bridge

- Main-process UDS gRPC + stream fan-out; load `apps/rex-web/dist`

### W128 — Electric Alive on Electron

- Remount AmbientCanvas + ParticleField; `rex` launches Electron

### W129 — Harness + retire Tauri path

- rex-ui-harness desktop = Electron; remove product Tauri launch; DX docs

## Cancelled

| ID | Former concern | Reason |
|----|----------------|--------|
| **R090–R096** | TUI compositor, Braille flux, cinematic terminal surfaces | Superseded by web UI program ([ADR 0042](architecture/decisions/0042-web-desktop-presentation-pivot.md)) |

## Related

- [ROADMAP.md](ROADMAP.md)
- [PRIORITIZATION.md](PRIORITIZATION.md)
- [ADR 0043](architecture/decisions/0043-electron-shell-for-electric-alive-compositor.md)
