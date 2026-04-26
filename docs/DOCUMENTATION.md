# Documentation Standards (Project)

This project keeps documentation standards in this file and in the `documentation-content` project rule, without referencing editor-specific home directories.

## Source of truth

- **[docs/DOCUMENTATION.md](DOCUMENTATION.md)** (this file) is the human-readable contract.
- The **`documentation-content`** project rule (scoped to `docs/**/*.md`) repeats the same expectations for agents. If something conflicts, **this file wins**.

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

## Minimum checklist before merge

- Is the document easy to scan?
- Did you replace long narrative blocks with bullets or tables where possible?
- Are assumptions and outcomes explicit?
- Do examples match the current codebase and scope?
- Does the text **add value** on its own, or does it mostly restate what Rex does **not** do / what was **decided in conversation** without helping a future reader? If so, trim or refocus.
