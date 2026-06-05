# REX editor extension

Consolidated reference for the VS Code / Cursor extension: **CLI consumer contract**, **architecture**, **behaviors**. Phasing remains in [`EXTENSION_ROADMAP.md`](EXTENSION_ROADMAP.md). Install and release remain in [`EXTENSION_RELEASE.md`](EXTENSION_RELEASE.md). System context: [`ARCHITECTURE.md`](ARCHITECTURE.md).

## Part A — Goals

- Stable boundary through **`rex`** subprocess NDJSON for streaming completion; optional unary **`rex.v1`** over UDS per **[ADR 0007](architecture/decisions/0007-editor-extension-hybrid-transport-cli-and-grpc.md)** (`rex-cli` shim).
- Mode-driven UX: **`ask`**, **`plan`**, **`agent`** with deterministic guardrails and approvals where required.
- **Thin client only:** the extension does **not** host the agent. Assistant reasoning runs in a **daemon-supervised sidecar**; `rex-daemon` brokers inference and tools ([MVP_SPEC.md](MVP_SPEC.md), [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md)).
- Markdown and code-block handling stay client-side; transport contract unchanged.
- **Integrated extension UX** (layout, composer, sessions, tool cards): design hub **[EXTENSION_UX.md](EXTENSION_UX.md)**; delivery rows **E-UX01…E-UX11** in **[EXTENSION_ROADMAP.md](EXTENSION_ROADMAP.md)**.

---

## Part B — MVP consumer contract

### Mode contract

| Mode | Purpose | Mutation behavior |
|---|---|---|
| `ask` | Research and explanation | Blocks file mutations. |
| `plan` | Structured planning and handoff | Mutations only after approval checkpoints. |
| `agent` | Guarded execution flow | Approval for execution and mutations. |

Rules: one active mode per session; visible transitions; insert/apply respect mode policy.

### Stream contract

```bash
rex complete "<prompt>" --format ndjson --mode <ask|plan|agent> [--model <id>]
```

- Extension passes **`--mode`** matching the active session mode on every `complete` call.
- Optional **`--model`** when the user sets **REX: Model Id** (`rex.modelId`) or passes `--model` on the CLI (otherwise daemon default applies).
- One JSON object per stdout line (`chunk`, `done`, `error`; additive non-terminal `tool`, `step`, `plan`).
- **`rex` flushes stdout after each NDJSON line** when the consumer is on a pipe (including the extension subprocess), so chunks are visible promptly instead of sitting in a block buffer.
- Exactly **one** terminal event per request path (`done` **or** `error`); `tool` and `step` lines may appear mid-stream and are non-terminal.

**Additive stream events (E-UX09):**

| Event | Fields | Purpose |
|-------|--------|---------|
| `tool` | `index`, `name`, `phase`, `detail?` | Broker tool lifecycle (`running`, `completed`, `failed`) |
| `step` | `index`, `phase`, `summary` | Orchestrator / subagent step visibility |

Fixtures: [`fixtures/ndjson_contract/tool_step_stream.ndjson`](../fixtures/ndjson_contract/tool_step_stream.ndjson).
- `error` objects may expose stable `code` for UX routing.

**Error codes and message quality:** canonical catalog, templates, and review checklist — [ERROR_HANDLING.md](ERROR_HANDLING.md). Wire-shape table (retry guidance):

| Code | Meaning | Retry guidance |
|---|---|---|
| `daemon_unavailable` | Daemon not reachable | Retry after daemon start |
| `sidecar_unavailable` | Sidecar required but unavailable | Fix sidecar setup; see [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md) |
| `inference_config` | Inference backend not configured | Edit JSON `inference.openai_compat`; see [CONFIGURATION.md](CONFIGURATION.md) |
| `stream_timeout` | No stream activity in window | Retry usually safe |
| `stream_interrupted` | Mid-flight failure | Retry usually safe |
| `stream_incomplete` | No terminal marker | Inspect daemon; avoid blind retry |
| `cancelled` | User cancelled | No retry unless user resubmits |
| `invalid_response` | Malformed NDJSON | Inspect CLI/daemon logs |
| `spawn_failed` | `rex` could not spawn | Fix install / PATH |
| `unknown` | Uncategorized | Manual diagnosis |

### Bootstrap flow

1. User selects mode. 2. Extension captures prompt/context. 3. Policy + approvals. 4. Spawn `rex complete … --format ndjson --mode <mode>`. 5. Parse lines. 6. Route `chunk` / `done` / `error` to UI.

### MVP agent behaviors (extension-owned)

| Behavior | MVP expectation |
|----------|-----------------|
| Mode selector | `ask` / `plan` / `agent` with visible policy summary |
| Execution approval | Required in **`agent`** before streaming starts |
| Mutation approval | Required for apply/insert in **`plan`** and **`agent`** |
| Apply to file | Diff + `WorkspaceEdit` after approval |
| Cancel | Returns UI to idle; terminal `error` with `cancelled` when applicable |
| Daemon status | Status bar reflects `ready` / `starting` / `unavailable` |

Daemon-side approval **context** over gRPC is **Should** follow-on ([ADR 0009](architecture/decisions/0009-centralized-agent-approvals-and-checkpoints.md)); extension policy satisfies Phase 1 acceptance in [MVP_SPEC.md](MVP_SPEC.md).

### Reliability and trace correlation

Bounded CLI retries for daemon-unavailable races; interrupted streams terminate with `error`. Per-request **`--trace-id`** on `rex complete` correlates CLI, daemon, and extension logs — [`CONFIGURATION.md`](CONFIGURATION.md).

### MVP non-goals

Using Node gRPC for **`StreamInference`** instead of **`rex` NDJSON**; **editor-only** RPCs on **`rex.v1`**; plugin lifecycle inside extension; fully autonomous unattended execution without approvals; coordinated multi-file agent (deferred architecture).

---

## Part C — Internal architecture

### Layering (`extensions/rex-vscode`)

| Layer | Responsibility |
|---|---|
| `src/extension.ts` | Activation + command wiring |
| `src/runtime/` | `rex` / daemon interaction |
| `src/ui/` | View hosts |
| `src/editor/` | Context snapshots, virtual docs |
| `src/platform/` | Capability detection (`cursorAdapter`) |
| `src/config/` | Settings |
| `src/shared/` | Host ↔ webview message types |
| `webview/` | React chat |

### Transport

`rex` child process `--format ndjson`; single terminal event invariant. Parsed markdown/code blocks remain extension-side only.

### Typed message bus (`src/shared/messages.ts`)

**Host → webview:** `streamStarted`, `streamChunk`, `streamDone`, `streamError`, `daemonState`, `modeState`, `approvalRequested`, `executionStep`.

**Webview → host:** `submitPrompt`, `cancelStream`, `applyCodeBlock`, `insertCodeBlock`, `copyCodeBlock`, `setMode`, `approvalDecision`.

### Mode orchestrator

Single host authority enforcing policy by mode (`ask`: no approvals for stream; **`agent`/`plan`** mutation/execution approvals per roadmap).

### Apply-to-file (`rex-proposal:` virtual doc → `vscode.diff` → `WorkspaceEdit`)

Accept/reject granularity: `file` or `selection` — unchanged behavior.

### Daemon lifecycle defaults

Manual `rex daemon` vs opt-in **`rex.daemonAutoStart`**; states `ready`/`starting`/`unavailable` surfaced in UI.

### Settings snapshot

| Key | Default |
|---|---|
| `rex.cliPath` | `rex` |
| `rex.daemonBinaryPath` | `rex` |
| `rex.daemonAutoStart` | `false` |

### Security / observability (extension side)

Strict webview CSP; clipboard via host; no analytics; **`OutputChannel("REX")`** for diagnostics; correlate structured lines with daemon/CLI traces.

### Packaging sketch

esbuild dual bundle (host excludes React); VSIX via `vsce`; see [`EXTENSION_RELEASE.md`](EXTENSION_RELEASE.md).

### Architecture non-goals (current slice)

Ghost-text inline completions; direct Node UDS client; MCP registration until daemon endpoint exists (`EXTENSION_ROADMAP.md` deferral).

## Related documents

| Document | Topic |
|---|---|
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | Whole-system architecture + ADRs |
| [`EXTENSION_ROADMAP.md`](EXTENSION_ROADMAP.md) | Phasing / forward work |
| [`EXTENSION_LOCAL_E2E.md`](EXTENSION_LOCAL_E2E.md) | Clone → working editor checklist |
| [`EXTENSION_RELEASE.md`](EXTENSION_RELEASE.md) | Packaging + release pipeline |
