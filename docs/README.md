# Documentation index

Documentation follows **[Diátaxis](https://diataxis.fr/)** roles; the **architecture description + ADRs** live in [ARCHITECTURE.md](ARCHITECTURE.md) and [architecture/decisions/](architecture/decisions/).

| Role | Where to read |
|------|----------------|
| **Tutorials** (first success path) | [README.md](../README.md) quickstart, [EXTENSION_LOCAL_E2E.md](EXTENSION_LOCAL_E2E.md) |
| **How-to** (task recipes) | [EXTENSION_RELEASE.md](EXTENSION_RELEASE.md), [CONFIGURATION.md](CONFIGURATION.md), [CI.md](CI.md), [DEPENDENCIES.md](DEPENDENCIES.md) |
| **Explanation** (design intent) | [PURPOSE_AND_PRINCIPLES.md](PURPOSE_AND_PRINCIPLES.md), [ARCHITECTURE.md](ARCHITECTURE.md), [AGENT_RUNTIME_ENVIRONMENT.md](AGENT_RUNTIME_ENVIRONMENT.md), [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md), [LONG_TERM_MEMORY.md](LONG_TERM_MEMORY.md), [MVP_SPEC.md](MVP_SPEC.md), [EXTENSION_ROADMAP.md](EXTENSION_ROADMAP.md) |
| **Reference** (narrow facts) | [ADAPTERS.md](ADAPTERS.md), [CACHING.md](CACHING.md), [EXTENSION.md](EXTENSION.md), `proto/rex/v1/rex.proto`, [DOCUMENTATION.md](DOCUMENTATION.md) |

## Repository map (`docs/`)

| Path | Purpose |
|------|---------|
| [PURPOSE_AND_PRINCIPLES.md](PURPOSE_AND_PRINCIPLES.md) | Project purpose and operating principles (canonical) |
| [DOCUMENTATION.md](DOCUMENTATION.md) | Writing checklist + doc-type norms |
| [DEVELOPER_EXPERIENCE_GUIDE.md](DEVELOPER_EXPERIENCE_GUIDE.md) | How to contribute and gate quality |
| [PRIORITIZATION.md](PRIORITIZATION.md) | MoSCoW + quick R-ICE |
| [ROADMAP.md](ROADMAP.md) | Now / Next / Later + engineering backlog IDs |
| [ARCHITECTURE.md](ARCHITECTURE.md) | SAD + C4-style views + links to ADRs |
| [AGENT_RUNTIME_ENVIRONMENT.md](AGENT_RUNTIME_ENVIRONMENT.md) | Isolated agent runtimes + host–guest transport concepts |
| [architecture/decisions/](architecture/decisions/) | ADR index + template |
| [MVP_SPEC.md](MVP_SPEC.md) | Phase 1 protocol & acceptance |
| [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md) | Daemon-first extensibility + optional sidecars |
| [ADAPTERS.md](ADAPTERS.md) | `InferenceRuntime` + capabilities |
| [CACHING.md](CACHING.md) | L1/L2/prefix policy |
| [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) | Pipeline + **economics lever matrix** |
| [LONG_TERM_MEMORY.md](LONG_TERM_MEMORY.md) | Long-term / project memory — **design hub** |
| [CONFIGURATION.md](CONFIGURATION.md) | `REX_*` catalog |
| [EXTENSION.md](EXTENSION.md) | Extension NDJSON contract + component layout |
| [EXTENSION_ROADMAP.md](EXTENSION_ROADMAP.md) | Extension phasing |
| [EXTENSION_RELEASE.md](EXTENSION_RELEASE.md) | Install & release |
| [EXTENSION_LOCAL_E2E.md](EXTENSION_LOCAL_E2E.md) | Clone → working editor |
| [CI.md](CI.md) | Automation gates |
| [DEPENDENCIES.md](DEPENDENCIES.md) | Toolchain prerequisites |

## Feature-area hub map

Use **one** canonical doc per row for feature **shape**; roadmaps link here—see [DOCUMENTATION.md](DOCUMENTATION.md).

| Feature area | Canonical hub |
|---|---|
| Inference adapters | [ADAPTERS.md](ADAPTERS.md), [ADR 0002](architecture/decisions/0002-inference-adapter-contract.md) |
| Daemon extensibility phases (sidecars) | [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md) |
| Layered cache policy | [CACHING.md](CACHING.md), [ADR 0003](architecture/decisions/0003-layered-cache-agent-mode-policy.md) |
| Routing / optional gateway stance | [ADR 0004](architecture/decisions/0004-routing-daemon-first-optional-http-gateway.md), economics rows in [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) |
| Extension NDJSON consumer contract | [EXTENSION.md](EXTENSION.md) |
| Extension UX phasing | [EXTENSION_ROADMAP.md](EXTENSION_ROADMAP.md) |
| Long-term / project memory | [LONG_TERM_MEMORY.md](LONG_TERM_MEMORY.md) |
| MCP-style tool interoperability (planned) | [ARCHITECTURE.md](ARCHITECTURE.md) interoperability rows, matrix in [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) |
| Purpose and principles | [PURPOSE_AND_PRINCIPLES.md](PURPOSE_AND_PRINCIPLES.md) |
| Isolated agent runtime (concepts) | [AGENT_RUNTIME_ENVIRONMENT.md](AGENT_RUNTIME_ENVIRONMENT.md), [ADR 0005](architecture/decisions/0005-rex-owns-sidecar-environment-not-agent-implementations.md) |

## Scripts (related)

| Path | Purpose |
|------|---------|
| [../scripts/install-extension.sh](../scripts/install-extension.sh) | Local VSIX build + install |
| [../scripts/dev-rex-extension.sh](../scripts/dev-rex-extension.sh) | Rust build + installer wrapper |

## Source vs generated artifacts

- Canonical text lives in **`docs/`** and source trees; **`target/`**, scratch logs, disposable VSIX artifacts stay out of Git unless a release flow dictates otherwise (`TEMP_*` coordination files remain gitignored).
