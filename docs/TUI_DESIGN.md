# Rex TUI design system


> Role: explanation | Status: active | Audience: contributors | Read when: TUI acceptance criteria
> Prefer: ## Purpose


## Summary

Product-grade terminal UI design system: layout, typography, motion, and tuiwright acceptance gates for the `rex` harness.

**Status:** `design accepted` (**R082**) — implementation **R080** (layout + tokens) and **R081** (motion) **Done** (validated with live tuiwright MCP text snapshots on the workspace debug binary).

**Product hub:** [CLI_OPERATOR_UX.md](CLI_OPERATOR_UX.md) (operator path and lifecycle). **Architecture:** [TERMINAL_HARNESS_ARCHITECTURE.md](TERMINAL_HARNESS_ARCHITECTURE.md), [ADR 0039](architecture/decisions/0039-terminal-harness-presentation-and-daemon-intelligence.md).

This document is the **acceptance gate** for presentation and motion work on the **`rex`** TUI. Pull requests that implement blink-only motion, code-like chrome, or wireframe overload **fail review** against this system.

## Product intent

Operators run bare **`rex`** and enter an **immersive agent workspace** in the terminal.

- The **transcript** is the stage: conversation and streamed markdown.
- **Chrome is infrastructure:** it orients and signals without competing for attention.
- Intelligence, tools, policy, and memory stay in the **daemon** and supervised sidecar; the TUI is a **presentation client** of the NDJSON event stream.

The character grid is a high-fidelity canvas. Product-grade means calm density, progressive disclosure, semantic tokens, and **choreographed** motion—not a junior chrome pass.

## Design principles

| Principle | Definition |
|-----------|------------|
| **Quiet Chrome, Loud Content** | Hairlines and dim dividers only. Near-monochrome base; one accent used sparingly. Transcript dominates. |
| **Organic Choreography** | Time-based slide, fade, coalesce, dissolve. Motion guides the eye; it is not decoration alone. |
| **Continuous feedback during work** | While async harness work runs, the UI must animate (flux on active hairline). Static wait fails review. |
| **Calm Density** | Hierarchy via typography and luminance, not endless boxes. |
| **Progressive Insight** | Protocol ephemera (`model=`, tool tags, raw JSON) stay hidden until `?`, focus, or expand. |
| **Symbolic Purity** | Health and phase use glyphs **and** semantic color (color-not-only). |
| **Spatial Permanence** | Panels do not jump on focus or transient events. Breakpoints change *which* panels exist, not their meaning while visible. |

Configuration remains **`REX_ROOT`** plus JSON only. Implementation must not invent cosmetic environment variables or cosmetic-only CLI flags.

## Anti-patterns (must fail review)

| Anti-pattern | Fail when |
|--------------|-----------|
| **Mediocre Blink** | Single-cell toggle (for example block caret vs space) or a lone spinner character is the primary activity signal |
| **Code-Like Chrome** | Default view shows `model=`, `mode=`, tool tags, `[INFO]`, or log-tail aesthetics |
| **Wireframe Overload** | Border depth greater than one; titled boxes on every pane |
| **Christmas Tree Palette** | Colors hardcoded outside the token system |
| **Layout Jitter** | Panel widths track content length; scroll position lost on resize |
| **Chrome-to-Content** | Non-content cells exceed about 15% of a region; redundant static headers |

**Clutter audit:** count border nesting, redundant labels, and chrome-to-content ratio. Prefer removing chrome over adding decoration.

## Current debt

**R080** and **R081** are implemented and MCP-validated. **R090–R096** (visual identity v2) are **Planned** — see [Visual identity v2](#visual-identity-v2) and [Implementation roadmap](#implementation-roadmap).

Baseline motion uses region buffer post-process (tachyonfx-equivalent; tachyonfx 0.25 targets `ratatui-core` and is not type-compatible with ratatui 0.29). Idle paints only when dirty so tuiwright Quiet (≥300ms) succeeds. Known gaps until **R091+**: dash hairline flux instead of Braille, binary FPS poll, no effect graph or dirty rects.

Do not reintroduce blink-only activity cues, titled-box wireframe chrome, always-on idle redraw, or ad-hoc colors outside the token map.

## Layout system

### Regions

| Region | Role |
|--------|------|
| **Global Header** | One row: workspace basename, session title (when set), health/phase glyph, mode glyph |
| **Transcript** | Dominant canvas: conversation and streamed markdown (`mdstream`) |
| **Timeline** | Secondary rail: background tasks as human phrases (not tool tags) |
| **Composer** | Bottom dock; grows upward within a height cap |
| **Status Footer** | One hairline row: ephemeral feedback and minimal key hints |

### Breakpoints

| Profile | Columns | Behavior |
|---------|---------|----------|
| **Wide** | ≥ 120 | Timeline fixed **30** columns; transcript padding **2** cells |
| **Standard** | 80–119 | Timeline fixed **24** columns; padding **1** cell |
| **Narrow** | 60–79 | Timeline **unmounted**; header glyphs only |
| **Micro** | under 60 | Full-screen “terminal too small” (show current vs required columns) |
| **Short height** | ≤ 24 rows | Composer max **5** rows; transcript ≥ **50%** of viewport |

Panels must not reorder on focus. Focus uses **hairline.focus** (and weight), not layout shifts.

### Wireframes (normative)

**Idle (standard, ≥80 cols)** — calm; timeline muted; composer focused:

```text
● workspace ○
[Operator]                                              │ ○ Timeline
…                                                       │   No active tasks
                                                        │
────────────────────────────────────────────────────────┼────────
❯ _                                                     │
────────────────────────────────────────────────────────┴────────
○ Ready                                                   [?]
```

**Session picker v2** (`rex --continue`) — horizontal carousel; centered session at full weight, adjacent sessions fade:

```text
● workspace ○
        ◁  Fix autostart lock reclaim  ▷
              2h ago · yesterday
────────────────────────────────────────────────────────────
← → select · Enter open · Esc quit                        [?]
```

Rules: carousel uses Braille half-blocks for adjacent scale; spring focus transition (**R093**); relative timestamps `text.tertiary`.

**Streaming** — transcript grows; timeline shows human task phrases; status shows working:

```text
● workspace ◆
[Agent]                                                 │ ● Timeline
…streaming markdown…                                    │   ▸ Reading file
                                                        │   ▸ Searching
────────────────────────────────────────────────────────┼────────
[ Agent is typing… ]                                    │
────────────────────────────────────────────────────────┴────────
● Working…                                                [?]
```

**Approval** — modal over dimmed backdrop (dissolve), human-first action text; technical id only on `?`:

```text
░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
░░░ ╭─────────────────────────────────────────────╮ ░░░
░░░ │ ◎ Action required                           │ ░░░
░░░ │ Agent requests permission to continue.      │ ░░░
░░░ │ [A] Approve   [D] Reject   [?] Details      │ ░░░
░░░ ╰─────────────────────────────────────────────╯ ░░░
░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
○ Awaiting operator…
```

**Micro (under 60 cols)** — stop complex layout; show required width.

## Token system

Probe truecolor at init. Use hex when available; otherwise map to the 16-color ANSI fallback.

### Surfaces

| Token | Purpose | Truecolor | 16-color |
|-------|---------|-----------|----------|
| `surface.base` | Transcript background | Transparent | Transparent |
| `surface.raised` | Timeline, composer | `#1A1B20` | Black |
| `surface.overlay` | Modal fill | `#24252B` | DarkGray |
| `surface.dimmed` | Modal backdrop | Computed dim | Black |

### Text

| Token | Purpose | Style | Truecolor | 16-color |
|-------|---------|-------|-----------|----------|
| `text.primary` | Operator input, agent primary | Bold | `#E2E2E2` | White |
| `text.secondary` | System messages, markdown body | Normal | `#A0A0A5` | Gray |
| `text.tertiary` | Timeline, timestamps, placeholders | Dim | `#606165` | DarkGray |
| `text.accent` | Highlights, active cues | Normal | `#82A0FF` | LightBlue |

### Hairlines

| Token | Purpose | Truecolor | 16-color |
|-------|---------|-----------|----------|
| `hairline.default` | Panel separators | `#303136` | DarkGray |
| `hairline.focus` | Active panel | `#82A0FF` | LightBlue |

### Status (glyph + color)

| Token | Glyph | Truecolor | 16-color |
|-------|-------|-----------|----------|
| `status.success` | ✓ | `#86E59A` | LightGreen |
| `status.warning` | ⚠ | `#FFC266` | Yellow |
| `status.error` | ✗ | `#FF6B6B` | LightRed |
| `status.working` | ● | `#82A0FF` | LightBlue |
| `status.idle` | ○ | `#606165` | DarkGray |

All colors in implementation code must resolve through these tokens (or a thin map to ratatui `Color`). Ad-hoc `Color::Cyan` scatter fails review.

## Component recipes

| Component | Default | Disclosed (`?` / focus / expand) |
|-----------|---------|----------------------------------|
| **Header** | Workspace basename, session title (`text.secondary`, ≤48 chars), health/phase glyph, mode glyph | Full path, model id, version, harness session id |
| **Transcript** | Streamed markdown; code blocks use left accent bar, not heavy outer boxes; blank line between messages | Optional raw toggle |
| **Timeline** | Human phrases (“Reading source file”), staggered list | Inline expand: parameters, duration, technical id |
| **Composer** | Accent prompt glyph; grows up within height cap; top hairline uses `hairline.focus` when focused | Mode name text |
| **Approval** | Human-first “what will change” / permission ask | Unified detail: paths, commands, policy ids |
| **Footer** | Minimal key glyphs | Full help, path, version |

**Default activity and approval must not** lead with broker capability tags or protocol field names. Map NDJSON `tool` / `activity` events through operator messaging ([OPERATION_FEEDBACK.md](OPERATION_FEEDBACK.md)).

## Motion system

### Runtime (tiered frame budget)

Decouple the animation clock from NDJSON ingestion. The compositor scheduler selects poll interval:

| Tier | FPS | Poll ms | Enter | Exit |
|------|-----|---------|-------|------|
| **Idle** | 0 | block on I/O | no in-flight work, no active effects | input or NDJSON |
| **Ambient** | 15 | 66.6 | streaming / daemon work | complete + 2 s |
| **Active** | 30 | 33.3 | keystroke / scroll / focus | 2 s idle |
| **Cinematic** | 60 | 16.6 | modal spring, banner drop | max 750 ms |

- Render widgets first; **EffectGraph** post-process on the buffer with region targeting.
- Wrap **damage regions** in synchronized output (`CSI ?2026`) when `cli.ui.sync_output` — not necessarily full frames.
- UI thread stays decoupled from the NDJSON consumer task.
- **Reflow guard:** pause ambient/cinematic tiers when layout exceeds 16 ms; resume when stable.
- **No new environment variables.** Harness determinism via `REX_HARNESS_SESSION_ID` probe path ([ADR 0041](architecture/decisions/0041-tui-hybrid-compositor-and-tiered-frame-budget.md)).

### Effect graph registry

| Trigger | Node | Duration | Easing / physics | Region |
|---------|------|----------|------------------|--------|
| Daemon connect | `fade_in` | 400ms | QuadOut | Viewport |
| Stream start | `slide_in` | 250ms | SineOut | Transcript block |
| Token streaming | `braille_flux` | Continuous | Linear sweep | Hairline + tail |
| Timeline task add | `coalesce` | 300ms | BounceOut | Timeline row |
| Approval open | `spring_in` + `hsl_dim` | 350ms | RK4 + HSL | Modal + backdrop |
| Approval close | `spring_out` | 250ms | RK4 | Modal + backdrop |
| Error | `hsl_shift` + `banner_drop` | 300ms | Linear + spring | Header |
| History fetch | `braille_flux` | Continuous | Linear | Transcript hairline |
| Composer typing | `edge_glow` | While typing | HSL fade | Composer cells |
| Timeline focus | `expand_card` | 200ms | QuadOut | Inline card |

### Choreography (normative)

| Trigger | Effect | Duration | Easing | Region |
|---------|--------|----------|--------|--------|
| Daemon connect | fade_in | 400ms | QuadOut | Viewport |
| Stream start | slide_in (bottom) | 250ms | SineOut | New transcript block |
| Token streaming | Braille flux on trailing edge | Continuous | Linear | Incoming text edge + active hairline |
| Timeline task add | coalesce | 300ms | BounceOut | New timeline row |
| Approval open | spring_in (backdrop hsl_dim) | 350ms | RK4 | Backdrop + modal |
| Approval close | spring_out | 250ms | RK4 | Backdrop + modal |
| Error | hsl_shift + banner_drop | 300ms | Linear + spring | Header hairline |
| History fetch | Braille flux on transcript hairline | Continuous | Linear | Transcript hairline |

**Indeterminate work:** Braille flux on the **active component’s top hairline**, not a lone spinner cell or dash sweep.

**Mediocre Blink** (single-cell caret toggle or lone spinner as the only cue) **fails** this system.

### In-flight operations invariant

While **any** async harness work is in flight (daemon ensure, live stream, incremental or retroactive history fetch, approval with pending backend), the TUI **must** show choreographed motion — typically Braille flux on the **active region hairline** — at **Ambient tier (15 FPS)** minimum. A **static screen** during known async work **fails review**.

| Phase | Tier | Rule |
|-------|------|------|
| **Idle** (no in-flight work) | **0 fps** | Dirty-flag paint only; Quiet ≥300ms for tuiwright |
| **In-flight work** | **Ambient–Active** | Effect graph active until complete or error |

## Visual identity v2

Advanced motion and color beyond R081 baseline:

| Element | Specification |
|---------|---------------|
| **Braille flux** | U+2800–U+28FF sweep on active hairlines; low-contrast tertiary in peripheral vision |
| **HSL ambient** | Truecolor token interpolation for error wash, approval dim, composer edge glow |
| **Spring modals** | RK4 damped harmonic for approval and banners; block glyphs (▆ ▃ _) at cell boundaries for motion blur |
| **Session carousel** | Horizontal picker; adjacent sessions scale/fade via Braille half-blocks |
| **Expandable cards** | Focus timeline chip → push-down card with JSON/diff; replaces binary `?` as primary inspect path |
| **Diff scrubbing** | Horizontal keys in approval modal interpolate green/red line intensity through change chunks |
| **Momentum scroll** | EMA velocity + friction on transcript history (**Could**, after viewport cache) |

## Guardrails

- Ease-out curves for ambient motion; no bounce on idle states.
- Cinematic tier capped at **750 ms** per trigger; auto-decay to Active/Ambient.
- Sync output wraps **damage only**; disabled during rapid composer typing.
- Prolonged operator + daemon inactivity → all motion decays to **Idle** static.
- Pause decorative motion during markdown reflow spikes.

## Competitive benchmarks

| Scenario | Market baseline | Rex target |
|----------|-----------------|------------|
| Cold start | >2000ms JS warmup | <250ms interactive composer |
| Long stream reflow | Stutter, dropped input | Layout cache; <5ms echo |
| Approval | Y/n in log | Spring modal + diff scrub |
| Session resume | CLI flags only | Visual carousel picker |
| Error recovery | Traceback in prompt | Semantic banner + hsl shift |

## Interaction states

| State | Presentation |
|-------|----------------|
| Idle | Calm transcript; muted timeline; composer focused |
| Ensuring daemon | Working glyph; optional connect fade |
| Streaming | Transcript updates; timeline tasks; flux on active hairline |
| Fetching history | Flux on transcript hairline; footer “Loading…”; optional dim on unfetched rows |
| Approval required | Modal + dimmed backdrop; human-first copy |
| Error | Error token on affected region; status line message |
| Terminal too small | Micro overlay only |

## Implementation acceptance checklist

Implementation PRs (**R090–R096**) must pass **all** items. Any failure rejects the PR.

| # | Criterion | Fail if |
|---|-----------|---------|
| 1 | Zero blink guarantee | Single-cell blink or hardware blink is the primary activity signal |
| 2 | Semantic token purity | Hardcoded colors outside the token map |
| 3 | Motion interpolation | Instant pop; no time-based region effects for listed triggers |
| 4 | Progressive disclosure | Protocol tags or tool ids in default idle/streaming views |
| 5 | Tear-free frames | Sync not wrapping damage when sync_output is on |
| 6 | Spatial permanence | Layout jitter or scroll loss across breakpoints |
| 7 | Environmental purity | New cosmetic env vars or cosmetic-only CLI flags |
| 8 | Computational integrity | Cannot sustain Ambient tier while streaming without pegging CPU |
| 9 | In-flight motion | ensure / stream / history fetch leaves UI static with no region animation |
| 10 | Braille flux | Dash-only hairline sweep as primary activity cue |
| 11 | Compositor tiers | Binary poll only; no Idle/Ambient/Active/Cinematic scheduler |
| 12 | Harness determinism | Probe cannot step animation clock for mid-frame snapshots |

## Validation

| Step | Method |
|------|--------|
| Install | `./scripts/install-cli.sh` |
| Run | `rex` |
| Agent live probe | tuiwright MCP: `tui_open` → `tui_wait_for` / `tui_send_keys` → `tui_snapshot` (**text**) → `tui_close` |
| Breakpoints | `tui_resize` when it settles, else `tui_close` + `tui_open` at target cols; text snapshot at narrow / standard / wide |
| Motion | `tui_record_start` + sequential text snapshots while streaming; Braille hairlines need `.cast` or PNG review |
| Stepped motion | Harness probe: PTY clock step → mid-spring text snapshot ([fixtures/tui_probe/README.md](../fixtures/tui_probe/README.md)) |
| Design review | Apply **tui-design** skill (clutter audit, responsive floor, in-flight motion) against this document |

A Rex **headless** NDJSON-replay / ANSI-snapshot adapter remains **Won't** ([TERMINAL_HARNESS_ARCHITECTURE.md](TERMINAL_HARNESS_ARCHITECTURE.md#testing-strategy)).

## Implementation roadmap

| ID | Scope | Status |
|----|-------|--------|
| **R082** | This design system (baseline) | **Done** |
| **R080** | Layout + tokens | **Done** |
| **R081** | Motion (choreography, flux hairlines) | **Done** |
| **R090** | Advanced visual identity design system (docs v2) | **Planned** |
| **R091** | Hybrid compositor + tiered frame budget | **Planned** |
| **R092** | Braille flux + HSL ambient motion | **Planned** |
| **R093** | Cinematic surfaces (springs, carousel, banners) | **Planned** |
| **R094** | Single-surface expandable cards + diff scrub | **Planned** |
| **R095** | Dirty-rect diffing + smart sync | **Planned** |
| **R096** | Deterministic tuiwright motion baselines | **Planned** |

## Related

- [CLI_OPERATOR_UX.md](CLI_OPERATOR_UX.md)
- [TERMINAL_HARNESS_ARCHITECTURE.md](TERMINAL_HARNESS_ARCHITECTURE.md)
- [OPERATION_FEEDBACK.md](OPERATION_FEEDBACK.md)
- [ADR 0041](architecture/decisions/0041-tui-hybrid-compositor-and-tiered-frame-budget.md)
- [CONFIGURATION.md](CONFIGURATION.md) (`cli.ui.enabled`, `cli.ui.sync_output`)
