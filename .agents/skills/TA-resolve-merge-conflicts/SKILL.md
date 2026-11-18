---
name: TA-resolve-merge-conflicts
description: Resolve explicitly selected pending merge conflicts and finish that merge within its existing authorization.
---

# Resolve the selected pending merge

Apply [the invocation contract](../../references/workflow-contract.md) and relevant
[merge policy](../../rules/git-merge-commits.md).
Executor: main session; result kind: `merge`.
Inputs: pending merge identity/parents, selected conflict paths, approved requirements,
write scope and existing authority/message for completing that merge.

1. Confirm the pending operation still matches the assignment. Read common-base,
   both source versions and affected requirements. Preserve unrelated index versions.
2. Resolve the selected conflicts against the combined intended behavior. A genuine
   unresolved requirement decision remains `needs_input`; do not invent approval.
3. Check the selected resolutions and finish only that authorized merge when all
   its conflicts are resolved. Record its actual acceptance status, including any
   allowed unverified intermediate merge.
4. Return resolved paths, candidate/commit identity, checks and remaining decisions.

Stop after this merge. Do not start broad review, canonical verification,
publication or cleanup. Clean textual resolution does not prove semantic agreement
of conflict-free Markdown or of the whole combined candidate.

