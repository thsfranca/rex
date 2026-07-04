# Architecture Decision Records (ADRs)

REX logs **accept/reject architectural choices** here so rationale survives chat and onboarding.

## Index

| ADR | Title | Status |
|-----|-------|--------|
| [0001](0001-daemon-owns-agent-orchestration-and-economics.md) | Daemon owns agent orchestration and economics | Accepted |
| [0002](0002-inference-adapter-contract.md) | Inference adapter contract and capabilities | Accepted |
| [0003](0003-layered-cache-agent-mode-policy.md) | Layered cache and agent mode exclusions | Accepted |
| [0004](0004-routing-daemon-first-optional-http-gateway.md) | Routing: daemon-first, optional HTTP gateway | Accepted |
| [0005](0005-rex-owns-sidecar-environment-not-agent-implementations.md) | Rex owns sidecar environment, not third-party agent implementations | Accepted |
| [0006](0006-gitignored-temp-tracking-sequential-handoffs.md) | Gitignored TEMP tracking for sequential multi-agent repository work | Accepted |
| [0007](0007-editor-extension-hybrid-transport-cli-and-grpc.md) | CLI client hybrid transport (`rex-cli` NDJSON primary, optional unary `rex.v1`) | Superseded → [0038](0038-cli-ndjson-stream-transport.md) |
| [0038](0038-cli-ndjson-stream-transport.md) | CLI NDJSON stream transport (post-extension removal) | Accepted |
| [0008](0008-dedicated-sidecar-control-plane-api.md) | Dedicated sidecar control-plane API (brokered integration) | Accepted |
| [0009](0009-centralized-agent-approvals-and-checkpoints.md) | Centralized agent execution approvals and checkpoints | Accepted |
| [0010](0010-daemon-exports-observability-via-otel-and-sidecar-api.md) | Daemon exports observability via OTLP; sidecar observability API | Accepted |
| [0011](0011-workspace-binding-and-turn-context-authority.md) | Workspace binding and turn context authority | Accepted |
| [0012](0012-layered-prompt-assemblies.md) | Layered prompt assemblies | Accepted |
| [0013](0013-access-policy-broker-completion.md) | Access policy broker completion | Accepted |
| [0014](0014-long-term-memory-boundary.md) | Long-term memory boundary | Accepted |
| [0015](0015-agent-knowledge-bundles.md) | Agent knowledge bundles | Accepted |
| [0016](0016-mcp-in-sidecar-envelope.md) | MCP in sidecar envelope | Accepted |
| [0017](0017-single-active-sidecar-phase-1.md) | Single-active sidecar for Phase 1 | Accepted |
| [0018](0018-gateway-first-multi-provider-inference.md) | Gateway-first multi-provider inference (LiteLLM primary) | Accepted (design); partial supersede by 0019 |
| [0019](0019-inference-gateway-opt-in-litellm.md) | Inference Gateway — opt-in LiteLLM, daemon-controlled | Accepted (design) |
| [0020](0020-otel-genai-semconv-with-rex-pipeline-metrics.md) | OpenTelemetry GenAI semconv plus Rex pipeline metrics | Accepted |
| [0021](0021-rex-owned-economics-store-byot-visualization.md) | Rex-owned economics store; BYOT visualization | Accepted |
| [0022](0022-viewer-editor-subagent-topology.md) | Viewer/Editor subagent topology for product agent | Accepted |
| [0023](0023-hybrid-agent-serialization-boundaries.md) | Hybrid agent serialization at boundaries (formats per layer) | Accepted |
| [0024](0024-plan-mode-artifacts-and-plan-save-broker.md) | Plan mode artifacts and plan.save broker | Accepted |
| [0025](0025-dual-economics-store-engines.md) | Dual economics store engines (SQLite default, mmap opt-in) | Accepted |
| [0026](0026-rex-owned-storage-grafana-otel-datasource.md) | Rex-owned storage; bundled Grafana via Rex OTel datasource | Accepted |
| [0027](0027-chce-columnar-mmap-engine.md) | CHCE columnar-mmap observability engine (mmap opt-in) | Accepted |
| [0028](0028-host-and-capability-sidecar-fleet.md) | Host and capability sidecar fleet | Accepted |
| [0029](0029-web-search-via-capability-sidecar.md) | Web search via capability sidecar (SearXNG) | Accepted |
| [0030](0030-incremental-run-turn-streaming.md) | Incremental `RunTurn` streaming (live sidecar flush) | Accepted |
| [0031](0031-ask-mode-research-broker.md) | Ask-mode research tools (`web.search` broker) | Accepted |
| [0032](0032-soft-cap-continue-turn.md) | Soft cap pause and ContinueTurn | Superseded by [0034](0034-remove-tool-step-caps.md) |
| [0033](0033-omlx-managed-local-inference.md) | oMLX managed local inference (Mac, OpenAI-compat) | Accepted (design) |
| [0034](0034-remove-tool-step-caps.md) | Remove tool step caps | Accepted |
| [0035](0035-cli-operator-ux-daemon-lifecycle-and-terminal-ui.md) | CLI operator UX — daemon lifecycle parity and terminal UI | Accepted (design) |
| [0036](0036-per-workspace-daemon-routing.md) | Per-workspace daemon routing | Accepted |
| [0037](0037-daemon-idle-shutdown.md) | Daemon idle lifecycle and auto-shutdown | Accepted |
| [0039](0039-terminal-harness-presentation-and-daemon-intelligence.md) | Terminal harness — presentation layer and daemon intelligence | Superseded → [0042](0042-web-desktop-presentation-pivot.md) |
| [0040](0040-harness-session-transcript-authority.md) | Harness session transcript authority (daemon context, TUI viewport) | Accepted |
| [0041](0041-tui-hybrid-compositor-and-tiered-frame-budget.md) | TUI hybrid compositor and tiered frame budget | Superseded → [0042](0042-web-desktop-presentation-pivot.md) |
| [0042](0042-web-desktop-presentation-pivot.md) | Web desktop presentation pivot (Tauri 2 + React) | Accepted |

## How to use

1. Copy [TEMPLATE.md](TEMPLATE.md).
2. Name files `NNNN-short-title.md` (sequential **number**, kebab-case **title**).
3. Fill **Status:** `Accepted` | `Superseded by ADR XXXX` | `Deprecated`.
4. When superseding: add a **Supersedes:** line on the new ADR and amend the superseded ADR status.

## Relation to code

Decisions attach to the **architecture description** in [../ARCHITECTURE.md](../ARCHITECTURE.md).
