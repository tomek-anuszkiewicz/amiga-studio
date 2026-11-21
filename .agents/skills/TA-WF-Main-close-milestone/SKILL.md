---
name: TA-WF-Main-close-milestone
description: Run full milestone tests, remove the selected completed roadmap milestone after success, and compact its settled diary history.
---

# Close the selected verified roadmap milestone

Apply the [result contract](../../references/workflow-contract.md).
Inputs: selected checkout, exact completed heading/item in
[ROADMAP.md](../../../ROADMAP.md) and its completion scope. Do not choose another
milestone or remove an unfinished requirement implicitly.

1. Read the selected roadmap section and supplied completion context. Identify
   which descendants are completed and preserve any distinct unfinished work.
   An ambiguous item or unresolved completion scope returns to the user.
2. Confirm the required local tools/fixtures and avoid concurrent writers to
   shared outputs. Run `python tools/verification/verify.py milestone`, adding
   `--results-dir <path>` when selected, under the existing
   [verification command contract](../../../tools/verification/README.md#completion-verification).
   Preserve the full profile and full SingleStep selection. Await exit status
   and inspect the reports. If tests fail, are interrupted or cannot run, leave the
   selected roadmap item in place and report the failure; do not repair or replace
   the full run with a sample.
3. After success, remove the selected completed section under the criteria below,
   update affected numbering/references and run
   `python tools/verification/checks/check_roadmap.py`.
4. Apply [TA-WF-Step-compact-diary](../TA-WF-Step-compact-diary/SKILL.md) to the
   settled history of the selected milestone. This is included in milestone
   closure; the user does not need to select compaction separately. Preserve
   unrelated and unfinished entries, unique decisions and their evidence. Use
   the actual successful verification and completed scope as the closure trigger.
   If there is no applicable history, report that observation. Report failed
   compaction/checks as partial closure rather than claiming completion.
5. Return the removed scope, retained work, full-test and roadmap-check results,
   diary changes/structure checks and remaining issues.

Stop after verification, the roadmap edit and diary compaction. Do not add code
review, implementation, commits, merge, push or worktree cleanup to this stage.

## Roadmap removal criteria

Remove only the exact verified completed scope. Routine documentation synchronization
does not perform completion-based pruning. A source edit or prepared artifact alone
is not a passing milestone test. Failed or unavailable tests leave the selected item
in place.

Remove verified completed scope completely, without completed tags, checkboxes,
strikethrough or historical capability summaries. Preserve distinct unfinished
requirements with concrete gates, priorities and dependencies. Update numbering,
references and assigned checkpoints. Record removed/retained scope and its
supporting evidence. Do not claim stage closure until its selected completion work
is finished. Report actual checks and unfinished scope.

The roadmap check verifies that the root roadmap exists and contains no completed
`[x]` task checkboxes. It does not validate numbering, links, completion evidence or
milestone readiness; inspect affected numbering and references separately.
