---
trigger: model_decision
description: Mandatory updating and pruning of ROADMAP.md whenever an active milestone or step is completed and verified.
---

# Roadmap Maintenance & Milestone Completion Rule (`ROADMAP.md`)

This rule governs the continuous synchronization, pruning, and milestone completion workflow for [`ROADMAP.md`](../../ROADMAP.md).

---

## 1. Step Completion & Active List Pruning

Whenever an agent is 100% certain that a roadmap milestone or step in [`ROADMAP.md`](../../ROADMAP.md) has been fully implemented and verified (all unit, integration, and architecture tests pass):
- As part of that **same task/commit**, you **must update [`ROADMAP.md`](../../ROADMAP.md)**:
  1. **Remove the completed task** from the active implementation list.
  2. **Update the concise completed baseline summary** at the top of the roadmap with the newly verified milestone and its capabilities.
  3. **Renumber and reorder remaining steps** so that [`ROADMAP.md`](../../ROADMAP.md) always reflects the live, remaining backlog.

---

## 2. Major Milestone Completion Gates

Whenever completing a major milestone (e.g. completing an entire phase or major subsystem block):

1. **Diary Compaction (`compact-diary`):**
   - Invoke the `compact-diary` skill ([`.agents/skills/compact-diary/`](../skills/compact-diary/SKILL.md)) to synthesize older completed milestone entries in [`DIARY.md`](../../DIARY.md) into high-level architectural digests.
2. **Dead Code Pruning (`prune-dead-code`):**
   - Invoke the `prune-dead-code` skill ([`.agents/skills/prune-dead-code/`](../skills/prune-dead-code/SKILL.md)) to audit and eliminate unreferenced functions, obsolete constants, unused imports, and superseded scaffolding across workspace crates.
3. **Milestone Review Protocol (`/code-review`):**
   - Run the `/code-review` workflow to audit the diff with a clean context before final user hand-off.
