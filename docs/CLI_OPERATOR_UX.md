# CLI operator UX — design hub

**Status:** `design accepted` — **R071** / **R075** / **R072** / **R073** implemented; **R079** visual identity (this hub); **R080–R081** presentation and motion Open; **R074** Could ([ROADMAP.md](ROADMAP.md)). Architecture: [ADR 0039](architecture/decisions/0039-terminal-harness-presentation-and-daemon-intelligence.md), [TERMINAL_HARNESS_ARCHITECTURE.md](TERMINAL_HARNESS_ARCHITECTURE.md).

## Purpose

Give **terminal operators** the **primary** Rex experience: a single command in a project directory provisions a per-workspace daemon and opens a responsive, keyboard-driven multi-pane workspace. The CLI **ensures the daemon when configured**, surfaces **legible operator messaging**, and preserves **`rex complete --format ndjson`** for automation and CI ([ADR 0038](architecture/decisions/0038-cli-ndjson-stream-transport.md), [NDJSON_STREAM.md](NDJSON_STREAM.md)).

**Primary surface:** multi-pane TUI for **`rex tui`** and TTY **`rex complete`**. Plain text and NDJSON pipe modes remain for scripts.

## Language policy

Describe **target experience and acceptance criteria in Rex terms only**. Do not name other CLIs or assistants as UX benchmarks in this hub or in **R071–R081** PR text. Comparative landscape synthesis lives in the external research repository.

## North-star UX

Operators run **`cd ~/projects/my-app && rex`** (or **`rex tui`**) and enter an immersive terminal workspace without babysitting a foreground daemon session.

1. CLI derives workspace identity from cwd / **`workspace.root`** and probes the per-workspace UDS under **`$REX_ROOT/sockets/`** ([ADR 0036](architecture/decisions/0036-per-workspace-daemon-routing.md), **R075** Done).
2. When the socket is unresponsive and **`daemon.auto_start`** is on, CLI spawns a detached **`rex daemon`**, shows a loading state, and polls **`GetSystemStatus`** until the daemon is ready (context pipeline warmed, sidecar supervised).
3. TUI transitions to the primary layout: header, activity timeline, streaming output, composer, footer.
4. The operator interacts via keyboard; tool executions appear as timeline cards; destructive mutations pause for explicit approval routed through the daemon broker.
5. Session memory and diagnostics are assembled by the daemon context pipeline—the TUI is a deterministic projection of the NDJSON event stream.

## Target experience

- **Daemon lifecycle:** probe UDS → spawn if down → poll until ready / idle / unavailable.
- **Terminal UI:** header (system health, model, workspace), activity timeline, streaming markdown output, composer (prompt + mode), footer (keys and recovery).
- **Operator messaging (Must):** curated plain-language strings mapped from lifecycle phases and NDJSON events ([OPERATION_FEEDBACK.md](OPERATION_FEEDBACK.md)).
- **NDJSON parity:** core event-loop parsing matches the automation path; terminal-only enhancements (fuzzy search, mid-stream cancel) are additive.
- **LLM narrator (Could):** optional short summaries of multi-step runs; off by default; never on the critical path for stream start.

## Scope

**In:**

- Per-workspace daemon **ensure** semantics (probe → spawn → poll → ready / idle / unavailable).
- Detached daemon spawn with logs redirected (not the operator’s interactive terminal).
- Full TUI for interactive sessions on a TTY.
- Structured operator messaging catalog (lifecycle + stream events).
- Approval modals for guarded broker tools (`fs.write`, `exec.shell`, MCP).
- Config and flags for auto-start, UI mode, readiness timeout, log path, bypass modes (design intent; schema lands in implementation PRs).
- Alignment with NDJSON automation consumers ([NDJSON_STREAM.md](NDJSON_STREAM.md)).

**Out:**

- NDJSON wire-shape or daemon RPC changes for UX alone.
- macOS **`launchd`** / systemd user units (Could follow-up—“always-on daemon” tier).
- LLM narrator in the first implementation slice (**R074**).
- Daemon-owned LSP, git auto-commit, and FTS5 memory (**R076–R078** v2)—see [TERMINAL_HARNESS_ARCHITECTURE.md](TERMINAL_HARNESS_ARCHITECTURE.md).

## Current vs target gap

| Capability | Rex today | Target |
|------------|-----------|--------|
| Daemon start | Auto-start shipped (**R071**) | Done |
| Per-workspace routing | **R075** Done | Done |
| Lifecycle feedback | Header shows session word (`idle` / `streaming` / `error`) | Symbol-first health + phase (**R080**) |
| Stream progress | Activity list + output pane | Same panes; human phrases and glyphs, not tool tags (**R080**) |
| Interactive session | **`rex tui`** + TTY-delegating **`complete`** (**R073**) | Done (chrome); presentation program **R080–R081** |
| Markdown output | Incremental **mdstream** path | Done |
| Tool approval | Modal with tool name + detail | Human-first change summary; technical id secondary (**R080**) |
| Friendly status | Minimal structured copy | Progressive disclosure; optional narrator (**R074**) |
| Layout density | Fixed 35/65 split; full labels on every pane | Responsive breakpoints; calm density (**R080**) |
| Motion | Redraw on input/events only | Tick-driven cues (**R081**) |

## Boundaries

```mermaid
flowchart TB
 subgraph operator [Operator terminal]
 TUI[rex_tui_or_complete_TTY]
 end
 subgraph cli [rex CLI]
 Lifecycle[ensure_daemon]
 MsgLayer[operator_messaging]
 Transport[NDJSON_consumer]
 end
 subgraph daemon [rex-daemon]
 UDS[UDS_rex_v1]
 Policy[ApprovalGate]
 end

 TUI --> Lifecycle
 TUI --> MsgLayer
 TUI --> Transport
 Lifecycle -->|"spawn if down"| UDS
 Transport --> UDS
 Policy -->|"approval_required"| Transport
 MsgLayer -->|"maps events"| TUI
```

| Layer | Owns |
|-------|------|
| TUI | Layout, keyboard, markdown pane, activity timeline, composer, approval modals |
| CLI lifecycle | Probe, detached spawn, single-flight, readiness poll |
| Operator messaging | Event → human string mapping; optional narrator hook |
| CLI transport | Internal NDJSON consumer; pipe mode unchanged |
| Automation clients | **`rex complete --format ndjson`** subprocess contract |
| Daemon / sidecar | Orchestration, streaming authority, broker policy ([ADR 0001](architecture/decisions/0001-daemon-owns-agent-orchestration-and-economics.md)) |

Technical detail: [TERMINAL_HARNESS_ARCHITECTURE.md](TERMINAL_HARNESS_ARCHITECTURE.md).

## Entry flow

| Step | Behavior |
|------|----------|
| 1 | Operator runs **`rex`** or **`rex tui`** in a project directory |
| 2 | CLI resolves **`workspace.root`** and workspace-scoped socket path |
| 3 | Probe UDS; on failure spawn detached daemon when **`daemon.auto_start`** is true |
| 4 | Poll **`GetSystemStatus`** until ready or timeout; show loading UI |
| 5 | Open multi-pane TUI; composer accepts first prompt |

## Terminal UI layout

Multi-pane grid optimized for wide terminals. Rendering stack: **`ratatui`** + **`crossterm`**; incremental markdown via **`mdstream`** ([ADR 0039](architecture/decisions/0039-terminal-harness-presentation-and-daemon-intelligence.md)).

```mermaid
flowchart TB
 subgraph grid [TUI layout]
 Header[Header strip]
 subgraph body [Body]
 Activity[Activity timeline]
 Output[Output pane]
 end
 Composer[Composer]
 Footer[Footer keys]
 end
 Header --> body
 body --> Composer
 Composer --> Footer
```

| Pane | Content |
|------|---------|
| **Header** | Rex version, workspace path, active model, daemon/sidecar health, active LSP servers (v2), trace id |
| **Activity** | Live **`activity`**, **`tool`**, **`step`**, **`plan`** timeline |
| **Output** | Streaming markdown (incremental render; raw toggle optional) |
| **Composer** | Prompt input, mode indicator (`ask` / `plan` / `agent`), LTM indicator, sandbox mode |
| **Footer** | Key hints, error recovery actions |

### Entry points

| Command | When |
|---------|------|
| **`rex tui`** | Primary interactive shell |
| **`rex`** / **`rex complete`** on TTY without **`--format ndjson`** | Delegates to TUI when **`cli.ui.enabled`** is **`auto`** or **`true`** |
| **`rex complete --format ndjson`** | Piped / CI / automation—no TUI ([ADR 0038](architecture/decisions/0038-cli-ndjson-stream-transport.md)) |
| **`rex complete --no-ui`** / **`--format text`** | Script-style plain output on TTY |

### Keyboard map

| Key | Action |
|-----|--------|
| Enter | Submit composer prompt |
| Esc | Cancel active stream turn (sends cancellation on control plane) |
| Ctrl+C (once) | Same as Esc—cancel current turn |
| Ctrl+C (twice) | Exit CLI |
| Tab | Cycle focus: Composer → Output → Activity |
| Shift+Tab | Cycle mode: `ask` → `plan` → `agent` |
| Ctrl+Y | Toggle permission bypass for non-destructive file mutations mid-session |
| Ctrl+M | Model picker overlay (updates inference route via gRPC) |
| Ctrl+R | Fuzzy search over session memory index (v2) |
| Ctrl+L | Clear output pane |
| ? | Toggle footer help |
| `/mode <name>` | Slash fallback for mode switch |

Mode **`ask`** enforces daemon policy that denies **`fs.write`** and **`exec.shell`**. Mode **`plan`** restricts broker to read-only and **`plan.save`**. Mode **`agent`** unlocks full mutative tools subject to approval policy.

## Visual identity and operator presentation (**R079–R081**)

**Program status:** **R079** design, **R080** presentation, and **R081** motion are **Done**.

### Principles

| Principle | Intent |
|-----------|--------|
| **Calm density** | Output is primary; chrome and secondary panes recede. Prefer one border depth between terminal edge and content. |
| **Symbol-first status** | Mode, health, streaming, and approval use glyphs, weight, and color; words only when needed. Color is never the only signal (pair with glyph or letter). |
| **Progressive disclosure** | Tool ids, capability tags, full paths, and model ids stay hidden by default; reveal on focus, **`?`**, or approval expand. |
| **Responsive layout** | Panels keep stable roles; breakpoints change *which* panels show, not their meaning. |
| **Organic motion** | Tick-driven cues (stream caret, activity spinner, health pulse) convey progress without flashy effects. |

Optional **`cli.ui.reduce_motion`** uses static glyphs instead of animated frames. Theme remains adaptive to terminal capabilities ([TERMINAL_HARNESS_ARCHITECTURE.md](TERMINAL_HARNESS_ARCHITECTURE.md) decision checklist).

### Current baseline (shipped **R073**)

Observed on a live **`rex tui`** session (wide terminal, idle):

- **Header:** full path, `model=…`, `mode=…`, version string; status word `idle` / `streaming` / `error` inside the block.
- **Body:** fixed horizontal split (~35% Activity / ~65% Output) with titled borders (`Activity`, `Output`).
- **Activity:** plain-language ready line; no glyph phase markers.
- **Composer / footer:** titled `Composer`; footer is a full key-hint sentence (`Enter: submit | Esc: cancel | …`).
- **Approval modal (when shown):** leads with tool name and detail strings.
- **Layout:** no breakpoint behavior — narrow terminals still use the same split and long header line (truncation only).
- **Motion:** no animation tick; UI redraws on input and stream events only.

### Target presentation (**R080**)

| Pane | Default (symbol-first) | Disclosed on demand |
|------|------------------------|---------------------|
| **Header** | Workspace basename, mode glyph, health dot, compact phase cue | Full path, model id, version, trace id (`?` or overlay) |
| **Activity** | Short human phrase or symbol + phase from operator messaging ([OPERATION_FEEDBACK.md](OPERATION_FEEDBACK.md)) | Tool id / capability / raw detail on focus or expand |
| **Output** | Streaming markdown (primary focus) | Raw toggle optional (unchanged intent) |
| **Composer** | Prompt + mode glyph; minimal chrome | Mode name text when cycling or on `?` |
| **Footer** | Minimal key glyphs | Full help when **`?`** toggled |
| **Approval** | What will change (path / summary) | Technical tool id as secondary line |

**Default activity and approval copy must not** lead with broker capability tags or protocol-style identifiers. Map NDJSON **`tool`** / **`activity`** events through the messaging catalog; do not dump event field names into the default timeline.

### Responsive breakpoints (**R080**)

| Width (cols) | Layout |
|--------------|--------|
| **Narrow** (cols under 80) | Single column: Output primary; Activity collapsed or stacked below; header truncated to basename + glyphs |
| **Medium** (80–119) | Activity narrow + Output wide |
| **Wide** (120 and up) | Full header / activity / output / composer / footer grid |

Panels must not reorder on focus. Below a minimum size, show a clear “terminal too small” message rather than an unusable grid.

### Motion inventory (**R081**)

| Cue | When |
|-----|------|
| Stream caret / pulse on output edge | Session streaming |
| Activity row spinner | Tool or activity in progress |
| Header health pulse | Daemon ensure / reconnect |
| Focused pane border accent | Tab focus cycle |

Requires a **tick** in the app loop while cues are active (not only event-driven redraw). When **`cli.ui.reduce_motion`** is on, use static glyphs instead of animated frames.

### Run and validate

| Step | Command / tool |
|------|----------------|
| Install | `./scripts/install-cli.sh` (or `--skip-shell-path`) |
| Run | `rex tui` from a project directory |
| Agent live validation | External live harness (tuiwright MCP): `tui_open` → `tui_wait_for` / `tui_send_keys` → `tui_snapshot` with **text** format → `tui_close` |
| Resize checks | `tui_resize` then text snapshot at narrow / medium / wide |
| Motion checks | Sequential text snapshots while streaming (glyph change over time) |

A Rex **headless** TUI adapter (NDJSON replay + ANSI snapshot) is **Won't** — live PTY validation is enough ([TERMINAL_HARNESS_ARCHITECTURE.md](TERMINAL_HARNESS_ARCHITECTURE.md#testing-strategy)).

### Acceptance (program)

| ID | Done when |
|----|-----------|
| **R079** | This section is the implementer source of truth; roadmaps list **R080–R081**. |
| **R080** | Semantic tokens; breakpoints; default idle snapshot free of tool tags; human-first approval; live harness checks at narrow/medium/wide. |
| **R081** | Tick loop; streaming/activity/header cues; optional reduce_motion; sequential live snapshots show changing glyphs. |

## Tool approval UX

When the sidecar requests a host mutation, the daemon emits a **`tool`** event with **`approval_required`** status. The TUI:

1. Pauses output-pane auto-scroll.
2. Renders an interactive modal (activity overlay or dedicated pane).
3. For **`fs.write`**, shows a unified diff preview; operator navigates with arrow keys.
4. **`A`** (Approve) or **`D`** (Deny) transmits authorization to the daemon via unary gRPC—**not** client-side execution.
5. On approve, daemon **`ApprovalGate`** executes the mutation natively and resumes the stream.

TTY / **`--approval-id`** contract remains for non-TUI paths ([OPERATION_FEEDBACK.md](OPERATION_FEEDBACK.md)).

## Error recovery

| Condition | TUI behavior |
|-----------|--------------|
| **`daemon_unavailable`** | Preserve transcript; gray composer; footer: restart daemon or Esc to exit |
| **`sidecar_unavailable`** | Same retention; prompt sidecar recovery |
| **`workspace_mismatch`** | Show mismatch detail; retain visual state |

Auto-restart of a crashed daemon is attempted once with operator confirmation ([ADR 0039](architecture/decisions/0039-terminal-harness-presentation-and-daemon-intelligence.md)).

## Session model

| Capability | Behavior |
|------------|----------|
| New chat | Clean visual slate; daemon injects **`ProjectMemoryRetrieval`** from prior sessions (v2) |
| Resume | Load transcript from local storage; repopulate Activity and Output panes |
| Branch | Duplicate transcript to a chosen node; new correlation **`turn_id`** with daemon |

## NDJSON parity

The TUI operates as a **deterministic projection** of the NDJSON state machine ([TERMINAL_HARNESS_ARCHITECTURE.md](TERMINAL_HARNESS_ARCHITECTURE.md)). Core parsing, truncation of large tool outputs, and terminal semantics must match **`rex complete --format ndjson`** consumers. Terminal-only features must not alter wire shape or break pipe consumers.

## Operator messaging catalog (Must)

Fixed copy mapped from lifecycle and NDJSON events. Wording may tune in implementation; **semantics** must stay stable for tests.

### Lifecycle (daemon ensure)

| State | Operator message |
|-------|------------------|
| Probe success | Ready — connected to Rex |
| Probe fail, auto-start off | Rex is not running. Enable **`daemon.auto_start`** or run **`rex daemon`** |
| Starting spawn | Starting Rex… |
| Poll waiting | Waiting for Rex to become ready… |
| Ready | Rex is ready |
| Timeout | Rex did not become ready within {timeout}s — see {log_path} |
| Spawn error | Could not start Rex: {reason} |

### Stream — `activity` phases

| `phase` | Operator message (template) |
|---------|----------------------------|
| `thinking` | Thinking… |
| `tool_running` | Running tools… |
| `broker_wait` | Waiting on broker… |
| `compacting` | Compacting context… |
| `preparing` | Preparing response… |
| (other) | {summary} if non-empty, else Working… |

### Stream — `tool` events

| `status` / `phase` | Operator message (template) |
|---------|----------------------------|
| `running` | {name}: {detail} |
| `completed` | {name} done |
| `failed` | {name} failed: {detail} |
| `approval_required` | {name} needs approval |

### Stream — `step` / `plan`

| Event | Operator message (template) |
|-------|----------------------------|
| `step` | {summary} |
| `plan` | Plan: {title} |

### Terminal errors (reuse [ERROR_HANDLING.md](ERROR_HANDLING.md))

Map stable **`code`** values to one-line operator hints (for example **`sidecar_unavailable`** → “Sidecar is not running—check **`rex sidecar doctor`**”).

## LLM narrator (Could — R074)

Optional layer that summarizes a completed multi-tool turn in natural language.

- **Default:** off (**`cli.ui.narrator: false`**).
- **Trigger:** after terminal **`done`**, only when activity pane had more than N tool/step events (threshold TBD).
- **Model:** small/fast local or configured inference; must not block **`StreamInference`** start.
- **Non-goal:** replacing structured messaging or NDJSON events.

## Interfaces (intent)

### Configuration (planned — [CONFIGURATION.md](CONFIGURATION.md#cli-operator-ux-planned))

```json
{
  "daemon": {
    "auto_start": true,
    "ready_timeout_secs": 10,
    "idle_shutdown_secs": 300,
    "log_path": "~/.rex/daemon.log"
  },
  "cli": {
    "ui": {
      "enabled": "auto",
      "narrator": false,
      "sync_output": true
    }
  },
  "git": {
    "auto_commit_dirty": true
  }
}
```

Precedence: project **`.rex/config.json`** → **`$REX_ROOT/config.json`** → flags (**`--no-daemon-autostart`**, **`--no-ui`**).

| Key | Default | Purpose |
|-----|---------|---------|
| **`daemon.auto_start`** | **`true`** | CLI ensures daemon before client RPCs |
| **`daemon.ready_timeout_secs`** | `10` | Readiness poll budget |
| **`daemon.idle_shutdown_secs`** | **`300`** | Idle auto-shutdown (**R071b**); **`0`** disables |
| **`daemon.log_path`** | `~/.rex/daemon.log` | Detached daemon stdout/stderr |
| **`cli.ui.enabled`** | `"auto"` | `auto` \| `true` \| `false` — TUI on TTY |
| **`cli.ui.narrator`** | `false` | Optional LLM summaries (**R074**) |
| **`cli.ui.sync_output`** | `true` | Emit `?2026` synchronized output when terminal supports it |
| **`cli.ui.reduce_motion`** | `false` | Static glyphs instead of animated TUI cues (**R081**) |
| **`git.auto_commit_dirty`** | `true` | Daemon broker auto-commits dirty files before AI edits (**R077**) |

### CLI flags (planned)

| Flag | Purpose |
|------|---------|
| **`--no-daemon-autostart`** | Fail fast with **`daemon_unavailable`** |
| **`--no-ui`** | Force plain text on TTY |

## Delivery items and acceptance

### R071 — CLI daemon auto-start (Done)

- When **`daemon.auto_start`** is true and socket is missing, CLI spawns detached **`rex daemon`** and polls **`GetSystemStatus`** until ready or timeout.
- Managed inference children start during daemon boot before the socket binds when configured.
- Single-flight: concurrent CLI invocations do not spawn duplicate daemons.
- CLI-spawned daemon survives CLI exit until idle shutdown; manual **`rex daemon`** in foreground still supported.
- Error messages reference **`daemon.log_path`** on spawn/timeout failures.

### R072 — Structured messaging + NDJSON core (Phase 1)

- NDJSON **`TurnState`** consumer parses **`chunk`**, **`activity`**, **`tool`**, **`step`**, **`plan`**, **`done`**, **`error`** per [TERMINAL_HARNESS_ARCHITECTURE.md](TERMINAL_HARNESS_ARCHITECTURE.md).
- **`mdstream`** incremental markdown on stdout path: flicker-free streaming without full-buffer reparsing.
- Tool and activity events render as formatted operator lines interleaved with markdown.
- Lifecycle and stream events use curated strings per catalog above.
- Messaging works in plain **`--format text --verbose`** stderr before TUI chrome lands.
- Concurrent / out-of-order **`tool`** events keyed by **`tool_call_id`**.
- Cancel discards events for canceled **`turn_id`**.
- No extra LLM calls.

### R073 — Full terminal UI + approval modals (Phase 2–3)

- **`rex tui`** opens multi-pane **`ratatui`** layout; TTY **`rex complete`** respects **`cli.ui.enabled`**.
- Background tokio NDJSON consumer → **`mpsc`** → IMGUI draw loop; UI thread never blocks on I/O.
- **`--format ndjson`** on non-TTY stdout unchanged.
- Approval modals for **`fs.write`** / **`exec.shell`** with diff preview; keystrokes unblock daemon **`ApprovalGate`**.
- **`Shift+Tab`** mode switch; **`Ctrl+Y`** bypass toggle; **`?2026`** synchronized output when enabled.
- Cancel returns UI to idle; preserves TTY / **`--approval-id`** contract for automation.

### R074 — Optional LLM narrator (Could)

- Off by default; configurable via **`cli.ui.narrator`**.
- Post-turn summary only; does not alter NDJSON stream.
- Narrator copy must follow progressive disclosure (no tool-tag spam in default UI).

### R079 — TUI visual identity design (docs)

- This hub section **Visual identity and operator presentation** is the source of truth.
- Roadmaps list **R080** / **R081**; validation path documents live harness text snapshots.

### R080 — TUI presentation (tokens, responsive, symbols, disclosure)

- Semantic color tokens; no scattered one-off colors for status.
- Breakpoints per table above; live harness resize checks.
- Default activity and approval free of leading tool tags; human-first approval copy.

### R081 — TUI motion

- App-loop tick while cues active; stream / activity / header cues; optional **`cli.ui.reduce_motion`**.

## Prioritization

| Item | MoSCoW | Notes |
|------|--------|-------|
| Design hub + ADR 0039 | **Must** | This document + [TERMINAL_HARNESS_ARCHITECTURE.md](TERMINAL_HARNESS_ARCHITECTURE.md) |
| R071 auto-start | **Should** | **Done** |
| R075 per-workspace | **Must** | **Done** |
| R072 messaging + NDJSON core | **Must** (program) | **Done** |
| R073 TUI + approvals | **Should** | **Done** |
| R079 visual identity docs | **Should** | **Done** |
| R080 presentation | **Should** | **Done** |
| R081 motion | **Should** | **Done** |
| R074 narrator | **Could** | After R073; prefer after R080 disclosure rules |
| R076–R078 daemon intelligence | **Could** / **Later** | After TUI MVP |

**Current program focus:** [ROADMAP.md](ROADMAP.md) — TUI presentation **R080** → motion **R081**. LangFuse discovery unblocked for scheduling; **RC-LF1** remains **Not met**.

## Open questions

- Default TTY behavior: always TUI vs plain text until **`rex tui`** is explicit (recommend **`auto`**).
- Log rotation policy for **`daemon.log_path`**.
- Whether **`rex status`** should print friendly lifecycle copy when auto-start runs.
- Threshold N for narrator trigger (**R074**).

**Resolved** (see [ADR 0039](architecture/decisions/0039-terminal-harness-presentation-and-daemon-intelligence.md)): TUI framework **`ratatui`**; markdown **`mdstream`**; LSP in daemon; git pre-commit in broker; UDS gRPC + NDJSON automation parity.

## Related

- [TERMINAL_HARNESS_ARCHITECTURE.md](TERMINAL_HARNESS_ARCHITECTURE.md) — technical architecture spoke
- [ADR 0035](architecture/decisions/0035-cli-operator-ux-daemon-lifecycle-and-terminal-ui.md) — lifecycle + TUI intent
- [ADR 0038](architecture/decisions/0038-cli-ndjson-stream-transport.md) — NDJSON primary transport
- [ADR 0039](architecture/decisions/0039-terminal-harness-presentation-and-daemon-intelligence.md) — harness decisions
- [OPERATION_FEEDBACK.md](OPERATION_FEEDBACK.md) — NDJSON event catalog
- [NDJSON_STREAM.md](NDJSON_STREAM.md) — automation stream contract
- [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md) — client surfaces
- [CONFIGURATION.md](CONFIGURATION.md) — planned keys
- [ERROR_HANDLING.md](ERROR_HANDLING.md) — stable error codes
