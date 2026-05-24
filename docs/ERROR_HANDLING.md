# Error handling

Canonical hub for **how Rex surfaces failures** across daemon, CLI, extension, sidecar, broker, and plugins. Wire shapes for the editor path live in [EXTENSION.md](EXTENSION.md); this document defines **principles**, **message quality**, **code taxonomy**, and **CI enforcement**.

## Purpose and audiences

| Audience | Use this doc to |
|----------|-----------------|
| **Operators / extension users** | Understand what failed and what to try next (via CLI NDJSON and UI messages). |
| **Integrators** | Emit and consume stable `error.code` values on the NDJSON stream path. |
| **Contributors** | Author errors at layer boundaries; review PRs for actionable, non-leaking messages. |

Product errors are **not** the same as CI failure codes (`FMT_FAIL`, `GUIDELINES_FAIL`, …) — see [CI.md](CI.md).

## Principles

Adapted from [AIP-193](https://google.aip.dev/193), [gRPC error handling](https://grpc.io/docs/guides/error/), and [RFC 9457](https://www.rfc-editor.org/rfc/rfc9457):

1. **Two channels** — Every user-facing failure exposes a stable **code** (machine) and a **message** (human). Do not encode classification only in prose.
2. **Actionable messages** — Brief formula: **what failed → why/context → next step**. No crate names, stack traces, or internal type names on operator paths.
3. **Audience split**
   - **User/operator path** (NDJSON stdout, extension UI): plain language and recovery steps; link to setup docs when helpful.
   - **Developer/debug path** (daemon logs, `REX_TRACE_ID`): may include socket paths, env var names, and `source()` chains — never secrets or full sensitive file contents.
4. **Memorable codes** — `snake_case` words, not UUIDs. Use domain prefixes for broker/policy (`protected_path`) when codes leave the NDJSON stream taxonomy.
5. **Test codes, not prose** — Contracts and CI validate **code identity** and terminal outcomes; message wording may improve without breaking clients.
6. **Boundary policy** — Convert internal errors **once** at each boundary: daemon → gRPC, CLI → NDJSON, extension → UI hints.

```mermaid
flowchart LR
  subgraph internal [Internal Rust]
    BrokerError
    SupervisorError
    CliError
  end
  subgraph boundaries [Boundary outputs]
    GrpcStatus["gRPC Status"]
    NdjsonError["NDJSON error.code + message"]
    BrokerOk["Broker ok/error"]
    UiHint["Extension UX hint"]
  end
  BrokerError --> GrpcStatus
  BrokerError --> BrokerOk
  SupervisorError --> GrpcStatus
  CliError --> NdjsonError
  NdjsonError --> UiHint
```

## Message authoring guide

### Template (user/operator path)

> **[Component]** failed to **[action]**. **[Context]**. **[Next step]**.

### Good examples

**NDJSON (CLI → extension):**

> Sidecar is required for agent mode but is not running. Start `rex-daemon` with `REX_SIDECAR_ENABLED=1` and ensure `rex-sidecar-stub` is on PATH. See [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md).

**Daemon unavailable (`CliError`):**

> Daemon is unavailable at /tmp/rex.sock; run `rex daemon` and retry.

**Broker policy deny:**

> Access denied: path is protected by workspace policy (`protected_path`). Choose a file inside the workspace root.

### Bad examples (do not ship on user paths)

| Bad | Why |
|-----|-----|
| `SupervisorError: sidecar spawn failed: No such file or directory (os error 2)` | Leaks Rust type and OS errno without recovery steps. |
| `access policy denied (protected_path): /Users/.../secrets.env` | Exposes full host path; embeds code only in prose. |
| `[broker.inference error: connection refused]` inside stream **text** | Looks like success; not a terminal structured error. |
| `Something went wrong` | No code, no action. |

### Forbidden patterns (user paths)

- Raw `Debug` / `{err:?}` output
- Embedding broker or sidecar failures in **chunk text** instead of terminal `error` or gRPC status
- Bare `io::Error` or `tonic::Status` strings without conversion at the boundary

## Layer matrix

| Surface | Required fields | Transport | Owner |
|---------|-----------------|-----------|-------|
| NDJSON terminal | `event`, `code`, `message` | CLI stdout | `rex-cli` emits; extension consumes — [EXTENSION.md](EXTENSION.md) |
| gRPC stream failure | gRPC status code + message (+ Rex code in metadata when available) | UDS `rex.v1` | `rex-daemon` |
| Broker unary | `ok`, `error` string today; **`code` target** for policy denies | `rex.v1` broker RPCs | `rex-daemon` |
| Sidecar stream | Structured terminal error or RPC status — **not** inline stream text | `rex.sidecar.v1` | Sidecar + daemon — [ADR 0008](architecture/decisions/0008-dedicated-sidecar-control-plane-api.md) |
| Extension UI | Classify by `code` first; message heuristics fallback only | Host ↔ webview | Extension |
| CI signals | `CI_SIGNAL` / `fail_code` | GitHub Actions | [CI.md](CI.md) — not product errors |

## Error code catalog (NDJSON stream)

**Machine-readable source:** [`fixtures/guidelines/error_codes.yaml`](../fixtures/guidelines/error_codes.yaml) — CI validates this file against TypeScript and fixtures. Update **yaml and this table together** when adding a stream code.

| Code | Meaning | Retry | Owner | Message template (operator-facing) |
|------|---------|-------|-------|----------------------------------|
| `daemon_unavailable` | Daemon not reachable | Yes | both | Daemon is unavailable at {socket}; run `rex daemon` and retry. |
| `sidecar_unavailable` | Sidecar required but missing or unhealthy | No | both | Sidecar is required but unavailable: {detail}. Enable sidecar supervision and ensure the sidecar binary is on PATH. |
| `inference_config` | Inference backend not configured | No | both | Inference runtime not configured: {detail}. Set `REX_OPENAI_COMPAT_*` per [CONFIGURATION.md](CONFIGURATION.md). |
| `stream_timeout` | No stream activity within window | Yes | both | Timed out waiting for daemon stream chunk after {seconds}s. |
| `stream_interrupted` | Mid-flight transport failure | Yes | both | Daemon interrupted the stream before completion. |
| `stream_incomplete` | Stream ended without terminal marker | No | both | Daemon stream ended without completion marker. |
| `cancelled` | User cancelled | Yes* | extension | Cancelled by user. |
| `invalid_response` | Malformed NDJSON or unknown event | No | extension | Malformed or unknown NDJSON from CLI. |
| `spawn_failed` | Could not spawn `rex-cli` | No | extension | Failed to spawn rex-cli; fix install or PATH. |
| `unknown` | Uncategorized | No | both | Inspect daemon and CLI logs; classify with a stable code when root cause is known. |

\*Retry only if the user resubmits the prompt.

### Broker / policy codes (not NDJSON stream codes)

Documented for broker responses and future structured fields; **not** in `error_codes.yaml` today.

| Code | Meaning | Layer |
|------|---------|-------|
| `protected_path` | Path blocked by workspace policy | daemon broker |
| `path_empty` | Empty path on fs operation | daemon broker |

## Known gaps (current codebase)

These are **documented inconsistencies**; fixing them is follow-up work, not required to comply with message guidelines when touching unrelated code.

| Gap | Current behavior | Guideline target |
|-----|------------------|------------------|
| gRPC → NDJSON mapping | CLI classifies some `FailedPrecondition` errors by **message substring** | Stable Rex code in gRPC metadata; CLI maps by code |
| Approval deny / checkpoint | Often surfaces as NDJSON `unknown` | Dedicated stream or broker codes |
| Sidecar stub broker failures | Embedded as `[broker.* error: …]` in stream **text** | Terminal structured error or gRPC status |
| Broker proto | `ok` + `error` string only | Add `code` field; keep message human-readable |
| Extension heuristics | `errorTaxonomy.ts` substring fallbacks when CLI omits `code` | Prefer CLI `code`; shrink heuristics over time |
| `docs/EXTENSION.md` table | Was missing setup codes | Synced with this catalog — link here for full detail |

## Security and redaction

- Do **not** return secrets, tokens, or full contents of protected files in user-facing messages.
- Policy denies: name the **policy code** and operation; avoid echoing sensitive paths beyond what the user already supplied.
- Debug logs may include more context when correlated with `REX_TRACE_ID` — see [CONFIGURATION.md](CONFIGURATION.md).

## Review checklist (error-related PRs)

- [ ] User-path message follows the template; no internal type names.
- [ ] Stable **code** present at the boundary (NDJSON `error.code` for stream terminals).
- [ ] Exactly **one** terminal `done` or `error` per NDJSON request path.
- [ ] New stream codes added to `error_codes.yaml`, this table, and `StreamErrorCode` in the extension.
- [ ] Fixtures updated under [`fixtures/ndjson_contract/`](../fixtures/ndjson_contract/) when wire shape changes.
- [ ] `./scripts/ci/run_guidelines_verify.sh` passes locally when codes or fixtures change.

## Adding a new NDJSON stream error

1. Add row to [`fixtures/guidelines/error_codes.yaml`](../fixtures/guidelines/error_codes.yaml) and the catalog table above.
2. Extend `StreamErrorCode` in [`extensions/rex-vscode/src/shared/messages.ts`](../extensions/rex-vscode/src/shared/messages.ts) and [`ndjsonParser.ts`](../extensions/rex-vscode/src/runtime/ndjsonParser.ts) `asErrorCode`.
3. Map in `rex-cli` (`CliError` + `ndjson_error_code`) when the daemon/CLI emits it.
4. Add or extend NDJSON fixture + conformance tests.
5. Run `./scripts/ci/run_guidelines_verify.sh`.

## CI enforcement

**Script:** [`scripts/ci/run_guidelines_verify.sh`](../scripts/ci/run_guidelines_verify.sh)

Runs executable checks under [`scripts/ci/guidelines/`](../scripts/ci/guidelines/). On failure, CI emits `CI_SIGNAL code=GUIDELINES_FAIL` — see [CI.md](CI.md).

**Initial check — error codes** (`check_error_codes.sh`):

1. `error_codes.yaml` ↔ `StreamErrorCode` union in `messages.ts` (bi-directional)
2. Every yaml code appears in this document's catalog table
3. NDJSON contract fixtures use only registered codes on terminal `error` events

**Extensibility** — add sibling scripts (same job, no new workflow):

| Script (future) | Guideline source | Example rule |
|-----------------|------------------|--------------|
| `check_doc_hub_index.sh` | [DOCUMENTATION.md](DOCUMENTATION.md) | Every major `docs/*.md` hub listed in [README.md](README.md) |
| `check_ndjson_terminal.sh` | [EXTENSION.md](EXTENSION.md) | Fixture streams have exactly one terminal event |
| `check_no_home_paths.sh` | Project policy | No editor home paths in committed files |

Run locally before PRs that touch error codes or guidelines:

```bash
./scripts/ci/run_guidelines_verify.sh
```

## Related docs

- [EXTENSION.md](EXTENSION.md) — NDJSON wire contract and bootstrap flow
- [DEVELOPER_EXPERIENCE_GUIDE.md](DEVELOPER_EXPERIENCE_GUIDE.md) — quality gates and review checklist
- [MVP_SPEC.md](MVP_SPEC.md) — RC-08 sidecar-missing clear error
- [ADR 0008](architecture/decisions/0008-dedicated-sidecar-control-plane-api.md) — sidecar structured errors
- [ADR 0009](architecture/decisions/0009-centralized-agent-approvals-and-checkpoints.md) — approval deny semantics
- [POLICY_ENGINE.md](POLICY_ENGINE.md) — structured policy denies (planned)
