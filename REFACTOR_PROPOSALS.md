# Refactor Proposals
<!-- One line per item. Keep IDs stable. -->
<!-- Status: NEW | READY | DOING | DONE | DROPPED | BLOCKED -->

- [R001] T:DaemonStreamFlowRefactor | S:DONE | P:Validate local stream lifecycle MVP | N:Y | V:5 | C:3 | D:-> | CM:1.0 | E:Deterministic terminal states plus runtime seam | Tr:-> | Pr:PR1
- [R002] T:CiTriggerAndGateSimplification | S:DONE | P:Protect main with lean CI gates | N:Y | V:4 | C:2 | D:-> | CM:1.0 | E:Duplicate push and PR runs | Tr:-> | Pr:PR1
- [R003] T:CiObservabilitySignalStandard | S:DONE | P:Speed CI failure triage | N:N | V:5 | C:2 | D:R002 | CM:1.3 | E:Generic gate output slowed triage | Tr:Ambiguous failures >3/week | Pr:PR1
- [R004] T:CliExtensionSeamHardening | S:NEW | P:Stabilize rex-cli contract for editor extension consumers | N:N | V:3 | C:3 | D:R001 | CM:1.0 | E:Extension host builds prompts client-side and relies on complete --format ndjson + status; no CLI change required for PR 2/PR 3, but cancellation handshake and structured error taxonomy may be needed once inline completions or retries land | Tr:Two consecutive extension PRs needing CLI changes to land a single UX outcome | Pr:EXT_PR_FOLLOWUP
