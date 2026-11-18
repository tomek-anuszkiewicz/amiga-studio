---
name: TA-sync-master
description: Merge an explicitly recorded local master commit into the selected task branch and return the merge state.
---

# Synchronize the selected task candidate

Apply [the invocation contract](../../references/workflow-contract.md) and the
applicable [merge policy](../../rules/git-merge-commits.md).
Executor: main session; result kind: `merge`.
Inputs: task checkout/branch, immutable current master `B`, source identity,
merge authorization and any authorized merge message.

1. Recheck both branches, checkouts, index versions and pending operations. Confirm
   local master is still exactly `B`; report `master_changed` before mutation
   otherwise. Do not fetch, stash, reset or relocate unrelated work.
2. Merge exactly `B` in the task checkout. Use the existing fast-forward operation
   when applicable, otherwise a real merge preserving history. Rebase, cherry-pick
   and squash require their own explicit direction. A clean merge can finish only
   under existing commit authority or the rule's unverified integration/assembly
   exception; otherwise retain and report its clean pending candidate. Actual
   conflicts leave the authorized pending merge for a separately selected resolver.
3. Return the resulting source/candidate, pending merge state and conflicts. Record
   any permitted unverified merge commit as awaiting combined acceptance.

Stop after synchronization. Conflicts return control without repairs. Combined
code/document semantic assessment, including conflict-free changed Markdown, and
verification remain separately selected prerequisites before readiness.

A clean pending merge can pass to caller-selected batch preparation, canonical
verification/review and prepared commits. The existing freeze record preserves
`merge_heads`; sealing checks the prepared first-parent merge completion. This is
a supported handoff, not a conflict requiring the resolver or an automatic call
to those later operations.
