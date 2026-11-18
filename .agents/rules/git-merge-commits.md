# Task location and local integration

## Choose the Task Checkout

Before new implementation, identify the current checkout and branch with Git.
Establish whether to work there or in a new worktree, including when already in a
worktree. An explicit location/new-worktree instruction answers this choice; reuse
it for corrections and continuation. If missing, ask with the current context and
continue only read-only discovery/planning meanwhile. Do not infer location from
a directory name or relocate a branch implicitly.

A new worktree authorizes its branch beside the primary checkout at
`../<repo_name>-<branch_name>`, without a second placement question. Use
[TA-start-plan](../skills/TA-start-plan/SKILL.md) and existing `tools/worktree.ps1`:
independent physical fixtures, references, configuration and graphs; no junctions,
shared targets or shared Cargo cache. Asset/configuration repair is the separately
selected [TA-repair-worktree](../skills/TA-repair-worktree/SKILL.md).

Bounded detached diagnostics retain their existing tool exception without a new
location question. Keep real implementation in its selected checkout, reproduce
the intended dirty inputs/fixtures, and report useful conclusions. Such results do
not replace canonical evidence from the real candidate. Check ownership and remove
the diagnostic checkout; failed cleanup remains outstanding.

## Authorization and Preparation

Distinguish synchronization, commits, publication and cleanup. Supplied approval
must identify exact source/target branches and checkouts, immutable target `B`,
selected pending scope, method, verification location/scope, closure records and
any cleanup paths. An existing concrete authorization remains valid; do not ask
again merely because a separate atomic operation starts. Missing authorization or
a material source/scope/method/baseline change returns to the caller for a decision.
No local operation implies fetch, push or remote branch deletion. Work selected on
the primary branch uses ordinary preparation, without a self-merge or removal.

Synchronize exactly `B` in the selected task checkout through
[TA-sync-master](../skills/TA-sync-master/SKILL.md). Divergent history uses a real
merge by default; rebase needs explicit direction identifying rewritten commits.
Do not silently squash, cherry-pick, stash, reset, relocate work or sweep unrelated
index entries into commits. Pending conflicts return for the separately selected
[TA-resolve-merge-conflicts](../skills/TA-resolve-merge-conflicts/SKILL.md). A narrow
sync/conflict operation does not launch review, verification or publication.

The existing integration exceptions remain precise:

- Approved pending task changes may receive an unverified local working commit
  before tests/audits/preparation/diary. Mark it awaiting integration verification
  and record it in final closure. Synchronize `B` and verify the full result even
  when that commit makes publication a fast-forward. It is never a tested-tip shortcut.
- Divergent committed history requires synchronization followed by complete
  combined verification, regardless of source-only evidence.
- Only one clean unchanged source tip `C` already containing `B`, with committed
  closure content and supplied affirmative evidence/confirmation that exactly `C`
  was tested, can use the approved unchanged fast-forward exception. Unknown or
  working status requires canonical verification. Any new content, including a
  diary edit, loses this exception. Confirmation is not fresh test evidence.

A merge commit can finish before final acceptance only within the approved
working/integration-assembly exceptions. Other synchronization returns its pending
merge candidate when ordinary commit prerequisites are absent. A clean pending
merge is a supported handoff to separately selected preparation, canonical
verification/review and mechanical completion. The existing freeze record captures
`merge_heads`; the prepared first commit must consume those parents before sealing.
Only unresolved conflicts require the conflict-resolution operation. Neither route
grants missing commit authority or starts the subsequent steps automatically.
Every complete candidate requires separate semantic reconciliation of code, tests and all tracked
Markdown changed on either side, including conflict-free paths and both move
endpoints. Compare common-base, both branches and merged code; preserve decisions
in diary/merge records. A clean textual merge is not semantic agreement.

## Combined Sources

The advanced protocol remains at the
[integration tool reference](../../tools/verification/README.md#selected-source-set).
Select the exact task IDs, plan copies, dependencies, original tips `S_i`, assembly
checkout, merge order and `B` before mutation. Registered worktrees do not establish
completion; committed selection excludes pending changes unless approved. Assemble
all selected sources and `B` using real merges, preserving original commits.
Intermediate assembly commits may precede final preparation/diary but await combined
verification; record their identities and retain source worktrees.

Verify the assembled history and complete remaining candidate once against `B`.
The canonical record's HEAD binds assembly; the frozen partition covers only later
mechanical commits. Several sources always require combined canonical evidence;
source-only passes or sequential publication cannot replace it. Explicit combined
verification also overrides the single-source tested-tip exception. Integration
alone is not roadmap/milestone closure. The V1 publisher accepts one selected task;
advanced multi-source work uses its explicit combined protocol, never a hidden fallback.

## Conditional Publication and Cleanup

[TA-close-task](../skills/TA-close-task/SKILL.md) consumes already committed, reviewed,
verified and sealed readiness or the eligible supplied tested-tip record. It
publishes exactly `B -> C` through the existing conditional tool or refuses stale
prerequisites. Require `B` in `C`'s ancestry, the accepted tree equal to `C^{tree}`,
clean unchanged source state and valid evidence. A separate SHA comparison followed
by an ordinary merge is insufficient. If master advances, return `master_changed`;
the caller must select new synchronization/review/verification under updated authority.
No publisher repair, new checks, commits or content edits are allowed.

For advanced combined publication, every original source tip remains in accepted
history; other sources stay clean/unchanged. Prepared reference locks cover all
sources and one target transition. Assembly cleanup uses final `C`; other source
cleanup uses original `S_i`. Reference updates are atomic; filesystem changes are
not. Recover only known consistent transitions through existing phase records;
observed interference blocks recovery. Never blindly reset or delete uncertain locks.

After confirmed publication and committed closure, separately authorized
[TA-cleanup-task](../skills/TA-cleanup-task/SKILL.md) rechecks exact tips, ancestry,
ownership, pending work and active commands. Preserve later/unrelated work. Remove
all ignored contents with the task worktree without inventorying, hashing, comparing,
copying or archiving them. Use existing non-forced teardown from the surviving
checkout; confirm directory and registration removal, then conditionally delete
only the unchanged integrated branch not checked out elsewhere. No recursive
fallback after Git failure, unrelated pruning or implicit bootstrap execution.

Keep the published control record under `.agent/tmp/integration/<batch-id>/` in the
surviving target until cleanup completes. Report publication and each cleanup
outcome separately; partial cleanup retries use that record without re-verification,
new commits or publication. Review bootstrap procedure changes and recommend needed
primary-checkout steps without running them automatically.
