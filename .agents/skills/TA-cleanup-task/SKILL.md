---
name: TA-cleanup-task
description: Remove only an authorized closed task's unchanged plan copies, worktree and branch after confirmed publication.
---

# Clean up the published task

Apply [the invocation contract](../../references/workflow-contract.md) and the
[cleanup policy](../../rules/git-merge-commits.md).
Executor: main session; result kind: `cleanup`.
Inputs: confirmed publication record, committed closure record, stable task ID,
exact authorized plans/worktree/branch, the surviving target checkout and
the required verified preservation references or recorded disposition.

1. Confirm publication and committed closure. Revalidate each selected plan's
   identity; preserve later edits and all unrelated/unfinished plans.
   Recheck preservation for the selected task under the
   [integration policy](../../rules/git-merge-commits.md). Keep the source when
   required input versions, archive checks or selected report receipts are
   missing or conflicting.
2. Remove only the authorized unchanged closed-task plan copies from the surviving
   checkout. Tracked removals must already belong to committed closure content;
   do not create a new commit or alter an accepted tree here.
3. From the surviving checkout run
   `python tools/verification/verify.py integrate cleanup record <surviving-record>`.
   The existing tool rechecks tips, ancestry, registration, pending work and active
   commands, uses `tools/worktree.ps1`, and conditionally deletes local branches.
4. Report plan, directory, Git registration and branch outcomes separately.
   Preserve the surviving control record for partial-cleanup retries.

Stop after cleanup/refusal. Never force removal, prune unrelated registrations or
recursively delete leftovers after Git failure. Ignored contents are discarded
with the worktree; do not inventory, hash, compare, copy or archive them. A cleanup
failure does not undo publication or start verification, commits or publication again.
