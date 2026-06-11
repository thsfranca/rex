# Documentation index

Documentation follows **[Diátaxis](https://diataxis.fr/)** roles; the **architecture description + ADRs** live in [ARCHITECTURE.md](ARCHITECTURE.md) and [architecture/decisions/](architecture/decisions/).

| Role | Where to read |
|------|----------------|
| **Tutorials** (first success path) | [README.md](../README.md) quickstart, [EXTENSION_LOCAL_E2E.md](EXTENSION_LOCAL_E2E.md) |
| **How-to** (task recipes) | [RELEASE.md](RELEASE.md), [EXTENSION_RELEASE.md](EXTENSION_RELEASE.md), [CONFIGURATION.md](CONFIGURATION.md), [CI.md](CI.md), [DEPENDENCIES.md](DEPENDENCIES.md) |
| **Explanation** (design intent) | [PURPOSE_AND_PRINCIPLES.md](PURPOSE_AND_PRINCIPLES.md), [V1_0.md](V1_0.md), [ARCHITECTURE.md](ARCHITECTURE.md), [ARCHITECTURE_GUIDELINES.md](ARCHITECTURE_GUIDELINES.md), [INFERENCE_GATEWAY.md](INFERENCE_GATEWAY.md), [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md), [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md), [AGENT_GRAPH_ARCHITECTURE.md](AGENT_GRAPH_ARCHITECTURE.md), [AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md), [POLICY_ENGINE.md](POLICY_ENGINE.md), [AGENT_RUNTIME_ENVIRONMENT.md](AGENT_RUNTIME_ENVIRONMENT.md), [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md), [LONG_TERM_MEMORY.md](LONG_TERM_MEMORY.md), [AGENT_KNOWLEDGE.md](AGENT_KNOWLEDGE.md), [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md), [LANGFUSE_DISCOVERY_ROADMAP.md](LANGFUSE_DISCOVERY_ROADMAP.md), [ECONOMICS_VALIDATION.md](ECONOMICS_VALIDATION.md), [MVP_SPEC.md](MVP_SPEC.md), [EXTENSION_ROADMAP.md](EXTENSION_ROADMAP.md), [EXTENSION_UX.md](EXTENSION_UX.md) |
| **Reference** (narrow facts) | [ADAPTERS.md](ADAPTERS.md), [CACHING.md](CACHING.md), [EXTENSION.md](EXTENSION.md), [ERROR_HANDLING.md](ERROR_HANDLING.md), [OBS_READ_API.md](OBS_READ_API.md), [OBS_STORE_MMAP_FORMAT.md](OBS_STORE_MMAP_FORMAT.md), `proto/rex/v1/rex.proto`, [DOCUMENTATION.md](DOCUMENTATION.md) |

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
| [ROADMAP.md](ROADMAP.md) | v1.0 closure queue (**RC-LF1** Must), LangFuse discovery, engineering backlog IDs |
| [ARCHITECTURE.md](ARCHITECTURE.md) | SAD + C4-style views + links to ADRs |
| [ARCHITECTURE_GUIDELINES.md](ARCHITECTURE_GUIDELINES.md) | Architecture policies, ownership, documentation layering |
| [AGENT_RUNTIME_ENVIRONMENT.md](AGENT_RUNTIME_ENVIRONMENT.md) | Deferred VM/container transport catalog |
| [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md) | Process sidecar: spawn, `rex.sidecar.v1`, UDS gRPC |
| [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md) | Product agent program (partial — shipped): `rex-agent`; stub CI default |
| [NATIVE_TOOL_CALLING.md](NATIVE_TOOL_CALLING.md) | Native broker tool calling — **design hub** (**R038** **Done**) |
| [AGENT_GRAPH_ARCHITECTURE.md](AGENT_GRAPH_ARCHITECTURE.md) | Token-efficient sidecar graph — **design hub** (**R027–R038**) |
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
| [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md) | LangFuse Cloud observability — **design hub** |
| [LANGFUSE_DISCOVERY_ROADMAP.md](LANGFUSE_DISCOVERY_ROADMAP.md) | LangFuse discovery tasks (**LF-D01–LF-D10**) |
| [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md) | **Superseded** — signal catalog (historical) |
| [CHCE_ROADMAP.md](CHCE_ROADMAP.md) | **Cancelled** — CHCE program (**R043–R054**) |
| [OBS_STORE_MMAP_FORMAT.md](OBS_STORE_MMAP_FORMAT.md) | **Cancelled** — CHCE format reference |
| [ECONOMICS_VALIDATION.md](ECONOMICS_VALIDATION.md) | Economics validation program — **design hub** |
| [OBS_READ_API.md](OBS_READ_API.md) | **Cancelled** — loopback read API (historical) |
| [OBSERVABILITY_INTEGRATIONS.md](OBSERVABILITY_INTEGRATIONS.md) | **Cancelled** — bundled Grafana suite (historical) |
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
| Operation feedback (live stream) | [OPERATION_FEEDBACK.md](OPERATION_FEEDBACK.md), [ADR 0028](architecture/decisions/0028-incremental-run-turn-streaming.md), [ADR 0029](architecture/decisions/0029-ask-mode-research-broker.md) |
| Long-term / project memory | [LONG_TERM_MEMORY.md](LONG_TERM_MEMORY.md) |
| Agent knowledge for AI sessions (planned) | [AGENT_KNOWLEDGE.md](AGENT_KNOWLEDGE.md) |
| LangFuse Cloud observability (primary) | [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md), [LANGFUSE_DISCOVERY_ROADMAP.md](LANGFUSE_DISCOVERY_ROADMAP.md), [CONFIGURATION.md](CONFIGURATION.md#observability), [ADR 0010](architecture/decisions/0010-daemon-exports-observability-via-otel-and-sidecar-api.md), [ADR 0020](architecture/decisions/0020-otel-genai-semconv-with-rex-pipeline-metrics.md) |
| Rex-owned observability suite (**cancelled**) | [OBSERVABILITY_AND_ECONOMICS.md](OBSERVABILITY_AND_ECONOMICS.md), [CHCE_ROADMAP.md](CHCE_ROADMAP.md), [OBS_READ_API.md](OBS_READ_API.md), [OBS_STORE_MMAP_FORMAT.md](OBS_STORE_MMAP_FORMAT.md), [OBSERVABILITY_INTEGRATIONS.md](OBSERVABILITY_INTEGRATIONS.md) |
| Native broker tool calling (**R038** **Done**) | [NATIVE_TOOL_CALLING.md](NATIVE_TOOL_CALLING.md), [ADR 0023](architecture/decisions/0023-hybrid-agent-serialization-boundaries.md) |
| Economics validation + live LLM smoke (**R039–R042** planned) | [ECONOMICS_VALIDATION.md](ECONOMICS_VALIDATION.md) |
| MCP-style tool interoperability (design accepted) | [ADR 0016](architecture/decisions/0016-mcp-in-sidecar-envelope.md), [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md), [ADR 0008](architecture/decisions/0008-dedicated-sidecar-control-plane-api.md) |
| Purpose and principles | [PURPOSE_AND_PRINCIPLES.md](PURPOSE_AND_PRINCIPLES.md) |
| Agent access + sidecar runtime | [AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md), [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md), [CAPABILITY_SIDECARS.md](CAPABILITY_SIDECARS.md), [POLICY_ENGINE.md](POLICY_ENGINE.md), [ADR 0005](architecture/decisions/0005-rex-owns-sidecar-environment-not-agent-implementations.md), [ADR 0008](architecture/decisions/0008-dedicated-sidecar-control-plane-api.md) |
| Web search via capability sidecar (**R055** planned) | [WEB_SEARCH.md](WEB_SEARCH.md), [ADR 0029](architecture/decisions/0029-web-search-via-capability-sidecar.md) |
| Product agent delivery (Should complete) | [AGENT_DELIVERY_ROADMAP.md](AGENT_DELIVERY_ROADMAP.md), [ROADMAP.md](ROADMAP.md) R013–R038 Done |
| Token-efficient agent graph (design accepted) | [AGENT_GRAPH_ARCHITECTURE.md](AGENT_GRAPH_ARCHITECTURE.md), [NATIVE_TOOL_CALLING.md](NATIVE_TOOL_CALLING.md), [ADR 0022](architecture/decisions/0022-viewer-editor-subagent-topology.md), [ADR 0023](architecture/decisions/0023-hybrid-agent-serialization-boundaries.md), [ROADMAP.md](ROADMAP.md) R027–R038 |
| CI quality and security gates (**R023** Done) | [CI_QUALITY_GATES.md](CI_QUALITY_GATES.md), [ROADMAP.md](ROADMAP.md) R024–R026 |
| Development assistance capabilities (daemon-owned context, turn contract) | [DEVELOPMENT_ASSISTANCE_CAPABILITIES.md](DEVELOPMENT_ASSISTANCE_CAPABILITIES.md), [ADRs 0011–0017](architecture/decisions/README.md) |
| Plan mode planning tools | [PLANNING_TOOLS.md](PLANNING_TOOLS.md), [ADR 0024](architecture/decisions/0024-plan-mode-artifacts-and-plan-save-broker.md) |

## Scripts (related)

| Path | Purpose |
|------|---------|
| [../scripts/reinstall-dev.sh](../scripts/reinstall-dev.sh) | Fast reinstall: `rex` + sidecar stub on PATH + VS Code/Cursor extension |
| [../scripts/install-cli.sh](../scripts/install-cli.sh) | Install unified `rex` binary to `~/.cargo/bin` |
| [../scripts/install-extension.sh](../scripts/install-extension.sh) | Local VSIX build + install |
| [../scripts/dev-rex-extension.sh](../scripts/dev-rex-extension.sh) | Wrapper around `reinstall-dev.sh` |
| [../scripts/verify_mvp_local.sh](../scripts/verify_mvp_local.sh) | MVP preflight (build, rust verify, sidecar verify, product path) |

## Source vs generated artifacts

- Canonical text lives in **`docs/`** and source trees; **`target/`**, scratch logs, disposable VSIX artifacts stay out of Git unless a release flow dictates otherwise (`TEMP_*` coordination files remain gitignored).
