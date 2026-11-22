---
name: TA-WF-Main-close-task
description: After successful final master checks, preserve task outputs, remove the merged worktree/branch/plan and refresh the project Graphify graph on final master.
---

# Close the verified task and refresh its project graph

Apply the [result contract](../../references/workflow-contract.md). Inputs: primary
checkout, selected task worktree/branch, completed plan paths, archive location
and successful final master test/review evidence for the delivered scope.
Use the existing task/execution archive under `.agent/task-archives/` when available;
otherwise use the explicitly selected task archive there. This stage authorizes
the selected preservation, cleanup and project graph refresh. Integration and
final master tests/review, including verification of any corrective commits,
must already be complete. Missing, failed or stale required evidence stops
closure; this skill does not run those tests or reviewers itself.

1. Inspect the Git and worktree state and the supplied final master evidence.
   Confirm the evidence covers the current delivered behavior after all
   corrections. Limit any readiness claim to what the evidence covers so it
   does not silently include unrelated later changes. Confirm that master
   includes the selected branch's work. Preserve later commits and uncommitted
   work, along with unrelated files, plans and index entries. Stop if the
   selected work is absent from master or unfinished. Before copying the
   selected outputs, confirm that the processes producing them have finished.
2. From the surviving checkout run the existing artifact tool:

   ```text
   python tools/artifacts/preserve.py --worktree <task-worktree> --destination <primary>/.agent/task-archives/<selected-task-archive>/outputs
   ```

   Its defaults cover local timing/command logs, coverage, SingleStep outputs,
   benchmark results and local test archives. Reports already in the primary
   checkout stay there. Add `--include <relative-path>` for known custom reports,
   screenshots and the ignored working plan when inside the task worktree.
   Preserve the needed plan/context and final master test/review results before
   deleting working copies. For selected files outside the tool's ignored
   scope, use an explicit ordinary copy or the existing
   [record preservation operation](../TA-WF-Step-collect-task-records/SKILL.md), without
   inventing a telemetry-registration prerequisite for ordinary file copying.
   Inspect the destination paths and the copy results. Continue to cleanup only
   after all selected copies succeed.
3. Run `tools/worktree.ps1 remove <selected-worktree>` from a surviving checkout,
   using the script's accepted name/path. Keep the script's checks; do not force
   removal or recursively delete leftovers after failure. Confirm the directory
   and Git registration outcomes separately.
4. Delete the selected merged local branch through ordinary Git and remove only
   the authorized completed-plan copies. Preserve unrelated or later-edited plans.
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
   any partial failure in text and JSON. Preserve the archive and execution
   context. Resume only missing closure work after a partial failure; do not
   recreate removed resources or stop shared services.

Stop after closure. Do not merge, run tests/reviewers, repair source, compact the
diary, close a milestone, push or invoke final master verification automatically.
