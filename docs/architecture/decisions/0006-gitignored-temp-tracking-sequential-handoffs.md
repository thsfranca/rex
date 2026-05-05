# ADR 0006: Gitignored TEMP tracking for sequential multi-agent repository work

- **Date:** 2026-05-05
- **Status:** Accepted

## Context

Development on Rex often involves **multiple AI-assisted sessions** (parallel or back-to-back) against the **same Git working tree**. Chat-only handoffs drop **branch intent**, **stash references**, **checklist truth**, and **next safe actions**, which increases duplicated work and merge friction.

Project planning rules already prescribe a **gitignored tracking record** at the repository root for work that modifies the tree: **`TEMP_<short-topic>.md`**, kept current until the **pull request for that slice** exists, then removed. `.gitignore` includes **`TEMP_*`**.

## Decision

1. **Location and lifecycle:** Use a single repository-root **`TEMP_<short-topic>.md`** for each PR slice (or non-PR in-repo deliverable). **Create or refresh it as the first plan step**; **maintain** it while the slice is open; **delete** it after the **PR is opened** (or when non-PR work is complete and handed off).

2. **Authoritative handoff:** That file is the **source of truth** for checklist progress, branch, stashes, blockers, and **next safe step** between sessions. Other agents treat unlisted temp paths as **not owned** by the current slice.

3. **Sequential sessions:** When the **same slice** is continued **one agent or session after another** (not only parallel risk), the tracking file must include **instructions for the next agent**: **purpose of the whole task (this slice)** (why the work exists and what “done” means), **preferred answer format** (how to report progress and completion), and **structure expectations** (how to update the file and the tree so the handoff stays consistent).

4. **Waiver:** Omit the file only when the user **explicitly** states **single-agent, no parallel risk, and no working-tree changes**—and the plan records that waiver.

## Consequences

- **Positive:** Repeatable, low-loss handoffs; aligns with small-PR and planning rules; no committed coordination noise.
- **Negative:** Ongoing discipline to update the file during work.
- **Risks / follow-up:** Assistants must be steered to read and update the file; policy text (planning rules and collaboration guardrails) stays the operational reference.

## Related

- [README.md](README.md) (ADR index) · [../ARCHITECTURE.md](../ARCHITECTURE.md) · root `.gitignore` (`TEMP_*`)
