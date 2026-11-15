---
name: git-close-task
description: Integrate a task branch into local master and close the task, including its plan, branch and worktree; inspect state, choose one of three paths and obtain plan approval before making changes. A commit-only request retains its ordinary scope.
---

# Close a Task Through Local Integration

Use `$git-close-task <branch>` for complete local integration and task closure.
The default target is local `master`. Follow the [integration rule](../../rules/git-merge-commits.md)
for authorization and exceptions, and the [tool reference](../../../tools/verification/README.md#local-integration)
for publication, recovery and cleanup commands. This invocation proposes the full
workflow; it does not approve its concrete plan or authorize remote operations.

## 1. Inspect and Obtain Plan Approval

Use native Git to identify both branches and registered checkouts, their tips,
pending operations, staged/unstaged/untracked changes and task ownership. Record
the immutable target tip as `B` and original source tip. Identify the task ID,
associated plan copies and unique ignored files that must survive cleanup.
Do not infer scope from a shared dirty checkout or modify files during discovery.

Select exactly one path, with pending task changes taking precedence:

| Source state | Proposed sequence |
| --- | --- |
| Pending task changes | Commit only approved task changes as awaiting verification; synchronize `B`; prepare and verify the combined result |
| Clean committed branch, divergent history | Synchronize `B`; prepare and verify the combined result |
| Clean committed branch, `B` already an ancestor of source tip `C` | Determine whether exactly `C` was verified; confirmed results need no new verification; working or unknown results use the verified path |

Present exact branch/worktree/plan names, selected changes, merge or explicitly
requested rebase, rewritten commits if applicable, verification location and
scope, proposed commits, closure record and cleanup/preservation paths. Ask the
user to approve the plan and wait. For unknown fast-forward test status, ask in
the same exchange whether `C` is verified or working/unknown. Clear session
evidence answers this status question, but never replaces plan approval.
Ask again only for material changes to the approved scope, method or baseline.

## 2. Prepare the Approved Result

For pending task changes, create the approved local working commit without
preliminary test, audit, review or preparation passes. Inspect scope and preserve
unrelated index entries; record in its message that integration verification is
pending. Include the working commit and its actual status in the final diary
entry. This path always requires integrated verification.

In the task checkout, synchronize exactly `B`. For divergent histories, default to:

```powershell
git merge --no-ff --no-commit <B>
```

Use `git merge --ff-only <B>` for simple source synchronization. Rebase only when
explicitly requested and included in the approved plan. Before rebase, record the
source tip as immutable `S`, after any working commit. Preserve unrelated work;
do not stash, reset or relocate it to bypass an obstruction.

For divergent integration, inventory every tracked Markdown path changed from
the common base on either side, including conflict-free paths and both endpoints
of moves. Reconcile common-base, both branches and candidate against the code.
Keep the per-path checklist under `.agent/tmp/`; retain substantive decisions in
the closure diary entry and prepared commit message.

Prepare a durable closure record with the stable task ID, delivered scope,
decisions, actual verification and remaining limits. Existing committed records
may already suffice. Transfer still-required future work to the roadmap or an
active plan. Do not claim a roadmap stage is complete merely to close this task;
actual stage closure retains its own audits and milestone checks.

## 3. Verify Once or Confirm Unchanged Fast-Forward

For the verified path, follow [batch preparation](../../rules/git-commits.md),
including scoped compliance, applicable coverage review, final diary entries,
commit partition and the four independent [review lanes](../code-review/SKILL.md). Run in
the selected task checkout:

```powershell
python tools/verification/verify.py commit --integration-base <B>
```

After an explicitly requested rebase, use the recorded original tip on every run:

```powershell
python tools/verification/verify.py commit --integration-base <B> --integration-source <S>
```

This retains the original divergent classification even though rebased history
contains no merge. Freeze rechecks selection from that recorded original tip.

This checks the complete integrated result, including committed task work; do not
run a separate profile before synchronization. Divergent integration retains
full ordinary-workspace coverage and standard harness tests. Correct failures
and repeat the same command, retaining demonstrably valid groups. Milestone
checks retain their own triggers. Freeze passing evidence, approval and partition;
create only prepared commits and seal their exact consumption. No routine
expensive rerun is needed merely because mechanical commits change HEAD.

For a clean unchanged fast-forward, use clear session evidence or the user's
affirmative confirmation that exactly `C` was verified. Record that determination
and the approved plan using `integrate accept-tested`, then publish its sealed
record. Do not fabricate canonical test results or run new tests, audits, review
or commits. Any content change, including a new diary entry, requires the verified
path. A working commit created in this invocation never qualifies for this shortcut.

## 4. Publish and Close

Publish the sealed record through `integrate publish`. It conditionally advances
exactly `B -> C` and updates the target checkout; do not substitute an ordinary
target-side merge. If the target advances, obtain approval of an updated plan
before preparing the new combined result. Use `integrate recover` after interrupted
publication; preserve unexpected edits for a concrete decision.

After confirmed publication and a committed closure record, remove only the
approved closed-task plan copies from source and target. Check their paths and
current contents before deletion; preserve unrelated or newly changed plans.
Plans are normally ignored; tracked plan removal belongs in the verified commit
partition. Do not archive the closed plan as an active task. Preserve needed
reports, evidence and unique ignored material under the surviving target's
`.agent/tmp/` before teardown.

Run `integrate cleanup` from the surviving target checkout. It preserves local
material, uses `tools/worktree.ps1` for ordinary removal and conditionally deletes
only the unchanged integrated local branch. Stop for later work, changed ownership
or active commands. Never force removal or recursively delete a leftover tree.
Verify branch absence, worktree registration, physical directory and plan copies
separately. Report publication separately from cleanup failure; retry cleanup
without repeating verification or creating replacement commits.

Report original/final commits, actual verification or its confirmation source,
remaining limitations, preserved artifacts and each cleanup outcome. Narrower
commit-only or merge-only requests retain their stated scope.
