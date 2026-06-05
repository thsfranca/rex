# Documentation index

Documentation follows **[Diátaxis](https://diataxis.fr/)** roles; the **architecture description + ADRs** live in [ARCHITECTURE.md](ARCHITECTURE.md) and [architecture/decisions/](architecture/decisions/).

| Role | Where to read |
|------|----------------|
| **Tutorials** (first success path) | [README.md](../README.md) quickstart, [EXTENSION_LOCAL_E2E.md](EXTENSION_LOCAL_E2E.md) |
| **How-to** (task recipes) | [RELEASE.md](RELEASE.md), [EXTENSION_RELEASE.md](EXTENSION_RELEASE.md), [CONFIGURATION.md](CONFIGURATION.md), [CI.md](CI.md), [DEPENDENCIES.md](DEPENDENCIES.md), [OBSERVABILITY_INTEGRATIONS.md](OBSERVABILITY_INTEGRATIONS.md) |
| **Explanation** (design intent) | [PURPOSE_AND_PRINCIPLES.md](PURPOSE_AND_PRINCIPLES.md), [V1_0.md](V1_0.md), [ARCHITECTURE.md](ARCHITECTURE.md), [ARCHITECTURE_GUIDELINES.md](ARCHITECTURE_GUIDELINES.md), [INFERENCE_GATEWAY.md](INFERENCE_GATEWAY.md), [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md), [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md), [AGENT_GRAPH_ARCHITECTURE.md](AGENT_GRAPH_ARCHITECTURE.md), [AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md), [POLICY_ENGINE.md](POLICY_ENGINE.md), [AGENT_RUNTIME_ENVIRONMENT.md](AGENT_RUNTIME_ENVIRONMENT.md), [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md), [LONG_TERM_MEMORY.md](LONG_TERM_MEMORY.md), [AGENT_KNOWLEDGE.md](AGENT_KNOWLEDGE.md), [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md), [ECONOMICS_VALIDATION.md](ECONOMICS_VALIDATION.md), [MVP_SPEC.md](MVP_SPEC.md), [EXTENSION_ROADMAP.md](EXTENSION_ROADMAP.md), [EXTENSION_UX.md](EXTENSION_UX.md) |
| **Reference** (narrow facts) | [ADAPTERS.md](ADAPTERS.md), [CACHING.md](CACHING.md), [EXTENSION.md](EXTENSION.md), [ERROR_HANDLING.md](ERROR_HANDLING.md), [OBS_STORE_MMAP_FORMAT.md](OBS_STORE_MMAP_FORMAT.md), `proto/rex/v1/rex.proto`, [DOCUMENTATION.md](DOCUMENTATION.md) |

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
| [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md) | Product agent program (partial — shipped): `rex-agent`; stub CI default |
| [AGENT_GRAPH_ARCHITECTURE.md](AGENT_GRAPH_ARCHITECTURE.md) | Token-efficient sidecar graph — **design hub** (**R027–R033**) |
| [AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md) | Environment access policy + sandbox profiles |
| [POLICY_ENGINE.md](POLICY_ENGINE.md) | Daemon policy pipeline (cache, approvals, access) |
| [architecture/decisions/](architecture/decisions/) | ADR index + template |
| [MVP_SPEC.md](MVP_SPEC.md) | Phase 1 product architecture and scope (not a second “done” list) |
| [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md) | Sidecar agent platform + brokered inference adapters |
| [INFERENCE_GATEWAY.md](INFERENCE_GATEWAY.md) | Opt-in LiteLLM gateway (daemon-controlled; not a sidecar) |
| [ADAPTERS.md](ADAPTERS.md) | `InferenceRuntime` + capabilities |
| [CACHING.md](CACHING.md) | L1/L2/prefix policy |
| [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) | Pipeline + **economics lever matrix** |
| [LONG_TERM_MEMORY.md](LONG_TERM_MEMORY.md) | Long-term / project memory — **design hub** |
| [AGENT_KNOWLEDGE.md](AGENT_KNOWLEDGE.md) | Agent-oriented project knowledge — **design hub** |
| [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md) | Observability suite — **design hub** |
| [OBS_STORE_MMAP_FORMAT.md](OBS_STORE_MMAP_FORMAT.md) | Mmap economics store on-disk format (reference) |
| [ECONOMICS_VALIDATION.md](ECONOMICS_VALIDATION.md) | Economics validation program — **design hub** |
| [OBSERVABILITY_INTEGRATIONS.md](OBSERVABILITY_INTEGRATIONS.md) | Bundled Grafana suite (`rex obs up`) + optional interop |
| [CONFIGURATION.md](CONFIGURATION.md) | JSON config schema + deprecated `REX_*` catalog |
| [EXTENSION.md](EXTENSION.md) | Extension NDJSON contract + component layout |
| [ERROR_HANDLING.md](ERROR_HANDLING.md) | Error codes, human-readable messages, CI guidelines gate |
| [PLANNING_TOOLS.md](PLANNING_TOOLS.md) | Plan mode tools, stream contract, broker plan.save |
| [EXTENSION_ROADMAP.md](EXTENSION_ROADMAP.md) | Extension phasing |
| [EXTENSION_UX.md](EXTENSION_UX.md) | Extension integrated UX design hub |
| [RELEASE.md](RELEASE.md) | Core + extension release automation |
| [EXTENSION_RELEASE.md](EXTENSION_RELEASE.md) | Install & extension release |
| [EXTENSION_LOCAL_E2E.md](EXTENSION_LOCAL_E2E.md) | Clone → working editor |
| [CI.md](CI.md) | Automation gates |
| [CI_QUALITY_GATES.md](CI_QUALITY_GATES.md) | CI quality and security gates (**R023** Done; **R024–R026** planned) |
| [DEPENDENCIES.md](DEPENDENCIES.md) | Toolchain prerequisites |

## Feature-area hub map

Use **one** canonical doc per row for feature **shape**; roadmaps link here—see [DOCUMENTATION.md](DOCUMENTATION.md).

| Feature area | Canonical hub |
|---|---|
| Inference adapters | [ADAPTERS.md](ADAPTERS.md), [ADR 0002](architecture/decisions/0002-inference-adapter-contract.md) |
| Inference Gateway (LiteLLM default API) | [INFERENCE_GATEWAY.md](INFERENCE_GATEWAY.md), [ADR 0019](architecture/decisions/0019-inference-gateway-opt-in-litellm.md), [ADR 0018](architecture/decisions/0018-gateway-first-multi-provider-inference.md) |
| Layered prompts (design accepted) | [ADR 0012](architecture/decisions/0012-layered-prompt-assemblies.md), [CONFIGURATION.md](CONFIGURATION.md#layered-prompts-design-accepted) |
| Local MLX adapter (planned) | [ADAPTERS.md](ADAPTERS.md#local-mlx-path-planned) |
| Vendor KV / prompt cache hints (planned) | [CACHING.md](CACHING.md#vendor-kv-and-prompt-cache-hints-planned) |
| Daemon extensibility / sidecars | [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md), [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md), [ADR 0008](architecture/decisions/0008-dedicated-sidecar-control-plane-api.md) |
| Layered cache policy | [CACHING.md](CACHING.md), [ADR 0003](architecture/decisions/0003-layered-cache-agent-mode-policy.md) |
| Routing / optional gateway stance | [ADR 0004](architecture/decisions/0004-routing-daemon-first-optional-http-gateway.md), economics rows in [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) |
| Extension transport + NDJSON contract | [EXTENSION.md](EXTENSION.md), [ERROR_HANDLING.md](ERROR_HANDLING.md), [ADR 0007](architecture/decisions/0007-editor-extension-hybrid-transport-cli-and-grpc.md) |
| Extension UX phasing | [EXTENSION_ROADMAP.md](EXTENSION_ROADMAP.md) |
| Extension integrated UX | [EXTENSION_UX.md](EXTENSION_UX.md) |
| Long-term / project memory | [LONG_TERM_MEMORY.md](LONG_TERM_MEMORY.md) |
| Agent knowledge for AI sessions (planned) | [AGENT_KNOWLEDGE.md](AGENT_KNOWLEDGE.md) |
| Observability suite (partial — Phase 2 sqlite + OTLP; Grafana planned) | [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md), [OBS_STORE_MMAP_FORMAT.md](OBS_STORE_MMAP_FORMAT.md), [OBSERVABILITY_INTEGRATIONS.md](OBSERVABILITY_INTEGRATIONS.md), [CONFIGURATION.md](CONFIGURATION.md#observability), [ADR 0010](architecture/decisions/0010-daemon-exports-observability-via-otel-and-sidecar-api.md), [ADR 0020](architecture/decisions/0020-otel-genai-semconv-with-rex-pipeline-metrics.md), [ADR 0021](architecture/decisions/0021-rex-owned-economics-store-byot-visualization.md), [ADR 0025](architecture/decisions/0025-dual-economics-store-engines.md), [ADR 0026](architecture/decisions/0026-rex-owned-storage-grafana-otel-datasource.md) |
| Economics validation (planned) | [ECONOMICS_VALIDATION.md](ECONOMICS_VALIDATION.md) |
| MCP-style tool interoperability (design accepted) | [ADR 0016](architecture/decisions/0016-mcp-in-sidecar-envelope.md), [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md), [ADR 0008](architecture/decisions/0008-dedicated-sidecar-control-plane-api.md) |
| Purpose and principles | [PURPOSE_AND_PRINCIPLES.md](PURPOSE_AND_PRINCIPLES.md) |
| Agent access + sidecar runtime | [AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md), [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md), [POLICY_ENGINE.md](POLICY_ENGINE.md), [ADR 0005](architecture/decisions/0005-rex-owns-sidecar-environment-not-agent-implementations.md), [ADR 0008](architecture/decisions/0008-dedicated-sidecar-control-plane-api.md) |
| Product agent delivery (partial — shipped) | [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md), [ROADMAP.md](ROADMAP.md) R013–R022 |
| Token-efficient agent graph (design accepted) | [AGENT_GRAPH_ARCHITECTURE.md](AGENT_GRAPH_ARCHITECTURE.md), [ADR 0022](architecture/decisions/0022-viewer-editor-subagent-topology.md), [ADR 0023](architecture/decisions/0023-hybrid-agent-serialization-boundaries.md), [ROADMAP.md](ROADMAP.md) R027–R036 |
| CI quality and security gates (**R023** Done) | [CI_QUALITY_GATES.md](CI_QUALITY_GATES.md), [ROADMAP.md](ROADMAP.md) R024–R026 |
| Development assistance capabilities (daemon-owned context, turn contract) | [DEVELOPMENT_ASSISTANCE_CAPABILITIES.md](DEVELOPMENT_ASSISTANCE_CAPABILITIES.md), [ADRs 0011–0017](architecture/decisions/README.md) |
| Plan mode planning tools | [PLANNING_TOOLS.md](PLANNING_TOOLS.md), [ADR 0024](architecture/decisions/0024-plan-mode-artifacts-and-plan-save-broker.md) |

## Scripts (related)

| Path | Purpose |
|------|---------|
| [../scripts/install-cli.sh](../scripts/install-cli.sh) | Install unified `rex` binary to `~/.cargo/bin` |
| [../scripts/install-extension.sh](../scripts/install-extension.sh) | Local VSIX build + install |
| [../scripts/dev-rex-extension.sh](../scripts/dev-rex-extension.sh) | Rust build + installer wrapper |
| [../scripts/verify_mvp_local.sh](../scripts/verify_mvp_local.sh) | MVP preflight (build, rust verify, sidecar verify, product path) |

## Source vs generated artifacts

- Canonical text lives in **`docs/`** and source trees; **`target/`**, scratch logs, disposable VSIX artifacts stay out of Git unless a release flow dictates otherwise (`TEMP_*` coordination files remain gitignored).
