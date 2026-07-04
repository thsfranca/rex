# ADR 0041: TUI hybrid compositor and tiered frame budget

- **Date:** 2026-07-04
- **Status:** Accepted

## Context

[R081](TUI_DESIGN.md) shipped region post-process motion as a flat cue list in `motion.rs`. Gemini deep-research synthesis (captured in techythings `rex-terminal-tui-fluidity-architecture.md`) identifies gaps: binary FPS polling, dash hairline flux instead of Braille, no effect graph, no dirty-rect diffing, and no deterministic harness clock.

[tachyonfx](https://github.com/ratatui/tachyonfx) 0.25 depends on `ratatui-core` and is not type-compatible with ratatui 0.29 in this workspace. External effect crates introduce version fragility on a core visual identity path.

Research proposed `REX_TEST_MODE` for deterministic animation. Rex config policy ([KISS config surface](../../CONFIGURATION.md)) limits product env vars to **`REX_ROOT`** only.

## Decision

1. **Hybrid compositor:** Immediate-mode ratatui layout → retained cell buffer → **EffectGraph** post-process → optional **dirty-rect** emit. Module under `crates/rex-cli/src/tui/compositor/`.
2. **Tiered frame budget:** Idle (0 FPS), Ambient (15), Active (30), Cinematic (60, max 750 ms per trigger). Scheduler drives `crossterm` poll interval.
3. **No tachyonfx dependency:** In-repo easing, spring (RK4), Braille raster, and HSL helpers.
4. **Harness determinism:** When the process runs in the **tuiwright probe fixture** (`REX_ROOT` → `fixtures/tui_probe/rex_root`, cwd → `fixtures/tui_probe/workspace`), use **SteppedClock** — F12 advances animation time 16 ms per press; no extra env vars.
5. **Synchronized output policy:** `cli.ui.sync_output` wraps **damage regions** only; disable sync batching during rapid composer typing bursts.
6. **Reflow guard:** Pause ambient/cinematic effects when markdown layout exceeds 16 ms; resume when transcript stabilizes.

## Consequences

- **Positive:** Cinematic visual identity with agent-testable motion; CPU/battery guardrails; portable Unicode-only effects.
- **Negative:** Larger TUI module surface; harness clock protocol to maintain in probe docs.
- **Risks:** Sync output + scroll regions can leak status text in some emulators — dirty-rect path must bound damage.

## Related

- [ADR 0039](0039-terminal-harness-presentation-and-daemon-intelligence.md) — presentation vs intelligence
- [TUI_DESIGN.md](../../TUI_DESIGN.md) — acceptance gate
- [TERMINAL_HARNESS_ARCHITECTURE.md](../../TERMINAL_HARNESS_ARCHITECTURE.md) — testing strategy
