# Refactor Proposals
<!-- One line per item. Keep IDs stable. -->
<!-- Status: NEW | READY | DOING | DONE | DROPPED | BLOCKED -->

- [R003] T:Supervisor PATH binary resolve | S:DONE | P:R017 agent | N:Y | V:4 | C:1 | D:- | CM:1.0 | E:doctor passes spawn fails bare name | Tr:-> | Pr:R017
- [R002] T:Consolidate CI gate jobs | S:DONE | P:CI | N:N | V:3 | C:2 | D:- | CM:0.5 | E:removed rust-checks/extension-checks domain gates; ci-checks reads verify results directly | Tr:chore/ci-simplify-gates | Pr:-
- [R001] T:Merge rex-cli/rex-daemon crates | S:DROPPED | P:R015 config | N:N | V:2 | C:4 | D:R014 | CM:1.2 | E:two crates one binary | Tr:R015 config landed; merge deferred — library split retained for reviewability | Pr:follow-up if needed
