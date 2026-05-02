# REX editor extension

Consolidated reference for the VS Code / Cursor extension: **CLI consumer contract**, **architecture**, **behaviors**. Phasing remains in [`EXTENSION_ROADMAP.md`](EXTENSION_ROADMAP.md). Install and release remain in [`EXTENSION_RELEASE.md`](EXTENSION_RELEASE.md). System context: [`ARCHITECTURE.md`](ARCHITECTURE.md).

## Part A — Goals

- Stable boundary through **`rex-cli`** and NDJSON.
- Mode-driven UX: **`ask`**, **`plan`**, **`agent`** with deterministic guardrails and approvals where required.
- Thin host: markdown and code-block handling stay client-side; transport contract unchanged.

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
rex-cli complete "<prompt>" --format ndjson
```

- One JSON object per stdout line (`chunk`, `done`, `error`).
- Exactly **one** terminal event per request path (`done` **or** `error`).
- `error` objects may expose stable `code` for UX routing.

Current `error.code` taxonomy:

| Code | Meaning | Retry guidance |
|---|---|---|
| `daemon_unavailable` | Daemon not reachable | Retry after daemon start |
| `stream_timeout` | No stream activity in window | Retry usually safe |
| `stream_interrupted` | Mid-flight failure | Retry usually safe |
| `stream_incomplete` | No terminal marker | Inspect daemon; avoid blind retry |
| `cancelled` | User cancelled | No retry unless user resubmits |
| `invalid_response` | Malformed NDJSON | Inspect CLI/daemon logs |
| `spawn_failed` | `rex-cli` could not spawn | Fix install / PATH |
| `unknown` | Uncategorized | Manual diagnosis |

### Bootstrap flow

1. User selects mode. 2. Extension captures prompt/context. 3. Policy + approvals. 4. Spawn `rex-cli complete … --format ndjson`. 5. Parse lines. 6. Route `chunk` / `done` / `error` to UI.

### Reliability and trace correlation

Bounded CLI retries for daemon-unavailable races; interrupted streams terminate with `error`. Per-request **`REX_TRACE_ID`** correlates CLI, daemon, extension logs — [`CONFIGURATION.md`](CONFIGURATION.md).

### MVP non-goals

Direct extension gRPC; plugin lifecycle inside extension; fully autonomous unattended execution without approvals; coordinated multi-file agent (deferred architecture).

---

## Part C — Internal architecture

### Layering (`extensions/rex-vscode`)

| Layer | Responsibility |
|---|---|
| `src/extension.ts` | Activation + command wiring |
| `src/runtime/` | `rex-cli` / daemon interaction |
| `src/ui/` | View hosts |
| `src/editor/` | Context snapshots, virtual docs |
| `src/platform/` | Capability detection (`cursorAdapter`) |
| `src/config/` | Settings |
| `src/shared/` | Host ↔ webview message types |
| `webview/` | React chat |

### Transport

`rex-cli` child process `--format ndjson`; single terminal event invariant. Parsed markdown/code blocks remain extension-side only.

### Typed message bus (`src/shared/messages.ts`)

**Host → webview:** `streamStarted`, `streamChunk`, `streamDone`, `streamError`, `daemonState`, `modeState`, `approvalRequested`, `executionStep`.

**Webview → host:** `submitPrompt`, `cancelStream`, `applyCodeBlock`, `insertCodeBlock`, `copyCodeBlock`, `setMode`, `approvalDecision`.

### Mode orchestrator

Single host authority enforcing policy by mode (`ask`: no approvals for stream; **`agent`/`plan`** mutation/execution approvals per roadmap).

### Apply-to-file (`rex-proposal:` virtual doc → `vscode.diff` → `WorkspaceEdit`)

Accept/reject granularity: `file` or `selection` — unchanged behavior.

### Daemon lifecycle defaults

Manual `rex-daemon` vs opt-in **`rex.daemonAutoStart`**; states `ready`/`starting`/`unavailable` surfaced in UI.

### Settings snapshot

| Key | Default |
|---|---|
| `rex.cliPath` | `rex-cli` |
| `rex.daemonBinaryPath` | `rex-daemon` |
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
