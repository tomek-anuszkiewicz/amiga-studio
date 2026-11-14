---
name: roadmap-maintenance
description: Review completion evidence, consolidate related work, reorder priorities, and prune verified tasks from ROADMAP.md.
---

# Roadmap Maintenance

Use this procedure when reorganizing [ROADMAP.md](../../../ROADMAP.md), reviewing task status, or finishing a listed task. The [roadmap policy](../../rules/roadmap-maintenance.md) defines what belongs in the backlog.

## Review and Reorganize

1. Read the affected tasks and the user's intended next actions. Distinguish project descriptions, methods, completed history, and pending work before editing.
2. Check current implementation, relevant commits, and recorded validation for suspected completed tasks. A plan, prompt, or prepared output directory does not prove delivery. If evidence is incomplete, keep a concrete verification task; for partial delivery, keep only the missing scope.
3. Consolidate related tasks into one workstream with concise scope and acceptance evidence. Preserve substantive requirements, dependencies, unresolved decisions, and failure handling; remove repeated objectives and generic process prose. Detailed execution plans belong in `.agent/tasks/`.
4. Order work by the user's priorities and hardware dependencies. Preparatory conversion may precede hardware implementation; within that implementation preserve bus/clocks -> DMA -> video -> peripherals -> firmware.
5. Renumber tasks and update internal references and affected consumers. Preserve pending extensions when removing project summaries. Keep maintenance instructions here and in the rule rather than copying them into the roadmap.

## Complete a Listed Task

1. Confirm implementation and prepare closure through [batch policy](../../rules/git-commits.md).
   Perform required semantic parity and scoped design review, diary compaction and
   checkpoint updates before final verification and freeze. A reorganization alone
   does not trigger subsystem milestone completion.
2. Prepare deletion of the completed task entirely. Leave distinct unfinished work
   with its own gate; keep evidence in diary and Git rather than a completed-capability
   section. Prepared pruning does not declare closure until final gates, review and
   the required completion commit succeed. Check the final roadmap:
   ```powershell
   python .agents/skills/roadmap-maintenance/scripts/check_roadmap.py
   ```
3. Run both canonical profiles for the final candidate:
   ```powershell
   python tools/verification/verify.py commit
   python tools/verification/verify.py milestone
   ```
   If commit evidence already applies to this candidate, run only the remaining
   milestone set. Milestone requires full SingleStepTests and the full language
   audit; it does not repeat commit tests. After corrections, repeat each affected
   profile's command; the verifier retains only valid groups.
   Do not finalize closure while either required verdict is missing or failed.
4. Include pruning and diary edits in the verified candidate. Any edits made after
   verification return to preparation and continuation. Once frozen, execute only
   the prepared completion commit and report the collected results. Follow the
   policy's reporting requirements when the vAmigaTS verification track resumes;
   the previous scorecard has been removed.

## Verify and Report

Check contiguous numbering, valid references, retained pending scope, and absence of completed-task markers. Report the resulting next actions, any tasks removed on verified evidence, unresolved status, and checks actually run. Do not present reorganization as completion of the work being planned.
