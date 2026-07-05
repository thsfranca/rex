# Rex web UI design system


> Role: explanation | Status: active | Audience: contributors | Read when: web UI acceptance criteria
> Prefer: ## Summary

## Summary

Product-grade **web-native desktop** design system: semantic tokens, motion tiers, layout breakpoints, and **rex-ui-harness** acceptance gates for the Tauri harness.

**Status:** `design accepted` — implementation **W100–W118** per [WEB_UI_ROADMAP.md](WEB_UI_ROADMAP.md). **Architecture:** [WEB_UI_ARCHITECTURE.md](WEB_UI_ARCHITECTURE.md), [ADR 0042](architecture/decisions/0042-web-desktop-presentation-pivot.md).

This document is the **acceptance gate** for presentation and motion work on the Rex desktop app. Pull requests that implement blink-only motion, code-like chrome, or wireframe overload **fail review** against this system.

**Operator path:** [OPERATOR_UX.md](OPERATOR_UX.md).

## Design principles

| Principle | Definition |
|-----------|------------|
| **Quiet Chrome, Loud Content** | Hairlines or absent borders. Transcript dominates hierarchy. |
| **Organic Choreography** | CSS transitions or spring physics. No sudden pops. |
| **Continuous Feedback** | Async work MUST animate (pulse, shimmer, shader flux). Static wait fails review. |
| **Calm Density** | Typography scale + whitespace — not nested cards. |
| **Progressive Insight** | Protocol fields hidden until hover, focus, or expand. |
| **Symbolic Purity** | Status uses icon geometry **and** semantic color. |
| **Spatial Permanence** | Layout does not jump on content reflow; breakpoints change configuration, not meaning. |

Configuration remains **`REX_ROOT`** plus JSON only. No cosmetic-only env vars or CLI flags.

## Anti-patterns (must fail review)

| Anti-pattern | Fail when |
|--------------|-----------|
| **Mediocre Blink** | Lone spinner or blinking caret is the sole activity signal |
| **Code-Like Chrome** | Default view shows `model=`, tool tags, `[INFO]`, log aesthetics |
| **Wireframe Overload** | Nested bordered cards on every message |
| **Christmas Tree Palette** | Hardcoded hex/RGB/HSL outside `--rex-*` tokens |
| **Layout Jitter** | Panel widths track content; scroll anchor lost on stream |
| **Flat Static Wait** | In-flight operations with zero motion |

## Token system

All styling MUST reference CSS custom properties (`--rex-*`). Raw color literals outside tokens are **automatic PR rejection** for agents.

### Design system module

Implementation lives in [`apps/rex-web/src/design-system/`](../apps/rex-web/src/design-system/):

| Path | Role |
|------|------|
| `tokens/*.css` | **Only** place raw color literals are allowed |
| `primitives.css` | Primitive component styles (token references only) |
| `primitives/*.tsx` | React primitives (`Button`, `Modal`, `Text`, `Surface`, `Stack`, `StatusDot`) |
| `theme/obsidian-calm.ts` | Legacy Obsidian token names (superseded by `electric-alive.ts`) |
| `theme/electric-alive.ts` | Typed token name exports for harness and TS consumers |

Feature components import primitives from `../design-system` (or `@/design-system` when aliased). Layout shell classes remain in `tokens.css` until migrated to `layout/ShellGrid`.

Token lint: `./scripts/ci/lint_ui_tokens.sh` (wired into `run_ui_verify.sh` build leg).

### Semantic color

| Token | Light | Dark | Application |
|-------|-------|------|-------------|
| `--rex-surface-base` | `#FAFAFA` | transparent / OS bg | Transcript background |
| `--rex-surface-raised` | `#FFFFFF` | `#1A1B20` | Timeline, composer |
| `--rex-surface-overlay` | `#FFFFFF` | `#24252B` | Modals, dropdowns |
| `--rex-surface-dimmed` | `rgba(0,0,0,0.2)` | `rgba(0,0,0,0.6)` | Modal backdrops |
| `--rex-text-primary` | `#111111` | `#E2E2E2` | Operator input, assistant text |
| `--rex-text-secondary` | `#666666` | `#A0A0A5` | Markdown body, system cues |
| `--rex-hairline-default` | `#E5E5E5` | `#303136` | Panel separators |
| `--rex-hairline-focus` | `#2563EB` | `#82A0FF` | Active panel borders |
| `--rex-status-success` | `#10B981` | `#86E59A` | Approval, checkmarks |
| `--rex-status-working` | `#2563EB` | `#82A0FF` | Active inference |
| `--rex-status-error` | `#EF4444` | `#FF6B6B` | Error banners |

### Typography

| Token | Family | Size | Line height | Weight |
|-------|--------|------|-------------|--------|
| `--rex-font-ui-base` | System Sans | 14px | 1.5 | 400 |
| `--rex-font-ui-bold` | System Sans | 14px | 1.5 | 600 |
| `--rex-font-mono-base` | JetBrains Mono | 13px | 1.6 | 400 |
| `--rex-font-mono-small` | JetBrains Mono | 11px | 1.4 | 400 |

### Motion

| Token | Duration | Easing | Application |
|-------|----------|--------|-------------|
| `--rex-ease-ambient` | 400ms | `cubic-bezier(0.33, 1, 0.68, 1)` | Connect fades |
| `--rex-ease-active` | 250ms | `cubic-bezier(0.25, 1, 0.50, 1)` | Hover, focus |
| `--rex-spring-modal` | 350ms | spring(mass=1, stiffness=300, damping=20) | Dialogs |

## Layout and breakpoints

CSS Grid macro-architecture; Flexbox for micro-components.

| Profile | Width | Behavior |
|---------|-------|----------|
| **Wide** | ≥ 1200px | Timeline fixed 280px; generative transcript padding |
| **Standard** | 960–1199px | Timeline 220px |
| **Narrow** | 640–959px | Timeline icon rail or bottom sheet |
| **Micro** | < 640px | Single column; timeline overlay drawer |

## Component catalog (intent)

| Component | Rules |
|-----------|-------|
| **Message block** | No explicit role tags; typography + avatar cues |
| **Streaming markdown** | Incremental render; code blocks with left accent + copy on hover |
| **Timeline row** | Human phrases default; hover reveals params and duration |
| **Composer** | Bottom dock; auto-resize textarea; mode segmented control |
| **Approval modal** | Human-first copy; horizontal diff scrubber for filesystem |
| **Session picker** | Horizontal scroll-snap carousel |

## Motion tiers

| Tier | FPS | Entry | Exit |
|------|-----|-------|------|
| **Idle** | 0 | No in-flight work | Input or stream event |
| **Ambient** | 15–30 | Streaming / background work | Complete + 2s decay |
| **Active** | 60 | Operator input | 2s idle |
| **Cinematic** | 60 | Modal, banners | Max 750ms per trigger |

Every animated component MUST include `prefers-reduced-motion` fallback.

## Default aesthetic

**Electric Alive** (default): deep void base with aurora gradients, luminous semantic accents, glass chrome, and continuous shader/particle motion during async work. Rollback theme: set `data-theme="obsidian"` on `<html>` for near-monochrome Obsidian Calm. **Glass Depth** (macOS blur) is folded into shell chrome.

## Effect graph (web)

| Trigger | Effect | Tier | Surface |
|---------|--------|------|---------|
| `generating` / `tool_running` | Aurora shader + hairline flux + status particle orbit | Ambient–Cinematic | Fullscreen + active panel hairline |
| Stream token delta | Edge glow pulse + particle burst at transcript tail | Active | Transcript |
| Timeline row add | Spring coalesce + HSL flash | Ambient | Timeline |
| Approval open | RK4 scale spring + backdrop HSL dim + particle ring | Cinematic | Modal |
| Approval close | Spring out (250ms cap) | Cinematic | Modal |
| Error | HSL shift to error hue + banner spring drop | Active | Header |
| Composer focus/typing | Edge glow sweep | Active | Composer |
| Session carousel focus | Spring scale/fade adjacent cards | Active | SessionPicker |
| Daemon connect | Viewport fade-in (400ms) | Ambient | Shell |

**Invariant:** while async work is in flight, at least one Ambient-tier motion MUST be visible — static wait fails review.

Spring modals use `--rex-spring-modal-stiffness` and `--rex-spring-modal-damping` from [`motion.css`](../apps/rex-web/src/design-system/tokens/motion.css).

## Agent PR gates

1. **Token exclusivity** — no raw hex/RGB/HSL in component code.
2. **Motion audit** — tier declared; reduced-motion fallback present.
3. **Harness gate** — rex-ui-harness MCP 100% pass on affected scenarios before merge.
4. **Copy audit** — broker tags translated to human phrasing in timeline.

## Validation

Minimum acceptance bundle (after `ui_open`):

1. `ui_goto_scenario` → `idle`
2. `ui_assert_token` on status indicator vs `--rex-status-success`
3. `ui_assert_layout` on shell grid
4. `ui_goto_scenario` → `streaming`; `ui_assert_motion` or canvas flux
5. `ui_close`

Setup: [fixtures/ui_probe/README.md](../fixtures/ui_probe/README.md), `./scripts/setup_ui_probe_env.sh`.

## Related

- [WEB_UI_ROADMAP.md](WEB_UI_ROADMAP.md)
- [WEB_UI_ARCHITECTURE.md](WEB_UI_ARCHITECTURE.md)
- [ADR 0042](architecture/decisions/0042-web-desktop-presentation-pivot.md)
