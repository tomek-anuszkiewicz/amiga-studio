# Selected Git operations

## Choose the task checkout

Use the checkout and branch selected by the user or the existing plan. Inspect
current branches, worktrees and pending changes before editing or merging.
Preserve unrelated files and staged content. Use the existing worktree script
for provisioning; do not duplicate its asset-copying behavior in instructions.

## Authorization and preparation

Run ordinary Git commands for the selected commit, merge or synchronization.
The user chooses the included branches, changes and checks. When integrating a
task branch into local master, check ancestry with
`git merge-base --is-ancestor master <task-branch>`. If master is an ancestor of
the task branch, use `git merge --ff-only <task-branch>`. If the task branch is
already an ancestor of master, report that its commits are already included.
For diverged histories, use `git merge --no-ff <task-branch>`. If Git returns an
unexpected error, report the inspection failure and stop before any forced
integration. A push requires separate authorization.
Do not stash, reset, discard unrelated changes or invent a different target.
Resolve assigned conflicts under the procedure below; report unresolved decisions
or failed commands. Report verification that actually ran and work that
remains. The report needs no freeze, seal, publication or readiness records.

When explicitly asked to merge local master into a task branch, run
`git merge master` in that task worktree. This ordinary operation is not a
prerequisite for implementation, review or delivery.

## Merge conflicts

Confirm the pending merge parents and assigned conflict paths. Read the common
ancestor, both versions and affected approved requirements. Resolve the combined
intended behavior while preserving unrelated index entries. Apply the shared
[internal migration contract](../../../references/internal-migrations.md) to
changed API/schema consumers and
[specification authority](../../../references/spec-compliance.md) to requirement
conflicts. Documentation resolutions retain unique reasoning under
[document ownership](../../../references/information-hierarchy.md) and the
[prose criteria](../../../references/narrative-criteria.md).

Check the selected resolutions and complete the authorized merge only after
resolving its conflicts. Stop if you need a decision about behavior or
interference makes the work unsafe. Do not assume approval. Report the
verification that ran and any intermediate merge that remains unverified.
A clean textual resolution establishes only that the conflicting text has
been reconciled. Claims about agreement in documents without conflicts or
across the combined change require evidence that covers that scope.

## Worktree removal

Preserve task outputs before removing a selected worktree using
[TA-WF-Main-close-task](../SKILL.md). The cleanup operation uses the
existing `tools/worktree.ps1` after successful artifact preservation. Confirm the
physical directory and Git registration outcomes separately. Never force removal
or recursively delete leftovers after a failed Git operation.
