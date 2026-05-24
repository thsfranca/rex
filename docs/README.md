# Documentation index

Documentation follows **[Diátaxis](https://diataxis.fr/)** roles; the **architecture description + ADRs** live in [ARCHITECTURE.md](ARCHITECTURE.md) and [architecture/decisions/](architecture/decisions/).

| Role | Where to read |
|------|----------------|
| **Tutorials** (first success path) | [README.md](../README.md) quickstart, [EXTENSION_LOCAL_E2E.md](EXTENSION_LOCAL_E2E.md) |
| **How-to** (task recipes) | [RELEASE.md](RELEASE.md), [EXTENSION_RELEASE.md](EXTENSION_RELEASE.md), [CONFIGURATION.md](CONFIGURATION.md), [CI.md](CI.md), [DEPENDENCIES.md](DEPENDENCIES.md), [OBSERVABILITY_INTEGRATIONS.md](OBSERVABILITY_INTEGRATIONS.md) |
| **Explanation** (design intent) | [PURPOSE_AND_PRINCIPLES.md](PURPOSE_AND_PRINCIPLES.md), [V1_0.md](V1_0.md), [ARCHITECTURE.md](ARCHITECTURE.md), [ARCHITECTURE_GUIDELINES.md](ARCHITECTURE_GUIDELINES.md), [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md), [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md), [AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md), [POLICY_ENGINE.md](POLICY_ENGINE.md), [AGENT_RUNTIME_ENVIRONMENT.md](AGENT_RUNTIME_ENVIRONMENT.md), [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md), [LONG_TERM_MEMORY.md](LONG_TERM_MEMORY.md), [AGENT_KNOWLEDGE.md](AGENT_KNOWLEDGE.md), [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md), [MVP_SPEC.md](MVP_SPEC.md), [EXTENSION_ROADMAP.md](EXTENSION_ROADMAP.md) |
| **Reference** (narrow facts) | [ADAPTERS.md](ADAPTERS.md), [CACHING.md](CACHING.md), [EXTENSION.md](EXTENSION.md), [ERROR_HANDLING.md](ERROR_HANDLING.md), `proto/rex/v1/rex.proto`, [DOCUMENTATION.md](DOCUMENTATION.md) |

## New documentation checklist

Before adding or materially editing a doc under `docs/`:

1. Pick the **Diátaxis mode** (tutorial, how-to, explanation, reference) — see [DOCUMENTATION.md](DOCUMENTATION.md#documentation-by-purpose-diátaxis).
2. Pick the **target path** from the repository map below (or extend an existing hub).
3. Keep **one dominant topic** per file; split at section boundaries when needed.
4. **Link** to feature hubs and ADRs instead of copying long inventories.
5. **Update this README** — add a row to the repository map and/or Diátaxis role table in the same change.
6. For **new roadmap features**, create the hub first — see [DOCUMENTATION.md — Roadmap and new features](DOCUMENTATION.md#roadmap-and-new-features).
7. Run the [minimum checklist before merge](DOCUMENTATION.md#minimum-checklist-before-merge) in DOCUMENTATION.md.

## Repository map (`docs/`)

| Path | Purpose |
|------|---------|
| [PURPOSE_AND_PRINCIPLES.md](PURPOSE_AND_PRINCIPLES.md) | Project purpose and operating principles (canonical) |
| [DOCUMENTATION.md](DOCUMENTATION.md) | Writing checklist + doc-type norms |
| [DEVELOPER_EXPERIENCE_GUIDE.md](DEVELOPER_EXPERIENCE_GUIDE.md) | How to contribute and gate quality |
| [PRIORITIZATION.md](PRIORITIZATION.md) | MoSCoW + quick R-ICE |
| [V1_0.md](V1_0.md) | v1.0 release criteria (SMART), SemVer meaning, tagging gate |
| [ROADMAP.md](ROADMAP.md) | Post-v1.0 **Should** / **Could** queue, **RC-S*** closure, engineering backlog IDs |
| [ARCHITECTURE.md](ARCHITECTURE.md) | SAD + C4-style views + links to ADRs |
| [ARCHITECTURE_GUIDELINES.md](ARCHITECTURE_GUIDELINES.md) | Architecture policies, ownership, documentation layering |
| [AGENT_RUNTIME_ENVIRONMENT.md](AGENT_RUNTIME_ENVIRONMENT.md) | Deferred VM/container transport catalog |
| [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md) | Process sidecar: spawn, `rex.sidecar.v1`, UDS gRPC |
| [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md) | Product agent program (planned): `rex-agent`, unified CLI, JSON config |
| [AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md) | Environment access policy + sandbox profiles |
| [POLICY_ENGINE.md](POLICY_ENGINE.md) | Daemon policy pipeline (cache, approvals, access) |
| [architecture/decisions/](architecture/decisions/) | ADR index + template |
| [MVP_SPEC.md](MVP_SPEC.md) | Phase 1 product architecture and scope (not a second “done” list) |
| [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md) | Sidecar agent platform + brokered inference adapters |
| [ADAPTERS.md](ADAPTERS.md) | `InferenceRuntime` + capabilities |
| [CACHING.md](CACHING.md) | L1/L2/prefix policy |
| [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) | Pipeline + **economics lever matrix** |
| [LONG_TERM_MEMORY.md](LONG_TERM_MEMORY.md) | Long-term / project memory — **design hub** |
| [AGENT_KNOWLEDGE.md](AGENT_KNOWLEDGE.md) | Agent-oriented project knowledge — **design hub** |
| [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md) | Observability suite + economics validation — **design hub** |
| [OBSERVABILITY_INTEGRATIONS.md](OBSERVABILITY_INTEGRATIONS.md) | Connect Rex OTLP to operator observability stacks (BYOT) |
| [CONFIGURATION.md](CONFIGURATION.md) | `REX_*` catalog |
| [EXTENSION.md](EXTENSION.md) | Extension NDJSON contract + component layout |
| [ERROR_HANDLING.md](ERROR_HANDLING.md) | Error codes, human-readable messages, CI guidelines gate |
| [EXTENSION_ROADMAP.md](EXTENSION_ROADMAP.md) | Extension phasing |
| [RELEASE.md](RELEASE.md) | Core + extension release automation |
| [EXTENSION_RELEASE.md](EXTENSION_RELEASE.md) | Install & extension release |
| [EXTENSION_LOCAL_E2E.md](EXTENSION_LOCAL_E2E.md) | Clone → working editor |
| [CI.md](CI.md) | Automation gates |
| [DEPENDENCIES.md](DEPENDENCIES.md) | Toolchain prerequisites |

## Feature-area hub map

Use **one** canonical doc per row for feature **shape**; roadmaps link here—see [DOCUMENTATION.md](DOCUMENTATION.md).

| Feature area | Canonical hub |
|---|---|
| Inference adapters | [ADAPTERS.md](ADAPTERS.md), [ADR 0002](architecture/decisions/0002-inference-adapter-contract.md) |
| Layered prompts (planned) | [CONFIGURATION.md](CONFIGURATION.md#layered-prompts-planned) |
| Local MLX adapter (planned) | [ADAPTERS.md](ADAPTERS.md#local-mlx-path-planned) |
| Vendor KV / prompt cache hints (planned) | [CACHING.md](CACHING.md#vendor-kv-and-prompt-cache-hints-planned) |
| Daemon extensibility / sidecars | [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md), [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md), [ADR 0008](architecture/decisions/0008-dedicated-sidecar-control-plane-api.md) |
| Layered cache policy | [CACHING.md](CACHING.md), [ADR 0003](architecture/decisions/0003-layered-cache-agent-mode-policy.md) |
| Routing / optional gateway stance | [ADR 0004](architecture/decisions/0004-routing-daemon-first-optional-http-gateway.md), economics rows in [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) |
| Extension transport + NDJSON contract | [EXTENSION.md](EXTENSION.md), [ERROR_HANDLING.md](ERROR_HANDLING.md), [ADR 0007](architecture/decisions/0007-editor-extension-hybrid-transport-cli-and-grpc.md) |
| Extension UX phasing | [EXTENSION_ROADMAP.md](EXTENSION_ROADMAP.md) |
| Long-term / project memory | [LONG_TERM_MEMORY.md](LONG_TERM_MEMORY.md) |
| Agent knowledge for AI sessions (planned) | [AGENT_KNOWLEDGE.md](AGENT_KNOWLEDGE.md) |
| Observability suite + economics validation (planned) | [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md), [OBSERVABILITY_INTEGRATIONS.md](OBSERVABILITY_INTEGRATIONS.md), [ADR 0010](architecture/decisions/0010-daemon-exports-observability-via-otel-and-sidecar-api.md) |
| MCP-style tool interoperability (planned) | [ARCHITECTURE.md](ARCHITECTURE.md) interoperability rows, matrix in [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md), [ADR 0008](architecture/decisions/0008-dedicated-sidecar-control-plane-api.md) |
| Purpose and principles | [PURPOSE_AND_PRINCIPLES.md](PURPOSE_AND_PRINCIPLES.md) |
| Agent access + sidecar runtime | [AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md), [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md), [POLICY_ENGINE.md](POLICY_ENGINE.md), [ADR 0005](architecture/decisions/0005-rex-owns-sidecar-environment-not-agent-implementations.md), [ADR 0008](architecture/decisions/0008-dedicated-sidecar-control-plane-api.md) |
| Product agent delivery (planned) | [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md), [ROADMAP.md](ROADMAP.md) R013–R019 |

## Scripts (related)

| Path | Purpose |
|------|---------|
| [../scripts/install-extension.sh](../scripts/install-extension.sh) | Local VSIX build + install |
| [../scripts/dev-rex-extension.sh](../scripts/dev-rex-extension.sh) | Rust build + installer wrapper |

## Source vs generated artifacts

- Canonical text lives in **`docs/`** and source trees; **`target/`**, scratch logs, disposable VSIX artifacts stay out of Git unless a release flow dictates otherwise (`TEMP_*` coordination files remain gitignored).
