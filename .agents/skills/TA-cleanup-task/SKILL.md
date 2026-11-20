---
name: TA-cleanup-task
description: Remove only an authorized closed task's unchanged plan copies, worktree and branch after confirmed publication.
---

# Clean up the published task

Apply [the invocation contract](../../references/workflow-contract.md) and the
[cleanup policy](../../rules/git-merge-commits.md).
Executor: main session; result kind: `cleanup`.
Inputs: confirmed publication record carrying the passing English receipt,
completed project graph and committed closure record, task and execution IDs bound at publication,
exact authorized plans/worktree/branch, the surviving target checkout and
the completed ordinary preservation state for that publication.

1. Confirm publication, its passing English receipt, committed closure and
   completed project graph for the selected publication before removing any plan.
   Apply the [service handoff check](../../../tools/verification/README.md#service-handoff-before-worktree-removal)
   when the task changed or ran an installed service.
   For a selected delivery take the authorized branch subset and original plan
   identities from the surviving record. Use each original source tip and the
   final tip for any selected assembly. Confirm each authorized plan has its own
   preserved file receipts in the closure record. Revalidate its plan identity
   and preserve later edits and all
   unrelated or unfinished plans.
   Confirm the original registration matches the publication's capture identity
   and the existing control record marks the required copies complete for that
   execution at the exact final master SHA. Confirm the archived paths survive. Keep the source when a required file
   remains active, uncopied or unavailable. Follow the
   [integration policy](../../rules/git-merge-commits.md) for approved inputs.
2. Remove only the authorized unchanged closed-task plan copies from the surviving
   checkout. Tracked removals must already belong to committed closure content;
   do not create a new commit or alter an accepted tree here.
3. From the surviving checkout run
   `python tools/verification/verify.py integrate cleanup record <surviving-record>`.
   The existing tool applies the recorded cleanup scope, rechecks tips, ancestry,
   registration, pending work and active commands, uses `tools/worktree.ps1`,
   and conditionally deletes local branches.
4. Report plan, directory, Git registration and branch outcomes separately.
   Preserve the surviving control record for partial-cleanup retries.

Stop after cleanup/refusal. Never force removal, prune unrelated registrations or
recursively delete leftovers after Git failure. Ignored contents are discarded
with the worktree; do not inventory, hash, compare, copy or archive them. A cleanup
failure does not undo publication or start verification, commits or publication again.
