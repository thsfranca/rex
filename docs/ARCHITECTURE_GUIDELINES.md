# Architecture guidelines

This document defines **cross-cutting architectural policies** for Rex: **who owns what**, how docs relate to code, and how to keep documentation **correct and maintainable**. It does **not** replace [ARCHITECTURE.md](ARCHITECTURE.md) (system description and views) or feature hubs such as [ADAPTERS.md](ADAPTERS.md).

## Purpose and audience

| Audience | Use this doc to |
|----------|------------------|
| Contributors | Align changes with **daemon authority**, **bounded contexts**, and **policy vs mechanism** before opening a PR. |
| Reviewers | Check that contracts and ownership match accepted ADRs and this policy hub. |
| Automation / assistants | Resolve conflicts between docs using **Layering of truth** below; cite ADRs for boundaries. |

## Layering of truth

When documents disagree, resolve in this **order**:

| Priority | Source | Role |
|----------|--------|------|
| 1 | [PURPOSE_AND_PRINCIPLES.md](PURPOSE_AND_PRINCIPLES.md) | Project **intent** and principles. |
| 2 | **This file** (`ARCHITECTURE_GUIDELINES.md`) | **Stable policies** and ownership expectations that span features. |
| 3 | Accepted **ADRs** under [architecture/decisions/](architecture/decisions/) | Recorded **decisions** with status and consequences. |
| 4 | Feature hubs ([ADAPTERS.md](ADAPTERS.md), [CACHING.md](CACHING.md), [EXTENSION.md](EXTENSION.md), …) | **Reference** behavior and contracts per area. |
| 5 | Code | **Shipped** behavior; docs **must** catch up when contracts change. |

**Conflict rules:**

- **Purpose / principles** beat informal roadmap wording when intent conflicts—update the roadmap or purpose deliberately, do not hide drift.
- **Accepted ADR** beats stale prose in guidelines or overview docs—fix the prose or supersede the ADR with a new one (see [architecture/decisions/README.md](architecture/decisions/README.md)).
- **More specific** hub beats generic overview for that feature area (same rule as [ROADMAP.md](ROADMAP.md)).

## Core policies (Rex)

These restate accepted boundaries; **authoritative detail** remains in linked ADRs and hubs.

### Daemon authority

**`rex-daemon`** owns **streaming contract semantics**, **economics-oriented policy** (modes, caches, pipelines, adapter envelope), and **terminal correctness** for `rex.v1` clients. See [ADR 0001](architecture/decisions/0001-daemon-owns-agent-orchestration-and-economics.md), [ARCHITECTURE.md](ARCHITECTURE.md).

### Policy vs mechanism

- **Policy** (what must hold): stream endings, cache eligibility rules, mode normalization intent, quotas as specified—**owned by the daemon core** and documented in hubs ([CACHING.md](CACHING.md), [MVP_SPEC.md](MVP_SPEC.md)).
- **Mechanism** (how it is done): LRU layout, subprocess spawning, future remote cache or sidecar RPC—**behind explicit seams** (today: modules and env-selected runtimes; tomorrow: narrow ports as implementations multiply).

Guidelines doc **does not** invent new runtime flags or APIs—only **documents direction** already reflected in ADRs and roadmaps.

### Bounded contexts

| Context | Boundary | Integrate by |
|---------|-----------|--------------|
| **`rex.v1` client plane** | Thin stable API for CLI, editors, scripts over UDS. | Protobuf + [MVP_SPEC.md](MVP_SPEC.md); NDJSON via `rex-cli` per [EXTENSION.md](EXTENSION.md). |
| **Sidecar control plane** | Brokered integration for isolated runtimes—**not** a widening of `rex.v1` into a generic tunnel. | Dedicated API per [ADR 0008](architecture/decisions/0008-dedicated-sidecar-control-plane-api.md); map types at the boundary. |

Do **not** leak raw **`rex.v1`** messages into sidecar-specific logic without translation; do **not** hide host reachability inside guests without daemon policy.

### Environment ownership vs agent implementations

**Rex** supervises **environment and contract** for optional isolated workloads; it does **not** mandate a single third-party agent codebase. See [ADR 0005](architecture/decisions/0005-rex-owns-sidecar-environment-not-agent-implementations.md).

## Ownership matrix

Canonical **decision** owner means “writes the norm in the referenced doc / ADR”; **implementation** may lag—mark **TBD** honestly.

| Policy area | Owns decision | Canonical doc(s) | ADR(s) |
|-------------|---------------|------------------|--------|
| Stream semantics and terminal outcomes | `rex-daemon` | [MVP_SPEC.md](MVP_SPEC.md), [ARCHITECTURE.md](ARCHITECTURE.md) | [0001](architecture/decisions/0001-daemon-owns-agent-orchestration-and-economics.md) |
| Layered cache / mode and `ask` vs `agent` | `rex-daemon` policy | [CACHING.md](CACHING.md) | [0003](architecture/decisions/0003-layered-cache-agent-mode-policy.md) |
| Inference adapter contract | `rex-daemon` + adapters | [ADAPTERS.md](ADAPTERS.md) | [0002](architecture/decisions/0002-inference-adapter-contract.md) |
| Routing / optional HTTP gateway stance | Product architecture | [ADR 0004](architecture/decisions/0004-routing-daemon-first-optional-http-gateway.md), [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) | 0004 |
| Sidecar ↔ daemon integration | Product architecture | [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md), [AGENT_RUNTIME_ENVIRONMENT.md](AGENT_RUNTIME_ENVIRONMENT.md) | [0005](architecture/decisions/0005-rex-owns-sidecar-environment-not-agent-implementations.md), [0008](architecture/decisions/0008-dedicated-sidecar-control-plane-api.md) |
| Extension transport (NDJSON + optional unary gRPC) | Extension + contract docs | [EXTENSION.md](EXTENSION.md), [EXTENSION_ROADMAP.md](EXTENSION_ROADMAP.md) | [0007](architecture/decisions/0007-editor-extension-hybrid-transport-cli-and-grpc.md) |
| Policy vs mechanism seams in code (ports, resolution-before-semantic-cache) | **TBD** — align implementation with this hub | [ROADMAP.md](ROADMAP.md) engineering backlog **R007** | See R007 note |
| Agent execution approvals / checkpoints | `rex-daemon` policy (UX in extension) | [EXTENSION.md](EXTENSION.md), [ROADMAP.md](ROADMAP.md) backlog **R008** | [0009](architecture/decisions/0009-centralized-agent-approvals-and-checkpoints.md) |

## When to write or update an ADR vs this file

| Situation | Action |
|-----------|--------|
| **Yes/no** with trade-offs and consequences (supersede prior choice) | New or updated **ADR**; link from here only if the guideline summary changes. |
| **Long-lived rule** that spans several ADRs (ownership, layering, documentation discipline) | Update **this file**; keep ADRs for specific forks. |
| **Shipped behavior** change | Update **code**, then **feature hub** and **MVP_SPEC** / CI expectations as needed; ADR if the **decision** is architectural. |

## Documentation quality expectations

Align with [DOCUMENTATION.md](DOCUMENTATION.md) and arc42-style documentation goals:

| Expectation | Rex practice |
|-------------|--------------|
| Correct | Diagrams and tables match **code and protos**; wrong docs are worse than missing ([DOCUMENTATION.md](DOCUMENTATION.md)). |
| Current | Update docs in the **same change** as behavior when feasible; use **Scope note** in [ROADMAP.md](ROADMAP.md) for deliberate lag. |
| Understandable | Active voice, bullets/tables, **one dominant topic per file**. |
| Referenceable | **Stable headings** for scanning and retrieval; link to hubs instead of copying long inventories. |
| Maintainable | **Hub-and-spoke**: one canonical feature doc per area; ADRs for formal acceptance. |

**When to update this guidelines file**

- Ownership or **bounded context** rules change.
- A new **class** of integration (for example sidecar API v2) needs a **policy** statement that is not yet an ADR.

## Related links

- [ARCHITECTURE.md](ARCHITECTURE.md) — structure and runtime views.
- [DEVELOPER_EXPERIENCE_GUIDE.md](DEVELOPER_EXPERIENCE_GUIDE.md) — contribution workflow and contracts.
- [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md) — daemon-first extensibility and sidecar phasing.
- [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) — economics lever matrix.
- [DOCUMENTATION.md](DOCUMENTATION.md) — writing norms and doc-type portfolio.

## See also

- [docs/README.md](README.md) — Diátaxis index.
- [architecture/decisions/README.md](architecture/decisions/README.md) — ADR index.
