# Documentation Standards (Project)

This project keeps documentation standards in this file and in the `documentation-content` project rule, without referencing editor-specific home directories.

## Source of truth

- **[docs/DOCUMENTATION.md](DOCUMENTATION.md)** (this file) is the human-readable **writing and structure** contract.
- **[docs/PURPOSE_AND_PRINCIPLES.md](PURPOSE_AND_PRINCIPLES.md)** is the canonical **project purpose and operating principles**. Documentation and plans **align** with it when it exists.
- The **`documentation-content`** project rule (scoped to `docs/**/*.md`; in this repo: `.cursor/rules/documentation-content.mdc`) repeats the same expectations for agents. If something conflicts about norms or hubs, **this file wins**; if something conflicts about **purpose or principles**, **`docs/PURPOSE_AND_PRINCIPLES.md` wins**.

### Feature-area hubs

- Assign **one** canonical explanation document in **`docs/`** per **major product feature** (definitions, optional design bets, and interface intent belong there).
- **ADRs** under `docs/architecture/decisions/` hold **accepted decisions** when you record them formally; linking text elsewhere stays thin.
- **Other** guides, roadmap rows, and the architecture overview **link** to the hub instead of copying full bullet inventories; point to MoSCoW or status in one line when useful.
- **Example hub:** long-term memory — [LONG_TERM_MEMORY.md](LONG_TERM_MEMORY.md).

## Document types (portfolio)

| Type | Canonical location |
|------|-------------------|
| **Diátaxis / role routing** | [docs/README.md](README.md) |
| Software architecture (**SAD**-style consolidation) | [ARCHITECTURE.md](ARCHITECTURE.md) |
| **Architecture Decision Records** | [architecture/decisions/](architecture/decisions/) — supersede stale decisions by status + successor ADR |

## Writing requirements

- Use active voice.
- Avoid passive voice.
- Avoid dense paragraphs.
- Prefer bullet points or tables for clear explanations.

## What to say (and what to skip)

- **State what the project is and does** in positive terms. **Avoid** running text whose main job is to say what Rex **is not** (for example “we are not a product org,” “this is not a quarterly plan”) **unless** that boundary is **directly useful** to the reader: a **real** constraint, **safety** limit, or **compatibility** rule they must follow.
- **Decisions and process history** (what was “discussed,” “voted,” or “rejected in chat”) **belongs only** where it is **required** for the document to work— for example a migration note, a breaking-change announcement, or an ADR. Otherwise describe the **current** design, API, or policy in **present tense** without meta-narration.
- **Prioritization and roadmap docs** still use **MoSCoW** labels such as “Won’t (now)”; those name **deferred** work and **should** point to a **source design doc**, not list idle negations.

## Formatting requirements

- Keep paragraphs to 1-3 sentences.
- Keep one main idea per paragraph.
- Use tables for comparisons, options, and mappings.
- Use numbered lists only for ordered procedures.

## Documentation scope and size (agents and retrieval)

Keep each document **focused**: one dominant topic per file where practical. Prefer **hub-and-spoke** (this file’s feature hubs + links) over **monolithic** pages that mix unrelated concerns—large mixed-topic files dilute retrieval for humans and for tools that chunk by section.

- **Split** when sections serve different audiences or lifecycles; **link** to ADRs for decisions instead of copying full inventories into roadmaps.
- **Stable headings** matter: they are the natural boundaries for scanning and for retrieval-sized slices.
- **Heuristics over hard caps:** refactor structure when a file accumulates unrelated narratives; avoid fixing only line counts.

Project agents should follow the **`documentation-content`** rule for `docs/**/*.md`; contributors using shared Cursor rules should align with the **`documentation-scope-for-agents`** global rule for scope and splits (see workspace policy names—not editor install paths).

## Minimum checklist before merge

- Is the document easy to scan?
- Did you replace long narrative blocks with bullets or tables where possible?
- Are assumptions and outcomes explicit?
- Do examples match the current codebase and scope?
- Is the file still **one coherent topic**, or should part of it move to a linked page?
- Does the text **add value** on its own, or does it mostly restate what Rex does **not** do / what was **decided in conversation** without helping a future reader? If so, trim or refocus.
