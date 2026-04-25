# Refactor Proposals
<!-- One line per item. Keep IDs stable. -->
<!-- Status: NEW | READY | DOING | DONE | DROPPED | BLOCKED -->

- [R001] T:DaemonStreamFlowRefactor | S:DONE | P:Validate local stream lifecycle MVP | N:Y | V:5 | C:3 | D:-> | CM:1.0 | E:Deterministic terminal states plus runtime seam | Tr:-> | Pr:PR1
- [R002] T:CiTriggerAndGateSimplification | S:DONE | P:Protect main with lean CI gates | N:Y | V:4 | C:2 | D:-> | CM:1.0 | E:Duplicate push and PR runs | Tr:-> | Pr:PR1
- [R003] T:CiObservabilitySignalStandard | S:DONE | P:Speed CI failure triage | N:N | V:5 | C:2 | D:R002 | CM:1.3 | E:Generic gate output slowed triage | Tr:Ambiguous failures >3/week | Pr:PR1
- [R004] T:CliExtensionSeamHardening | S:READY | P:Stabilize rex-cli contract for editor extension consumers | N:Y | V:4 | C:3 | D:R001 | CM:1.2 | E:Post-ship extension needs deterministic cancel and error taxonomy | Tr:-> | Pr:PR2
- [R005] T:CrossBoundaryContractConformance | S:NEW | P:Protect extension and CLI NDJSON contract stability | N:Y | V:5 | C:3 | D:R004 | CM:1.3 | E:Need integration guard for one-terminal-event contract | Tr:-> | Pr:PR1
- [R006] T:BroadCliRuntimeRefactor | S:DROPPED | P:Keep delivery focused on reliability and observability outcomes | N:N | V:2 | C:5 | D:R004,R005 | CM:1.6 | E:No architecture mismatch evidence at current scale | Tr:Reopen if repeated >3 module edits in 2 PRs | Pr:->
- [R007] T:ModeOrchestratorBoundary | S:NEW | P:Ship ask-plan-agent MVP with deterministic mode policy | N:Y | V:5 | C:4 | D:R005 | CM:1.3 | E:No unified mode policy layer today | Tr:-> | Pr:PR1
- [R008] T:ApprovalGateExecutionFlow | S:NEW | P:Guard agent execution with explicit approvals and checkpoints | N:Y | V:5 | C:4 | D:R007 | CM:1.3 | E:Execution safety needs centralized gate | Tr:-> | Pr:PR2
