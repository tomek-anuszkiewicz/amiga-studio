---
name: git-close-task
description: Integrate one or several selected task branches into local master as one verified result and close their tasks, plans, branches and worktrees; discover the exact set and obtain one concrete plan approval. A commit-only request retains its ordinary scope.
---

# Close Tasks Through Local Integration

Use `$git-close-task <branch>` or a natural-language request naming several
branches/worktrees for complete local integration and task closure. The default
target is local `master`. Follow the [integration rule](../../rules/git-merge-commits.md)
for authorization and exceptions, and the [tool reference](../../../tools/verification/README.md#local-integration)
for record formats and command options. Invoking the skill proposes the workflow;
it does not approve its concrete plan or authorize remote operations.

## 1. Discover the Set and Obtain One Approval

Use native Git to inspect the target and every selected source: registered
checkouts, immutable tips, pending operations, staged/unstaged/untracked changes,
task ownership, stable task IDs, plan copies, dependencies and active commands.
An explicit request for all task worktrees permits discovery of that set;
registration alone does not prove completion. A request for committed work does
not select pending changes. Resolve ambiguous ownership or dependencies first.
Discovery is read-only. Record immutable target `B` and each original source `S_i`.

For one source select a path, with pending task changes taking precedence:

| Source state | Proposed sequence |
| --- | --- |
| Pending task changes | Commit only approved task changes as awaiting verification; synchronize `B`; prepare and verify the combined result |
| Clean committed branch, divergent history | Synchronize `B`; prepare and verify the combined result |
| Clean committed branch, `B` already an ancestor of source tip `C` | Determine whether exactly `C` was verified; confirmed results need no new verification; working or unknown results use the verified path |

Several sources always require one canonical combined verification. An explicit
request for combined verification also overrides the single-source tested-tip
shortcut. Present one concrete approval plan for the full source set and merge
order, exact branches/checkouts/plans, selected changes, explicitly selected
assembly checkout, verification scope, commits, closure records and exact cleanup
paths. Do not implicitly create or relocate an assembly checkout. Use a selected
source as the assembly branch; a new task checkout requires its own explicit
selection under the [checkout rule](../../rules/git-merge-commits.md#choose-the-task-checkout).

Default to real merges preserving commits. Rebase, cherry-pick and squash require
explicit direction; identify rewritten commits in the plan. For unknown
single-source fast-forward status, ask whether exactly `C` is tested in the same
approval exchange. Clear session evidence can answer that question, but does not
replace plan approval. Wait for approval before mutation; ask again only for
material scope, method or baseline changes.

## 2. Assemble the Approved Result

For selected pending task changes, make only the approved local working commits
without preliminary tests, audits, review or preparation. Mark them as awaiting
integration verification. Preserve unrelated index entries and record their
identities and actual status in the final closure entry. Record the resulting
immutable source tips before assembly. This path requires combined verification.

In the selected assembly checkout, synchronize exactly `B`, then merge other
selected `S_i` in dependency order. Use `git merge --no-ff --no-commit <commit>`
for divergent histories and `git merge --ff-only <commit>` for simple source
synchronization. Resolve conflicts without stashing, resetting or relocating
unrelated work. If a single-source rebase was explicitly approved, retain its
original tip `S` before rewriting history.

For a batch, finish the real intermediate merges needed for assembly before
verification. The [bounded exception](../../rules/git-merge-commits.md#authorization-and-preparation)
allows these local merge commits to await combined verification and diary
recording. Identify them as unverified in their messages and record their IDs
in the closure entry. They are not individually accepted or published. The
canonical evidence binds the assembled HEAD, including its exact parent history;
the frozen partition covers only the remaining prepared commits. Never substitute
successive publication of partial results for acceptance of the whole batch.

Reconcile code, tests and every tracked Markdown path changed from the common
base in any selected history or the target, including conflict-free paths and
both move endpoints. Compare the common base, source versions and candidate
against the assembled code. Keep the per-path checklist under `.agent/tmp/` and
substantive decisions in closure records and prepared commit messages.

Prepare a durable closure record for each task with its stable ID, delivered
scope, decisions, actual checks and remaining limits. Existing committed records
may suffice. Transfer remaining required work to the roadmap or an active plan.
Prepare tracked plan removals in the verified partition; retain ignored plan
copies and every source checkout through verification and publication. Integration
alone does not establish roadmap or milestone completion.

## 3. Prepare and Verify One Combined Candidate

Follow [batch preparation](../../rules/git-commits.md): scoped compliance,
applicable coverage review, language resolution, diary entries, final partition
and four independent [review lanes](../code-review/SKILL.md). In the assembly checkout:

```powershell
python tools/verification/verify.py commit --integration-base <B>
```

For an explicitly approved single-source rebase, add `--integration-source <S>`
on every run to preserve original divergence classification. Several-source
integration preserves the original tips in history and uses real merges.

This verifies the complete result against `B`, including committed source work;
source-only passes cannot establish combined correctness. Divergent integration
retains full ordinary-workspace coverage and standard harness tests. Full
SingleStep and broader milestone checks retain their own triggers. Correct
failures and repeat the same command, reusing only evidence the verifier validates.

For several sources, prepare the immutable source-list artifact described in the
[tool reference](../../../tools/verification/README.md#selected-source-set) and bind
its digest in consolidated review approval. Use `integrate freeze` with `sources
PATH`; freeze also checks source ownership, tips and inclusion in assembled
history. Freeze passing evidence, complete partition, approval and selected
sources together; make only prepared commits and run `integrate seal`. No routine
expensive rerun is needed merely because mechanical commits change HEAD.

For a clean unchanged single-source fast-forward, the approved plan may instead
accept clear session evidence or affirmative user confirmation that exactly `C`
was tested. Record that determination with `integrate accept-tested`. Do not
fabricate canonical results or run new tests, audits, reviews or commits. A working
commit made during this invocation never qualifies. Any content change, including
a diary entry, requires the canonical path.

## 4. Publish Once and Close Each Task

Run `integrate publish` on the sealed record. It locks all selected references
and conditionally advances exactly `B -> C`, updating the target checkout. Do not
substitute an ordinary target-side merge. If the target advances, obtain approval
of an updated plan before synchronizing and verifying its new immutable tip.
Use `integrate recover` after interruption; preserve unexpected edits for a
concrete decision. Publication retains its control record under
`.agent/tmp/integration/<batch-id>/integration.json` in the surviving target checkout.

After confirmed publication and committed closure records, remove only approved
closed-task plan copies from the surviving checkout. Check current contents and
preserve unrelated or newly changed plans. Ignored source plans disappear with
their worktrees. Do not archive closed plans as active work.

Review `git diff --name-only <B> <C> -- tools/bootstrap/ docs/developers.md
.agents/skills/git-worktree/` and any changed bootstrap instructions. Recommend
necessary bootstrap steps for the primary checkout; do not run them automatically.

Run `integrate cleanup record .agent/tmp/integration/<batch-id>/integration.json` from
the surviving target checkout. It checks each source independently against its
recorded tip (`C` for the assembly branch, original `S_i` for other sources),
ancestry, ownership, pending work and active commands. It uses ordinary
`tools/worktree.ps1` removal and conditional local branch deletion, with no force,
recursive fallback, pruning or ignored-file archives. All ignored files in removed
checkouts are discarded, including configuration, references, caches and reports.
Do not inventory, hash, compare, copy or archive them.

Report directory, registration, branch and plan outcomes separately for each
source. Preserve later work and active checkouts. A blocked source does not undo
publication or prevent independent sources from being cleaned. Retry from the
surviving control record without repeating verification or commits; it remains
available even after the assembly checkout is removed. Report original/final
commits, actual checks or confirmation source, limitations and bootstrap advice.
Narrower commit-only or merge-only requests retain their stated scope.
