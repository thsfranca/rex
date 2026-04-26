---
name: docs-feature-roadmap-prioritization
description: >-
  Requires updating roadmap and prioritization when project documentation
  adds or expands features. Use when editing docs that introduce capabilities,
  work items, or deferred scope; when adding rows to design roadmaps; when the
  user adds features to documentation; or when closing a doc task that should
  sync ROADMAP or PRIORITIZATION.
---

# Documentation features → roadmap prioritization (Rex)

## Rule

**Any feature added or materially expanded in project documentation must trigger an update to roadmap prioritization** in the same change set, unless the user explicitly defers the roadmap pass (then leave a **short TODO in the PR** and follow up next).

- **“Feature”** here means: a new capability, deliverable, work item, or **Won’t (now)** / deferred line that changes what Rex might build, learn, or park. It includes new bullets in [MVP_SPEC.md](../../../docs/MVP_SPEC.md), [PLUGIN_ROADMAP.md](../../../docs/PLUGIN_ROADMAP.md), [EXTENSION_ROADMAP.md](../../../docs/EXTENSION_ROADMAP.md), and similar, when they imply scheduling or focus.

- **Trivial doc fixes** (typos, phrasing, links, formatting) **do not** require a roadmap pass.

## Where prioritization lives

- **[docs/PRIORITIZATION.md](../../../docs/PRIORITIZATION.md)** — **MoSCoW** (Must / Should / Could / Won’t), **R-ICE** (optional tie-break inside a bucket), and Rex **tie-breakers** (contract risk, CI cost, blast radius).
- **[docs/ROADMAP.md](../../../docs/ROADMAP.md)** — **Now / Next / Later**; short tables with **source links** per row; **Parked in design docs**; “How to refresh this file”.

## Rules to follow (project)

Apply the **[rex-roadmap-docs](../../rules/rex-roadmap-docs.mdc)** project rule whenever you touch [docs/ROADMAP.md](../../../docs/ROADMAP.md) or [docs/PRIORITIZATION.md](../../../docs/PRIORITIZATION.md). In short:

- Every work item in `ROADMAP.md` has a **source link** to an existing project doc (no orphan feature lines).
- **Won’t (now)** items name **which design doc** to reopen.
- Use **MoSCoW** and **R-ICE** terms consistently with [PRIORITIZATION.md](../../../docs/PRIORITIZATION.md); do not paste long scoring essays in `ROADMAP.md` — point back to that file.
- Do not imply new **CI** requirements (for example **Cursor CLI** on required jobs) without matching updates in [docs/CI.md](../../../docs/CI.md) and [docs/DEPENDENCIES.md](../../../docs/DEPENDENCIES.md).

Writing style for docs stays aligned with [docs/DOCUMENTATION.md](../../../docs/DOCUMENTATION.md) and [documentation-content.mdc](../../rules/documentation-content.mdc).

## Checklist (agent)

1. **Identify** whether the doc change is a new or expanded **feature** (or deferred scope) vs. a trivial edit.
2. If it is a feature: **place** the detailed story in the right design doc (see `ROADMAP.md` “How to refresh this file”).
3. **Bucket** the item with **MoSCoW**; use **R-ICE** only to break ties between **Should**-level items.
4. **Update** [docs/ROADMAP.md](../../../docs/ROADMAP.md) (add/move a row, or the Parked table) with **source** column links; re-check [docs/PRIORITIZATION.md](../../../docs/PRIORITIZATION.md) if labels or “how often to touch this” context changed.
5. **Verify** [rex-roadmap-docs.mdc](../../rules/rex-roadmap-docs.mdc) constraints (links, CI claims, no editor-config drift in `docs/**`).

## Exception

[PRIORITIZATION.md](../../../docs/PRIORITIZATION.md) allows a **very small, focused** change to skip a full prioritization pass **only when** the change does **not** shift priorities. This skill treats **feature** documentation as **priority-shifting** by default, so the exception does **not** apply to new or expanded features.
