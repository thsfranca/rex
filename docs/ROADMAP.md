# Roadmap

**Purpose:** track **v1.0 closure** and **Could** follow-ups mapped to **[V1_0.md](V1_0.md)** release criteria. Must **RC-*** remain canonical in that hub. [PURPOSE_AND_PRINCIPLES.md](PURPOSE_AND_PRINCIPLES.md) states intent; [MVP_SPEC.md](MVP_SPEC.md) is Phase 1 **architecture and scope** (no separate completion status). [PRIORITIZATION.md](PRIORITIZATION.md) describes MoSCoW bucketing and light R-ICE scoring.

**Version:** workspace **`1.0.0`** preparatory in [Cargo.toml](../Cargo.toml); **v1.0 not Met** — observability Must **RC-LF1** open ([V1_0.md](V1_0.md)).

## Release criteria status

Canonical **RC-* definitions, evidence, and status** live only in **[V1_0.md](V1_0.md)**. Update that hub first; this roadmap tracks open work IDs (**R***, **LF-***) toward closure — not a duplicate criteria table.

**v1.0 today:** **Not Met** — Must **RC-LF1** open ([LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md)).

## Theme order (dependency mental model)

```mermaid
flowchart LR
 core[CoreStreaming_RC01_RC07]
 side[Sidecar_RC03]
 broker[BrokeredHttpTool_RC04]
 desktopClient[WebDesktop_ADR0042]
 policyBroker[AccessPolicyBroker_RC05]
 agent[R013_R038_agent_Done]
 webUI[W100_W118_web_UI_Done]
 langfuseDisc[LangFuse_discovery_LF-D]
 langfuseRm[LF-R01_dead_code_removal]
 langfuseImpl[LF-F01_export]
 v1[v1_0_RC_LF1_Met]
 core --> side
 side --> broker
 broker --> desktopClient
 side --> policyBroker
 core --> agent
 desktopClient --> agent
 agent --> webUI
 webUI --> langfuseDisc
 langfuseDisc --> langfuseRm
 langfuseRm --> langfuseImpl
 policyBroker --> v1
 desktopClient --> v1
 agent --> v1
 langfuseImpl --> v1
```

**Current focus:** Web UI **W125–W129** Electron shell migration ([WEB_UI_ROADMAP.md](WEB_UI_ROADMAP.md), [ADR 0043](architecture/decisions/0043-electron-shell-for-electric-alive-compositor.md)); LangFuse program **LF-D01+** toward **RC-LF1** ([LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md)).

## Now — stable baseline

Streaming/agent Must **RC-01–RC-10** are **Met**. Observability Must **RC-LF1** is **open** (v1.0 **not Met**). **RC-S1–RC-S2** and **RC-S6** (extension / validation Should) are **Met**. Product agent program **R013–R038** is **Done**. Rex-owned observability program **R043–R054** is **Cancelled**.

| Priority | What | Notes |
|----------|------|-------|
| **Should** | Optional hardening | Stream/log polish beyond **RC-07** (Met); `cargo-deny`, Semgrep (**R026** Could) |
| **Removed** | VS Code extension (in-repo) | Removed 2026-07 — desktop web UI is the operator surface; see [OPERATOR_UX.md](OPERATOR_UX.md), [ADR 0043](architecture/decisions/0043-electron-shell-for-electric-alive-compositor.md) |

## Next — prioritized queue (audit 2026-07-01)

Canonical scoring: [PRIORITIZATION.md — Current focus queue](PRIORITIZATION.md#current-focus-queue-audit-2026-07-01). **One PR per row** where feasible; merge-wait between slices.

| Rank | ID / theme | MoSCoW | RC-* | Source(s) | Status |
|------|------------|--------|------|-----------|--------|
| 1 | **W125–W129** — Electron shell for Electric Alive + visible chrome | **Must** | — | [WEB_UI_ROADMAP.md](WEB_UI_ROADMAP.md), [ADR 0043](architecture/decisions/0043-electron-shell-for-electric-alive-compositor.md) | Open |
| 2 | **LF-D01+** — LangFuse discovery + export | **Must** | **RC-LF1** | [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md), [LANGFUSE_DISCOVERY_ROADMAP.md](LANGFUSE_DISCOVERY_ROADMAP.md) | Open |
| 3 | **R067** — intent-aware retrieval for short prompts | **Should** | — | [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md#advisory-intent-retrieval-r067) | Open |
| 4 | **R068** — ask answer-first prompt policy | **Should** | — | [AGENT_GRAPH_ARCHITECTURE.md](AGENT_GRAPH_ARCHITECTURE.md#advisory-ask-efficiency-r067r070) | Open |
| 5 | **R069** — remove tool step caps | **Should** | — | [ADR 0034](architecture/decisions/0034-remove-tool-step-caps.md) | Open |
| 6 | **R070** — deterministic init intent gating | **Should** | — | [AGENT_GRAPH_ARCHITECTURE.md](AGENT_GRAPH_ARCHITECTURE.md#advisory-ask-efficiency-r067r070) | Open |
| 7 | **R040** — nightly live-smoke workflow | **Should** | RC-S6 | [ECONOMICS_VALIDATION.md](ECONOMICS_VALIDATION.md), [CI.md](CI.md) | Open |
| 8 | **R059** — `workspace.search` broker | **Should** | — | [AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md#workspace-search-broker-r059) | Open |
| — | **W100–W118** — web desktop operator surface | **Must/Should** | — | [WEB_UI_ROADMAP.md](WEB_UI_ROADMAP.md), [OPERATOR_UX.md](OPERATOR_UX.md) | **Done** |
| — | **W119–W124** — Electric Alive physics motion v2 | **Could** | — | [WEB_UI_DESIGN.md](WEB_UI_DESIGN.md) | **Done** |
| — | **R072–R073**, **R080–R082** — terminal harness (superseded) | — | — | [historical/CLI_OPERATOR_UX.md](historical/CLI_OPERATOR_UX.md) | **Done** (superseded by **W100+**) |
| — | **R090–R096** — TUI visual identity v2 | **Won't (now)** | — | [WEB_UI_ROADMAP.md](WEB_UI_ROADMAP.md) | **Cancelled** |

### LangFuse integration (RC-LF1) — deferred

Hub: [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md). Discovery: [LANGFUSE_DISCOVERY_ROADMAP.md](LANGFUSE_DISCOVERY_ROADMAP.md). **Sequencing:** terminal harness and web UI pivot **Done** (**R072–R073**, **W100–W118**); LangFuse program is **unblocked**. **RC-LF1** remains **Not met**; v1.0 tag blocked until export ships.

| RC-* | Theme | Status |
|------|-------|--------|
| **RC-LF1** | LangFuse Cloud receives Rex daemon economics export (OTLP); operator can inspect traces in Cloud UI | **Not met** — deferred |
| **RC-S6** | Live LLM smoke (**R039–R040**) | **Met** / **R040** open |

### Cancelled — Rex-owned observability (RC-S3–S5, R043–R054)

Superseded by LangFuse-primary stance ([LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md)). Shipped code remains until **LF-R01**.

| Former RC / ID | Status |
|----------------|--------|
| **RC-S3** | **Superseded** |
| **RC-S4**, **R043–R049** | **Cancelled** — [CHCE_ROADMAP.md](historical/CHCE_ROADMAP.md) |
| **RC-S5**, **R050–R051** | **Cancelled** |
| **R052–R054** | **Cancelled** |

### Done — product agent program

Canonical design: [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md). **`rex-agent`** shipped; **`rex-sidecar-stub`** = CI/harness default.

**Done:** **R013–R022**, **R017–R019**, **R027–R032**, **R034**, **R031**, **R037**, **R038**, **R023–R025**. **Could follow-ups only:** **R016**, **R033**, **R036**.

## Later — only if the core path stays healthy

| Priority | What | Source(s) | Notes |
|----------|------|-----------|--------|
| **Could** | L2 **semantic** cache | [CACHING.md](CACHING.md), [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md) | Out of v1.0 |
| **Could** | **oMLX managed local inference** (Mac) | [OMLX_INFERENCE.md](OMLX_INFERENCE.md), [ADR 0033](architecture/decisions/0033-omlx-managed-local-inference.md) | **Done** — in-daemon MLX deferred — [ADAPTERS.md](ADAPTERS.md#in-daemon-mlx-path-deferred) |
| **Could** | Native Anthropic Messages adapter (secondary) | [ADAPTERS.md](ADAPTERS.md#direct-anthropic-messages-api-planned--secondary), [ADR 0018](architecture/decisions/0018-gateway-first-multi-provider-inference.md) | After LiteLLM profile |
| **Could** | Gateway adapters beyond broker HTTP | [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md), [ADR 0004](architecture/decisions/0004-routing-daemon-first-optional-http-gateway.md) | After router story matures |
| **Could** | Vendor KV / prompt cache hints | [CACHING.md](CACHING.md#vendor-kv-and-prompt-cache-hints-planned) | Depends on outbound API |
| **Could** | **LF-F02–LF-F07** LangFuse features | [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md) | After **LF-F01** |
| **Should** | **R071** — daemon auto-start on operator launch | [OPERATOR_UX.md](OPERATOR_UX.md), [ADR 0035](architecture/decisions/0035-cli-operator-ux-daemon-lifecycle-and-terminal-ui.md) | **Done** |
| **Should** | **R071b** — daemon idle lifecycle + auto-shutdown | [ADR 0037](architecture/decisions/0037-daemon-idle-shutdown.md) | **Done** |
| **Must** | **R075** — per-workspace daemon routing | [ADR 0036](architecture/decisions/0036-per-workspace-daemon-routing.md), [CONFIGURATION.md](CONFIGURATION.md) | **Done** |
| **Could** | **R074** — optional LLM status narrator | [historical/CLI_OPERATOR_UX.md](historical/CLI_OPERATOR_UX.md) | Open — desktop may subsume |
| **Should** | **R077** — brokered git dirty-state auto-commit | [historical/TERMINAL_HARNESS_ARCHITECTURE.md](historical/TERMINAL_HARNESS_ARCHITECTURE.md), [POLICY_ENGINE.md](POLICY_ENGINE.md) | Open — **`git.auto_commit_dirty`** |
| **Could** | **R076** — daemon-owned LSP workspace diagnostics | [historical/TERMINAL_HARNESS_ARCHITECTURE.md](historical/TERMINAL_HARNESS_ARCHITECTURE.md) | Open |
| **Could** | **R078** — dynamic MCP approval schema UI | [historical/TERMINAL_HARNESS_ARCHITECTURE.md](historical/TERMINAL_HARNESS_ARCHITECTURE.md), [ADR 0016](architecture/decisions/0016-mcp-in-sidecar-envelope.md) | Open; may follow **R033** |
| **Must** | **LF-D01–LF-F01** — LangFuse discovery + export | [LANGFUSE_DISCOVERY_ROADMAP.md](LANGFUSE_DISCOVERY_ROADMAP.md) | Open — **RC-LF1** |
| **Won't (now)** | VM/container as **default Mac** sidecar envelope | [AGENT_RUNTIME_ENVIRONMENT.md](AGENT_RUNTIME_ENVIRONMENT.md) | Process + broker instead |
| **Won't (now)** | Self-hosted LangFuse on Mac | [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md) | Cloud default |
| **Won't** | Headless terminal adapter for external agent harnesses | [historical/TERMINAL_HARNESS_ARCHITECTURE.md](historical/TERMINAL_HARNESS_ARCHITECTURE.md) | Superseded by rex-ui-harness |

## Engineering backlog (refactor / contract IDs)

| ID | Theme | Priority |
|----|-------|----------|
| R004 | CLI NDJSON seam hardening | Done |
| R005 | Cross-boundary NDJSON conformance tests | Done |
| R007 | Policy engine / cache seams | Done |
| R008 | Centralized agent approvals | Done |
| R009 | CLI contract tests (approval-id, probe recovery) | Done |
| R010 | Broker `fs.write` | Done |
| R011 | Broker `exec.shell` allowlist | Done |
| **R012** | **AccessPolicy broker centralization** (RC-05) | **Done** |
| **R013** | Platform enablers (`BrokerListDir`, `RunTurn.model`, stream passthrough) | Done |
| **R014** | Unified `rex` CLI | Done |
| **R015** | JSON config + `rex proto install` | Done |
| **R082** | Config surface cleanup (JSON-only; sole product env **`REX_ROOT`**; dead key removal) | **Should** — Done (docs, schema, operator template) — [CONFIGURATION.md](CONFIGURATION.md) |
| **R083** | Doc agent retrieval structure (AGENTS router, manifest, historical lane) | **Should** — **Done** — [AGENTS.md](AGENTS.md), [manifest.yaml](manifest.yaml) |
| **R016** | Multi-active sidecar broadcast | Could — deferred |
| **R017–R038** | Product agent program | **Done** — see [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md) |
| **R023–R025** | Supply chain / SAST / Ruff | **Done** |
| **R026** | Rex-specific guidelines + optional Semgrep | Could |
| **R039** | Ollama live smoke harness | **Should** — **Met** |
| **R040** | Nightly live-LLM workflow | **Should** — Open |
| **R041–R042** | Gateway smoke / run manifest | Could |
| **R044** | Operation feedback during daemon execution | **Done** — [OPERATION_FEEDBACK.md](OPERATION_FEEDBACK.md) |
| **R055** | `web.search` via SearXNG capability sidecar | Could — **after R056** — [WEB_SEARCH.md](WEB_SEARCH.md) |
| **R056** | Host + capability sidecar fleet | Could — [CAPABILITY_SIDECARS.md](CAPABILITY_SIDECARS.md) |
| **R043–R054** | CHCE / Rex observability store program | **Cancelled** — [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md) |
| **LF-R01** | Remove Rex observability dead code | **Must** — after docs pivot merge |
| **LF-F01–LF-F07** | LangFuse integration features | **Should** — post-discovery |

## Parked in design docs

| Topic | When to pull in | Source |
|--------|-----------------|--------|
| **Remote** networking, **TLS**, **production auth** | Operator story + threat model ready | [MVP_SPEC.md](MVP_SPEC.md), [ARCHITECTURE.md](ARCHITECTURE.md) |
| **Wasm** in-process plugins | Sidecar path mature | [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md) |
| **Node gRPC `StreamInference`** in editor extension | New ADR supersedes hybrid policy | [ADR 0007](architecture/decisions/0007-editor-extension-hybrid-transport-cli-and-grpc.md) (extension removed) |
| **Long-term / project memory** | After benchmark gate | [LONG_TERM_MEMORY.md](LONG_TERM_MEMORY.md) |
| **Agent knowledge** | After R015 | [AGENT_KNOWLEDGE.md](AGENT_KNOWLEDGE.md) |
| **MCP in sidecar** | Deferred | [ADR 0016](architecture/decisions/0016-mcp-in-sidecar-envelope.md) |
| **Capability sidecar fleet + web search** | After **RC-LF1** headroom | [CAPABILITY_SIDECARS.md](CAPABILITY_SIDECARS.md), [WEB_SEARCH.md](WEB_SEARCH.md) |
| **LangFuse integration** | **Active** — harness pivot complete | [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md), [LANGFUSE_DISCOVERY_ROADMAP.md](LANGFUSE_DISCOVERY_ROADMAP.md) |
| ~~**Rex-owned observability suite**~~ | **Cancelled** — removal **LF-R01** | [OBSERVABILITY_AND_ECONOMICS.md](historical/OBSERVABILITY_AND_ECONOMICS.md) (superseded) |
| ~~**CHCE mmap program**~~ | **Cancelled** | [CHCE_ROADMAP.md](historical/CHCE_ROADMAP.md) |

**CI:** [CI.md](CI.md) — **RC-10** Met; live smoke **R039** Met; **R040** open.

## How to refresh this file

1. Update **[V1_0.md](V1_0.md)** **RC-*** status when a gap closes; mirror the compact table above.
2. Skim [MVP_SPEC.md](MVP_SPEC.md) when **scope** changes.
3. **New product features:** hub first — [DOCUMENTATION.md](DOCUMENTATION.md#roadmap-and-new-features).
4. Re-check [PRIORITIZATION.md](PRIORITIZATION.md) when moving rows.

### Prioritization audit (2026-07-01)

Pivot: **Web desktop** is the primary operator surface ([ADR 0042](architecture/decisions/0042-web-desktop-presentation-pivot.md)). **W100–W118** **Done**. **Next slice:** LangFuse discovery **LF-D01+** toward **RC-LF1**. Prior audit: 2026-06-09 (LangFuse-primary).

## Related

- [V1_0.md](V1_0.md) — release criteria (canonical **done**)
- [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md) — LangFuse hub
- [LANGFUSE_DISCOVERY_ROADMAP.md](LANGFUSE_DISCOVERY_ROADMAP.md) — discovery queue
- [docs/README.md](README.md) — documentation index
- [PRIORITIZATION.md](PRIORITIZATION.md) — bucketing and scoring
