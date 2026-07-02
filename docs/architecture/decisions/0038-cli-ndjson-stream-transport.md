# ADR 0038: CLI NDJSON stream transport

**Status:** Accepted  
**Date:** 2026-07-01  
**Supersedes:** [0007-editor-extension-hybrid-transport-cli-and-grpc.md](0007-editor-extension-hybrid-transport-cli-and-grpc.md)

## Context

Rex removed the in-repo VS Code/Cursor extension. Thin clients are now **CLI and automation only**. Streaming completion still uses **`rex complete --format ndjson`** as the primary consumer contract ([NDJSON_STREAM.md](../../NDJSON_STREAM.md)).

## Decision

- **Primary transport:** subprocess NDJSON from the unified **`rex`** binary.
- **Optional unary gRPC:** remains available on **`rex.v1`** over UDS for status and future CLI-native calls — not Node in the monorepo.
- **Contract hub:** [NDJSON_STREAM.md](../../NDJSON_STREAM.md) replaces extension-specific docs.

## Consequences

- No VSIX release plane or npm toolchain in the monorepo.
- Error code sync validates YAML ↔ `rex-cli` runtime ↔ docs (not TypeScript).
- ADR 0007 remains for historical context; new work references this ADR and NDJSON_STREAM.

## Related

- [NDJSON_STREAM.md](../../NDJSON_STREAM.md)
- [CLI_OPERATOR_UX.md](../../CLI_OPERATOR_UX.md)
- [ADR 0007](0007-editor-extension-hybrid-transport-cli-and-grpc.md) (superseded)
