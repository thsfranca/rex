# ADR 0007: CLI client hybrid transport (`rex-cli` NDJSON primary, optional unary `rex.v1`)

**Implementation note (2026):** the unified **`rex`** binary fulfills the NDJSON subprocess contract described here (`rex complete --format ndjson`); **`rex-cli`** remains a compatibility shim.

- **Date:** 2026-05-05
- **Status:** Superseded by [0038-cli-ndjson-stream-transport.md](0038-cli-ndjson-stream-transport.md) (2026-07-01). Historical: editor extension was the original thin client; contract now applies to CLI/scripts only.

## Context

The rex CLI must stay **thin** (UX and policy presentation) while **`rex-daemon`** owns inference, streaming authority, and economics ([ADR 0001](0001-daemon-owns-agent-orchestration-and-economics.md)). Transport choices:

- **A — `rex-cli` only:** subprocess + **`--format ndjson`** for all daemon interaction — portable, easy to test, no Node gRPC stack.
- **B — Node gRPC only:** extension speaks **`rex.v1`** directly over the same UDS as other clients — one wire protocol, but heavier dependency and runtime matrix (VS Code vs Cursor).
- **C — Hybrid:** keep **`rex-cli` NDJSON** as the **primary** path for **streaming completion**; allow **optional** direct **`rex.v1`** use from the extension for **unary** RPCs when maintainers choose (for example status / health), without replacing the NDJSON streaming contract by default.

This ADR accepts **Option C**. It aligns with **isolated sidecar + dedicated sidecar API** direction ([ADR 0005](0005-rex-owns-sidecar-environment-not-agent-implementations.md), [ADR 0008](0008-dedicated-sidecar-control-plane-api.md)): the **editor path** stays on **`rex.v1`**-family surfaces only; **sidecar integration** uses the **separate** sidecar ↔ daemon API — not the extension transport choice.

**MCP / tool interoperability placement** (brokered sidecar, MCP primarily in the guest envelope) is **recorded as target architecture elsewhere** and **implementation-deferred** — not gated by this ADR.

## Decision

1. **Streaming completion in the extension** uses **`rex-cli complete … --format ndjson`** as the **canonical** path. The stable NDJSON consumer contract remains **[NDJSON_STREAM.md](../../NDJSON_STREAM.md)**.

2. **Optional unary `rex.v1` over UDS** from Node (**`grpc-js`** + generated stubs) is **allowed** for calls that map cleanly to **existing `rex.v1` unary** RPCs (for example **`GetStatus`**), when maintainers accept the Node gRPC dependency cost for that slice.

3. **Do not** implement **`StreamInference`** from the extension via Node gRPC **instead of** the NDJSON **`rex-cli`** path **without** a **new ADR** — that would change reliability, cancellation, and testing assumptions.

4. **Do not** add **editor-only** or **extension-specific** RPCs to **`rex.v1`**; editor remains a thin client on the **shared** contract ([NDJSON_STREAM.md](../../NDJSON_STREAM.md) non-goals).

5. **`rex-cli`** remains the **supported façade** for scripting, CI, and operators who do not embed gRPC clients.

## Consequences

- **Positive:** Portable default path (NDJSON); optional unary gRPC where low latency / typed status matters; one **`rex.v1`** contract for daemon semantics; avoids forcing the full Node streaming stack for chat by default.
- **Negative:** Two integration paths to maintain (CLI subprocess + optional gRPC); discipline required so unary gRPC does not silently duplicate or drift from CLI behavior.
- **Risks / follow-up:** Any expansion of Node-side **`rex.v1`** usage should stay narrow and tested on **both** VS Code and Cursor hosts; MCP-heavy tooling stays **out of scope** for the extension until the deferred brokered-sidecar story ships ([ADR 0008](0008-dedicated-sidecar-control-plane-api.md), [CONTEXT_EFFICIENCY.md](../../CONTEXT_EFFICIENCY.md) matrix when MCP work is scheduled).

## Related

- [NDJSON_STREAM.md](../../NDJSON_STREAM.md) · [ROADMAP.md](../../ROADMAP.md) · [../ARCHITECTURE.md](../ARCHITECTURE.md)
- [ADR 0001](0001-daemon-owns-agent-orchestration-and-economics.md) · [ADR 0005](0005-rex-owns-sidecar-environment-not-agent-implementations.md) · [ADR 0008](0008-dedicated-sidecar-control-plane-api.md)
- [README.md](README.md) (index)
