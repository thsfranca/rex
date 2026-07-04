# KISS config surface

Guardrails for Rex configuration and runtime binding when designing or reviewing changes.

## Rules

- Prefer **process context** (cwd, spawn directory, CLI flags) over new `workspace.*` or `REX_*` keys unless a simpler path cannot work.
- One default **product** path; no parallel **harness-only** flags for the same behavior without an ADR delta and an explicit KISS challenger note (simpler option first, why insufficient).
- Before adding a config key: state the simpler option, why it fails, and what measured risk the key mitigates.
- When removing keys, update [CONFIGURATION.md](CONFIGURATION.md) removed-keys table and [ERROR_HANDLING.md](ERROR_HANDLING.md) if operator messages change.

## Example (resolved)

**Before:** `workspace.root` required for product; `workspace.allow_cwd_fallback` for harness — dual modes for the same binding.

**After:** workspace = canonical **cwd** only ([ADR 0011](architecture/decisions/0011-workspace-binding-and-turn-context-authority.md), PR #375).

## Related

- [CONFIGURATION.md](CONFIGURATION.md) — JSON schema and removed keys
- [DEVELOPER_EXPERIENCE_GUIDE.md](DEVELOPER_EXPERIENCE_GUIDE.md) — AI review checklist
