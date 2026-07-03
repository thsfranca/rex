# Refactor Proposals
<!-- One line per item. Keep IDs stable. -->
<!-- Status: NEW | READY | DOING | DONE | DROPPED | BLOCKED -->

- [R082a] T:Docs JSON-only one env REX_ROOT | S:DOING | P:R082 | N:Y | V:5 | C:1 | Tr:chore/r082-config-docs-one-env | E:CONFIGURATION + hubs; no legacy env catalogs
- [R082b] T:Remove dead agent step/soft-cap keys | S:READY | P:R082 | N:Y | V:4 | C:2 | D:R082a | Tr:chore/r082-remove-dead-agent-caps | E:ADR 0034 follow-up; schema + agent
- [R082c] T:Drop REX_ALLOW_CWD_WORKSPACE + schema sync | S:READY | P:R082 | N:Y | V:4 | C:2 | D:R082b | Tr:chore/r082-schema-sync | E:JSON-only harness; promote agent-only keys
- [R082d] T:Slim operator_init_template | S:NEW | P:R082 | N:N | V:2 | C:1 | D:R082c | Tr:chore/r082-operator-template | E:agent-only list; harness appendix
- [R004] T: turn_correlation module | S:DONE | P:R021 | N:Y | V:3 | C:1 | Tr:feat/r021-turn-correlation | E:isolate hash/strip from service.rs
- [R003] T:Supervisor PATH binary resolve | S:DONE | P:R017 agent | N:Y | V:4 | C:1 | D:- | CM:1.0 | E:doctor passes spawn fails bare name | Tr:-> | Pr:R017
- [R002] T:Consolidate CI gate jobs | S:DONE | P:CI | N:N | V:3 | C:2 | D:- | CM:0.5 | E:removed rust-checks/extension-checks domain gates; ci-checks reads verify results directly | Tr:chore/ci-simplify-gates | Pr:-
- [R001] T:Merge rex-cli/rex-daemon crates | S:DROPPED | P:R015 config | N:N | V:2 | C:4 | D:R014 | CM:1.2 | E:two crates one binary | Tr:R015 config landed; merge deferred — library split retained for reviewability | Pr:follow-up if needed
