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
| [0007](0007-editor-extension-hybrid-transport-cli-and-grpc.md) | Editor extension hybrid transport (`rex-cli` NDJSON primary, optional unary `rex.v1`) | Accepted |
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

## How to use

1. Copy [TEMPLATE.md](TEMPLATE.md).
2. Name files `NNNN-short-title.md` (sequential **number**, kebab-case **title**).
3. Fill **Status:** `Accepted` | `Superseded by ADR XXXX` | `Deprecated`.
4. When superseding: add a **Supersedes:** line on the new ADR and amend the superseded ADR status.

## Relation to code

Decisions attach to the **architecture description** in [../ARCHITECTURE.md](../ARCHITECTURE.md).
