# ADR 0043: Electron shell for Electric Alive compositor

**Status:** Accepted  
**Date:** 2026-07-11  
**Supersedes (shell choice only):** [0042](0042-web-desktop-presentation-pivot.md) decisions 1–2, 4–7 (Tauri/WKWebView host and IPC host shape)  
**Preserves from 0042:** Option C (presentation client + daemon intelligence), UDS `rex.v1`, React presentation in `apps/rex-web`, no cosmetic-only config keys, macOS Apple Silicon v1  
**Aligns with:** [0001](0001-daemon-owns-agent-orchestration-and-economics.md), [0040](0040-harness-session-transcript-authority.md)

## Context

[ADR 0042](0042-web-desktop-presentation-pivot.md) chose **Tauri 2** over Electron for lower idle RAM and a Rust-aligned backend. The product design ([WEB_UI_DESIGN.md](../../WEB_UI_DESIGN.md) **Electric Alive**) requires fullscreen ambient **WebGL** (aurora shader) and particle fields while async work runs, **and** always-visible operator chrome.

On macOS, Tauri embeds **WKWebView**. Fullscreen / `inset:0` WebGL canvases are promoted above HTML chrome: the shell appears briefly, then only the background remains (#417). Workarounds that unmount Electric Alive layers satisfy visibility but fail the design gate; remounting regl reintroduces bury (#416 modal path).

Electric Alive and visible chrome are **one** product requirement. The Tauri/WKWebView host cannot deliver both with the required technique. Upstream Tauri cannot fix Apple WebKit compositor policy.

Hub: [WEB_UI_ARCHITECTURE.md](../../WEB_UI_ARCHITECTURE.md).

## Decision

1. **Primary operator surface:** **Electron** (Chromium) desktop application loading **React 19** from `apps/rex-web`. Bare **`rex`** launches or focuses this window. No terminal TUI fallback.

2. **Desktop shell choice:** **Electron** over Tauri/Wails for v1 — Chromium stacks WebGL with HTML predictably; Electric Alive shaders/particles stay mounted without burying chrome. Higher idle RAM vs Tauri is an accepted cost.

3. **Architecture pattern:** Preserve **Option C** — presentation client + daemon intelligence. Keep UDS gRPC as the client transport. Renderer **must not** open UDS.

4. **IPC hybrid model (Electron):**
   - **Unary control plane:** Electron **main** (preload `contextBridge`) → UDS tonic/`@grpc/grpc-js` (`GetSystemStatus`, `FetchSessionEvents`, `RespondToToolApproval`, …).
   - **Streaming plane:** Main subscribes to `StreamInference`, forwards normalized events to the renderer (IPC channel / EventEmitter — not per-chunk renderer round-trips).
   - **Backpressure:** ring buffer per subscription in main.
   - **Reconnection:** Main probes UDS; emits daemon lifecycle status to the renderer.

5. **Monorepo layout (target):**
   - `apps/rex-desktop-electron/` — Electron main, preload, packaging.
   - `apps/rex-web/` — React presentation (unchanged role).
   - `crates/rex-ui-harness/` — MCP + Playwright against **Electron**.
   - `crates/rex-desktop/` — Tauri shell **retired from product path** after Electron launch ships (archive/remove in follow-up slices).

6. **Stream normalization:** Prefer TypeScript gRPC client in Electron main first; keep `rex-stream-ui` semantics for event vocabulary. Renderer consumes the same DTO/effect shapes via Zustand.

7. **Validation — compositor proof (required):**
   - Before treating Electric Alive WebGL as shippable on a host, desktop verify MUST prove **chrome + fullscreen WebGL co-visibility** for **≥5 seconds**.
   - Proof samples: luminance of chrome (not background-only) **and** hit-test that shell contains composer/header target.
   - Sample at least at **t=0, 1s, 3s, 5s** after WebGL init.
   - Failure = bury regression (must fail CI). Empty harness TestExecution must not report success.
   - Desktop harness loads **`apps/rex-web/dist`**; mock only at daemon config (`REX_ROOT`).

8. **Configuration:** No new JSON keys for UI mode — process context (`rex` spawn) selects the desktop app ([KISS config surface](../../CONFIGURATION.md)).

9. **Platform scope (v1):** macOS Apple Silicon first; signing/notarization deferred as before (W107).

10. **UI framework:** Keep **React 19** and the existing design-system module for this migration series.

## Consequences

- **Positive:** Electric Alive WebGL and visible chrome can coexist; harness validation closer to Chromium DevTools; stops WKWebView dead-end iteration.
- **Negative:** Higher idle RAM (~150–400 MB class vs Tauri); Electron packaging and security baseline (contextIsolation, no Node in renderer); migrate IPC from Tauri invoke/Channel.
- **Risks / follow-up:** Electron compositor proof scaffold; daemon bridge in main; remount AmbientCanvas/ParticleField; harness Electron transport; retire Tauri product launch ([WEB_UI_ROADMAP.md](../../WEB_UI_ROADMAP.md) W125+).

## Superseded ADR amendments

- **ADR 0042:** Web-native pivot, Option C, React `apps/rex-web`, and daemon UDS boundary **remain**. **Tauri 2 / WKWebView as product shell**, Tauri Channel IPC host, and tauri-plugin-playwright as the native validation host are **superseded** by this ADR.
