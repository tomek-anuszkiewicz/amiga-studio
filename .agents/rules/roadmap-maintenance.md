---
trigger: model_decision
description: Mandatory updating and pruning of ROADMAP.md whenever an active milestone or step is completed and verified.
---

# Roadmap Maintenance & Milestone Completion Rule (`ROADMAP.md`)

This rule governs the continuous synchronization, pruning, and milestone completion workflow for [`ROADMAP.md`](../../ROADMAP.md).

---

## 1. Step Completion & Active Backlog Pruning

Whenever an agent is 100% certain that a roadmap milestone or step in [`ROADMAP.md`](../../ROADMAP.md) has been fully implemented and verified (all unit, integration, and architecture tests pass):
- As part of that **same task/commit**, you **must update [`ROADMAP.md`](../../ROADMAP.md)** following the [`roadmap-maintenance`](../skills/roadmap-maintenance/SKILL.md) skill:
  1. **Zero Retention of Completed Items:** Never mark tasks with `[COMPLETED]`, `[Completed: ...]`, `[x]`, or strikethrough. Do NOT retain completed tasks or verbose implementation breakdowns in Section 2.
  2. **Completely Delete Finished Tasks:** Delete the completed task, step, or sub-step text from Section 2 ("Core Implementation Strategy (Remaining Milestones)"). The active roadmap must strictly reflect only pending and in-progress work.
  3. **Update Concise Baseline Summary:** If the completed work represents an architectural milestone or major subsystem capability, update the concise completed baseline summary in Section 1 with high-level verified capabilities.
  4. **Renumber and Reorder Remaining Steps:** Keep remaining steps and sub-steps sequentially numbered and contiguous.
  5. **Historical Logging in DIARY.md:** Granular execution narratives, files touched, and technical rationales belong strictly in [`DIARY.md`](../../DIARY.md) (Section 10) and Git commit history, never in [`ROADMAP.md`](../../ROADMAP.md).

---

## 2. Major Milestone Completion Gates

Whenever completing a major milestone (e.g. completing an entire phase or major subsystem block):

1. **Diary Compaction (`compact-diary`):**
   - Invoke the `compact-diary` skill ([`.agents/skills/compact-diary/`](../skills/compact-diary/SKILL.md)) to synthesize older completed milestone entries in [`DIARY.md`](../../DIARY.md) into high-level architectural digests.
2. **Dead Code Pruning (`prune-dead-code`):**
   - Invoke the `prune-dead-code` skill ([`.agents/skills/prune-dead-code/`](../skills/prune-dead-code/SKILL.md)) to audit and eliminate unreferenced functions, obsolete constants, unused imports, and superseded scaffolding across workspace crates.
3. **Milestone Review Protocol (`/code-review`):**
   - Run the `/code-review` workflow to audit the diff with a clean context before final user hand-off.
