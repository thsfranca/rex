# ADR 0005: Rex owns sidecar environment, not third-party agent implementations

- **Date:** 2026-05-02
- **Status:** Accepted

## Context

Rex positions **`rex-daemon` as the economics and stream authority** for clients ([ADR 0001](0001-daemon-owns-agent-orchestration-and-economics.md)). The project also explores **isolated runtimes** for future agent-style workloads (process or VM boundary) so LLM-driven code does not gain **ambient** host access.

Wording like “Rex owns the agent” is easy to misread as “Rex ships and maintains a single agent codebase.” The actual intent is: Rex **supervises the environment and contract** in which an agent runs; **alternative agent stacks** (for example a third-party graph runtime) should remain **pluggable** as long as they respect the **daemon boundary**.

Project **purpose and principles** are canonical in [../../PURPOSE_AND_PRINCIPLES.md](../../PURPOSE_AND_PRINCIPLES.md), including **incremental delivery** and **sidecars when justified** ([../../PLUGIN_ROADMAP.md](../../PLUGIN_ROADMAP.md)).

## Decision

1. **`rex-daemon` owns the isolated sidecar / agent-runtime environment** in the product sense: **lifecycle hooks**, **resource envelope** (as defined when implemented), **authorized channel** to the daemon, and **policy alignment** with stream and economics rules.

2. **Rex does not own third-party or alternate “agent product” source trees.** Implementations are **brought** into the environment; Rex defines **how** they may reach the host (for example **only** through the daemon API), not **which** Python framework must be used.

3. The **“sidecar”** concept may evolve from “optional gRPC plugin process” to **“managed environment”** (for example OCI or VM template) without changing [ADR 0001](0001-daemon-owns-agent-orchestration-and-economics.md): economics and **StreamInference** contract stay in the daemon.

## Consequences

- **Positive:** Clearer story for **swappable agents** and **environment supervision**; less confusion with “Rex is one LangGraph app.”
- **Negative:** More docs to keep aligned ([../../AGENT_RUNTIME_ENVIRONMENT.md](../../AGENT_RUNTIME_ENVIRONMENT.md), purpose doc).
- **Risks / follow-up:** Any **supervisor** or transport choice must still honor **incremental** shipping and not imply features are live before code exists.

## Related

- [ADR 0001](0001-daemon-owns-agent-orchestration-and-economics.md) · [../../AGENT_RUNTIME_ENVIRONMENT.md](../../AGENT_RUNTIME_ENVIRONMENT.md) · [../../PURPOSE_AND_PRINCIPLES.md](../../PURPOSE_AND_PRINCIPLES.md)
