---
name: TA-WF-Main-close-task
description: After successful final master checks, preserve task outputs, remove the merged worktree/branch/plan and refresh the project Graphify graph on final master.
---

# Close the verified task and refresh its project graph

Apply the [result contract](../../references/workflow-contract.md). Use the
primary checkout, completed plan paths, archive location and successful final
master test and review evidence for the delivered scope. Identify the selected
task worktree and branch if the task used them.
Use the existing task/execution archive under `.agent/task-archives/` when available;
otherwise use the explicitly selected task archive there. This stage authorizes
the selected preservation, cleanup and project graph refresh. Integration and
final master tests/review, including verification of any corrective commits,
must already be complete. Missing, failed or stale required evidence stops
closure; this skill does not run those tests or reviewers itself.

1. Inspect the Git and worktree state and the supplied final master evidence.
   Confirm the evidence covers the current delivered behavior after all
   corrections. Limit any readiness claim to what the evidence covers so it
   does not silently include unrelated later changes. For a task branch,
   confirm that master includes its work. Preserve later commits and uncommitted
   work, along with unrelated files, plans and index entries. Stop if the
   selected work is absent from master or unfinished. Before copying the
   selected outputs, confirm that the processes producing them have finished.
2. Use [TA-WF-Step-collect-task-records](../TA-WF-Step-collect-task-records/SKILL.md)
   to preserve the task's results and logs. Include final master test and review
   outputs from the primary checkout and any working plan or context needed
   after cleanup. Use explicit paths for outputs from the shared primary
   checkout. Record the locations of reports already in the selected archive.
   Inspect the destination paths and continue to cleanup after all selected
   copies succeed.
3. For a task with a selected worktree, run `tools/worktree.ps1 remove <selected-worktree>`
   from a surviving checkout using the script's accepted name or path. Follow
   the script's checks and preserve leftovers after failure. Confirm the
   directory and Git registration outcomes separately.
4. Delete the selected merged local branch if the task used one. Remove only the
   authorized completed-plan copies. Preserve unrelated or later-edited plans.
   If a plan is tracked on master, commit only its selected removal as closure
   housekeeping, preserving the unrelated index; do not leave that deletion as
   an unnoticed dirty change. Check the scoped housekeeping diff; do not commit
   code or change the verified behavior here.
5. In the primary checkout on final local `master`, run
   [TA-Graphify-refresh](../TA-Graphify-refresh/SKILL.md) for `project` using
   `python tools/graphify/refresh.py --graph project`. Await the real result and
   record the source checkout and the final master SHA, including any authorized
   plan-removal commit. Keep the refresh limited to the project graph. It
   updates the graph structure, so report the freshness of document semantics
   separately and leave semantic extraction to its selected operation. If the
   refresh fails, report partial closure and preserve its diagnostics. On retry,
   preserve completed cleanup and leave removed resources removed.
6. Return archive/report and retained plan locations, worktree/Git-registration,
   branch and plan outcomes, project graph command/result, semantic limits and
   any partial failure under the shared result contract. Preserve the archive and execution
   context. Resume only missing closure work after a partial failure; do not
   recreate removed resources or stop shared services.

Stop after closure. Do not merge, run tests/reviewers, repair source, compact the
diary, close a milestone, push or invoke final master verification automatically.
