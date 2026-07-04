# Agent documentation router


> Role: reference | Status: active | Audience: agents | Read when: task-scoped doc loading
> Prefer: ## Task routes

Task-oriented entry for AI assistants and contributors. Load **one primary doc** per task — not the full [README.md](README.md) index.

Machine catalog: [manifest.yaml](manifest.yaml). Human index: [README.md](README.md).

## Task routes

| Task | Primary doc | Also useful |
|------|-------------|-------------|
| Fix CI or automation gates | [CI.md](CI.md) | [CI_QUALITY_GATES.md](CI_QUALITY_GATES.md), [DEVELOPER_EXPERIENCE_GUIDE.md](DEVELOPER_EXPERIENCE_GUIDE.md) |
| Add or change error codes / CLI messages | [ERROR_HANDLING.md](ERROR_HANDLING.md) | [NDJSON_STREAM.md](NDJSON_STREAM.md) |
| NDJSON stream / CLI transport contract | [NDJSON_STREAM.md](NDJSON_STREAM.md) | [ERROR_HANDLING.md](ERROR_HANDLING.md), [ADR 0038](architecture/decisions/0038-cli-ndjson-stream-transport.md) |
| JSON config keys / env precedence | [CONFIGURATION.md](CONFIGURATION.md) | [CONFIGURATION_OPERATOR.md](CONFIGURATION_OPERATOR.md), [KISS_CONFIG_SURFACE.md](KISS_CONFIG_SURFACE.md) |
| Operator setup (daemon, LiteLLM, oMLX) | [CONFIGURATION_OPERATOR.md](CONFIGURATION_OPERATOR.md) | [CLI_OPERATOR_UX.md](CLI_OPERATOR_UX.md), [DEPENDENCIES.md](DEPENDENCIES.md) |
| TUI design / acceptance | [TUI_DESIGN.md](TUI_DESIGN.md) | [CLI_OPERATOR_UX.md](CLI_OPERATOR_UX.md), [TERMINAL_HARNESS_ARCHITECTURE.md](TERMINAL_HARNESS_ARCHITECTURE.md) |
| Sidecar spawn / broker API | [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md) | [AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md), [POLICY_ENGINE.md](POLICY_ENGINE.md) |
| Release / versioning | [RELEASE.md](RELEASE.md) | [V1_0.md](V1_0.md), [ROADMAP.md](ROADMAP.md) |
| System architecture overview | [ARCHITECTURE.md](ARCHITECTURE.md) | [ARCHITECTURE_GUIDELINES.md](ARCHITECTURE_GUIDELINES.md), [MVP_SPEC.md](MVP_SPEC.md) |
| Inference adapters / broker HTTP | [ADAPTERS.md](ADAPTERS.md) | [INFERENCE_GATEWAY.md](INFERENCE_GATEWAY.md), [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md) |
| Context / token economics | [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) | [CACHING.md](CACHING.md), [AGENT_GRAPH_ARCHITECTURE.md](AGENT_GRAPH_ARCHITECTURE.md) |
| Priorities / next work | [PRIORITIZATION.md](PRIORITIZATION.md) | [ROADMAP.md](ROADMAP.md), [V1_0.md](V1_0.md) |
| Writing or moving docs | [DOCUMENTATION.md](DOCUMENTATION.md) | This file, [manifest.yaml](manifest.yaml) |

## Do not load by default

- Full [README.md](README.md) repository map (use task routes above).
- [historical/](historical/) — cancelled or superseded programs only when researching history.

## Bundle pointer (KnowledgeRetrieval)

Hybrid pointer per [ADR 0015](architecture/decisions/0015-agent-knowledge-bundles.md):

| Field | Value |
|-------|-------|
| `bundle_id` | `rex-docs-v1` |
| `min_revision` | git commit of last manifest sync (daemon logs `knowledge=drift` on mismatch in agent mode) |

Payload: hub `## Summary` sections indexed via [manifest.yaml](manifest.yaml). See [AGENT_KNOWLEDGE.md](AGENT_KNOWLEDGE.md).
