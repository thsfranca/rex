# Documentation Standards (Project)

This project keeps documentation standards in this file and in the `documentation-content` project rule, without referencing editor-specific home directories.

## Source of truth

- **[docs/DOCUMENTATION.md](DOCUMENTATION.md)** (this file) is the human-readable **writing and structure** contract.
- **[docs/PURPOSE_AND_PRINCIPLES.md](PURPOSE_AND_PRINCIPLES.md)** is the canonical **project purpose and operating principles**. Documentation and plans **align** with it when it exists.
- Editors may mirror these norms in a local **`documentation-content`** rule scoped to `docs/**/*.md`. If something conflicts about norms or hubs, **this file wins**; if something conflicts about **purpose or principles**, **`docs/PURPOSE_AND_PRINCIPLES.md` wins**.

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
| **Architecture guidelines** (policies, ownership, doc discipline) | [ARCHITECTURE_GUIDELINES.md](ARCHITECTURE_GUIDELINES.md) |
| **Architecture Decision Records** | [architecture/decisions/](architecture/decisions/) — supersede stale decisions by status + successor ADR |

Diátaxis **role routing** (tutorials, how-to, explanation, reference) is maintained in [docs/README.md](README.md). Do **not** create empty top-level Diátaxis folders; extend the README index when a new role needs a home.

## Documentation by purpose (Diátaxis)

Match each document to **one primary reader need**. Full role lists live in [docs/README.md](README.md).

| Mode | Reader need | What to write | Rex examples | Do not |
|------|-------------|---------------|--------------|--------|
| **Tutorial** | Learn by doing; first success | Step-by-step path to a working outcome; assume little context | [README.md](../README.md) quickstart, [CLI_OPERATOR_UX.md](CLI_OPERATOR_UX.md) | Dump API tables, ADR history, or full architecture |
| **How-to** | Accomplish a specific task | Goal-oriented recipe; prerequisites + ordered steps | [RELEASE.md](RELEASE.md), [CI.md](CI.md), [CONFIGURATION.md](CONFIGURATION.md) | Teach fundamentals or restate full feature design |
| **Explanation** | Understand why and how things fit | Design intent, boundaries, trade-offs, hub-level bets | [PURPOSE_AND_PRINCIPLES.md](PURPOSE_AND_PRINCIPLES.md), feature hubs, [ARCHITECTURE.md](ARCHITECTURE.md) | Replace reference catalogs or operator runbooks |
| **Reference** | Look up facts quickly | Accurate, complete, scannable facts; structure mirrors the thing described | [ADAPTERS.md](ADAPTERS.md), [NDJSON_STREAM.md](NDJSON_STREAM.md), `proto/rex/v1/rex.proto` | Narrative onboarding or undecided design bets |

## Choosing a location

Use this flow before creating or materially editing a doc:

1. **Behavior or contract facts** (API, env vars, capabilities) → **reference** hub under `docs/` or proto source.
2. **Accepted architectural fork** with trade-offs → **ADR** under [architecture/decisions/](architecture/decisions/) (see [README](architecture/decisions/README.md); supersede, do not rewrite accepted ADRs).
3. **Cross-feature policy or ownership** → [ARCHITECTURE_GUIDELINES.md](ARCHITECTURE_GUIDELINES.md).
4. **Feature shape, bets, or interface intent** → **one** feature hub under `docs/` (see [Feature-area hubs](#feature-area-hubs)).
5. **Operator or contributor task** → **how-to** doc (release, CI, configuration, dependencies, …).
6. **Onboarding / first run** → top-level [README.md](../README.md) quickstart or [CLI_OPERATOR_UX.md](CLI_OPERATOR_UX.md).
7. **Unsure** → add a row to [docs/README.md](README.md) repository map only after the doc exists; pick the closest mode above.

## When to create vs update

| Situation | Action |
|-----------|--------|
| Shipped behavior or contract change | Update the **feature hub** (and reference doc if facts changed) in the **same PR** when feasible |
| New top-level `docs/*.md` file | Add a row to [docs/README.md](README.md) (repository map and/or Diátaxis role table) in the same change |
| Architectural yes/no with consequences | New **ADR** at decision time; link from hub or [ARCHITECTURE_GUIDELINES.md](ARCHITECTURE_GUIDELINES.md) |
| Accepted ADR superseded | New ADR with **Supersedes** / status update on the old record — see [architecture/decisions/README.md](architecture/decisions/README.md) |
| New roadmap feature row | **Hub first**, then row — see [Roadmap and new features](#roadmap-and-new-features) |
| Deliberate doc lag | One-line scope note in [ROADMAP.md](ROADMAP.md); do not leave silent drift |

## Writing requirements

- Use active voice.
- Avoid passive voice.
- Avoid dense paragraphs.
- Prefer bullet points or tables for clear explanations.
- **Lead with relevance:** state the most important information first so readers can tell whether the page applies to them.

## What to say (and what to skip)

- **State what the project is and does** in positive terms. **Avoid** running text whose main job is to say what Rex **is not** (for example “we are not a product org,” “this is not a quarterly plan”) **unless** that boundary is **directly useful** to the reader: a **real** constraint, **safety** limit, or **compatibility** rule they must follow.
- **Decisions and process history** (what was “discussed,” “voted,” or “rejected in chat”) **belongs only** where it is **required** for the document to work— for example a migration note, a breaking-change announcement, or an ADR. Otherwise describe the **current** design, API, or policy in **present tense** without meta-narration.
- **Prioritization and roadmap docs** still use **MoSCoW** labels such as “Won’t (now)”; those name **deferred** work and **should** point to a **source design doc**, not list idle negations.

## Formatting requirements

- Keep paragraphs to 1-3 sentences.
- Keep one main idea per paragraph.
- Use tables for comparisons, options, and mappings.
- Use numbered lists only for ordered procedures.

## Agent retrieval entrypoints

- **Task router:** [AGENTS.md](AGENTS.md) — load one primary doc per contributor or agent task; avoid pulling the full [README.md](README.md) index.
- **Machine index:** root [llms.txt](../llms.txt) and [manifest.yaml](manifest.yaml) — role, status, and section anchors for retrieval-sized slices.

## Retrieval header (required on feature hubs)

Place a blockquote immediately after the title on every **feature-area hub** and on docs listed in [manifest.yaml](manifest.yaml):

```markdown
> Role: explanation | Status: active | Audience: contributors | Read when: designing sidecar spawn
> Prefer: ## Boundaries
```

| Field | Values |
|-------|--------|
| **Role** | `tutorial`, `how-to`, `explanation`, `reference` (Diátaxis) |
| **Status** | `active`, `planned`, `design accepted`, `cancelled`, `superseded` |
| **Audience** | `operators`, `contributors`, `agents`, or combinations |
| **Read when** | One line: task or decision that warrants opening this file |
| **Prefer** | Stable `##` section anchor to load first |

## Documentation scope and size (agents and retrieval)

Keep each document **focused**: one dominant topic per file where practical. Prefer **hub-and-spoke** (this file’s feature hubs + links) over **monolithic** pages that mix unrelated concerns—large mixed-topic files dilute retrieval for humans and for tools that chunk by section.

- **Split** when sections serve different audiences or lifecycles; **link** to ADRs for decisions instead of copying full inventories into roadmaps.
- **Stable headings** matter: they are the natural boundaries for scanning and for retrieval-sized slices.
- **Heuristics over hard caps:** refactor structure when a file accumulates unrelated narratives; avoid fixing only line counts.

Project agents should follow the **`documentation-content`** rule for `docs/**/*.md`; contributors using shared Cursor rules should align with the **`documentation-scope-for-agents`** global rule for scope and splits (see workspace policy names—not editor install paths).

## Technical architecture documentation

Architecture hubs, ADRs, and design-intent roadmaps follow the **`technical-architecture-documentation`** global rule unless this project’s **`documentation-content`** rule overrides it (**project wins** on conflict for Rex `docs/`).

| Requirement | Meaning for Rex |
|-------------|-----------------|
| **Architecture-level prose** | Describe components, trust zones, and contracts—not crate walkthroughs. |
| **Diagrams over source** | Prefer **mermaid** for flows and boundaries; avoid Rust/TypeScript function listings in `docs/`. |
| **Named references** | Cite modules (for example `policy.rs`, `approvals.rs`) and proto packages by name; link to code in the repo, do not paste implementations. |
| **Allowed fenced blocks** | Short proto/API excerpts, config examples, operator shell commands—when needed for reproduction. |
| **ADRs** | Decision + consequences + links; narrative detail lives in **hub** docs ([AGENT_ACCESS_POLICY.md](AGENT_ACCESS_POLICY.md), [SIDECAR_RUNTIME.md](SIDECAR_RUNTIME.md), [POLICY_ENGINE.md](POLICY_ENGINE.md), …). |

## Pull request descriptions

- Use [`.github/pull_request_template.md`](../.github/pull_request_template.md) for every PR.
- If a change updates **shipped behavior, setup, or operations** but the matching **`docs/`** edits are **only** in **another open pull request**, add under **Summary** a line **`Documentation:`** with a link to that PR (`#NNN` or full URL for this repository). Leave the template’s docs checklist item **unchecked** on the code-only PR until documentation is merged or included on that branch. Do not enable auto-merge until the checklist is honestly complete — see [DEVELOPER_EXPERIENCE_GUIDE.md](DEVELOPER_EXPERIENCE_GUIDE.md) (CI and PR expectations).

## Roadmap and new features

Applies when adding or materially expanding a **product/feature** row in [ROADMAP.md](ROADMAP.md) or [PLUGIN_ROADMAP.md](PLUGIN_ROADMAP.md).

**Exempt (no new hub required):**

- Status-only updates (Met, Done, priority shuffle with unchanged scope).
- **Engineering backlog** `R0xx` rows that harden an **existing** contract under an existing hub (link the hub in Notes).
- Rows that only **link** to an existing hub with unchanged scope.

**Minimum first design** — create or extend a feature hub under `docs/` before the roadmap row. Exemplar: [LONG_TERM_MEMORY.md](LONG_TERM_MEMORY.md). Follow the **`rex-documentation-for-agents`** user guide for shape and placement (policy name only in committed rules).

| Section | Required content |
|---------|------------------|
| **Purpose** | 1–3 sentences; align with [PURPOSE_AND_PRINCIPLES.md](PURPOSE_AND_PRINCIPLES.md) |
| **Status** | `planned`, `design bet`, or `accepted` (with ADR link if accepted) |
| **Scope** | In/out for this design stage (bullets) |
| **Boundaries** | Policy vs mechanism ownership; link [ARCHITECTURE_GUIDELINES.md](ARCHITECTURE_GUIDELINES.md) or ADR when architectural |
| **Interfaces (intent)** | Contract names (proto, NDJSON, env)—no full implementations in `docs/` |
| **Roadmap linkage** | Roadmap **Source(s)** column points to this hub; hub **Cross-links** back to the roadmap |

**Order of work:**

1. Create or extend the feature hub per this file and [docs/README.md](README.md).
2. Add a row to the [feature hub map](README.md#feature-area-hub-map) when the area is new.
3. Add the roadmap row with **Source(s)** = hub link (not prose-only).
4. Run the [minimum checklist before merge](#minimum-checklist-before-merge).

Optional on first pass: mermaid diagram, design-bet table, economics row in [CONTEXT_EFFICIENCY.md](CONTEXT_EFFICIENCY.md) when the feature is a cost lever.

## Minimum checklist before merge

- Is the document easy to scan?
- Did you replace long narrative blocks with bullets or tables where possible?
- Are assumptions and outcomes explicit?
- Do examples match the current codebase and scope?
- Is the file still **one coherent topic**, or should part of it move to a linked page?
- Does the text **add value** on its own, or does it mostly restate what Rex does **not** do / what was **decided in conversation** without helping a future reader? If so, trim or refocus.
- For architecture/design docs: prefer **diagrams** over source listings; see **Technical architecture documentation** above.
- For new roadmap feature rows: hub exists per **Roadmap and new features** above.
