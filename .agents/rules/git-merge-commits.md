# Task Location and Local Integration

## Choose the Task Checkout

Before each new implementation task, identify the current checkout and branch
with native Git. Ask whether to continue there or create a new task worktree,
including on the primary branch and in an existing task worktree. Name the current
context in the question. Without an answer, continue only read-only discovery or
planning. Never infer relocation permission from directory names or existing
worktrees. An explicit instruction to work here, on a named branch, or in a new
worktree answers the question; reuse that choice for corrections and continuation.
In a new session, ask when the prior choice cannot be established.

Selecting a new worktree authorizes its necessary task branch. Place it beside
the primary checkout as `../<repo_name>-<branch_name>`; no second placement
question is needed. Use independent physical fixtures, references, `.env`, Cargo
caches and graph data, with no junctions or shared targets. Follow
[git-worktree](../skills/git-worktree/SKILL.md).

A disposable detached worktree is allowed without a location question for a
bounded test or diagnostic. Keep the real implementation and final commits in
the selected checkout. Reproduce the intended dirty files and fixtures when
testing a dirty candidate; baseline HEAD alone does not identify it. Report the
experiment, preserve useful results, check ownership and remove the test checkout.
Its results cannot replace canonical acceptance evidence from the selected
checkout. Failed cleanup remains outstanding work.

## Authorization and Preparation

Distinguish ordinary work, commit authorization, merge authorization and complete
local integration with cleanup. A commit alone permits neither integration nor
branch/worktree removal. Use [git-close-task](../skills/git-close-task/SKILL.md)
for complete local integration, task closure and cleanup. Before every integration,
including fast-forward, inspect state read-only, present the concrete plan and
wait for explicit user approval before any repository mutation. Name source and
target branches and checkouts, selected pending changes, integration method,
verification location and scope, commits, closure record and exact cleanup paths.
Include any unresolved tested-commit question in this approval exchange. Approval
covers routine corrections and the stated cleanup; material scope, method or
baseline changes require an updated plan and approval. No local operation implies
fetching, pushing or remote branch deletion. Work selected on the primary branch
uses ordinary preparation without a feature branch, self-merge or removal phase.

For integration between different branches, resolve the primary branch and its
checkout, then record its immutable commit as `B`. Inspect both checkouts, index
entries, pending Git operations and unrelated work. Select one approved path:

- **Pending task changes:** create a local working commit of only the approved
  changes without first running tests, audits, review or batch preparation. This
  is a narrow exception to ordinary commit gates and diary timing: mark the commit
  as awaiting integration verification and record it in the final closure entry.
  Then synchronize exactly `B` and verify the complete result, even if the working
  commit makes publication a fast-forward. Do not claim this commit was tested.
- **Clean committed branch with divergent history:** synchronize exactly `B`,
  then verify the complete result. Prior commit test status does not change this
  requirement; do not verify the old branch separately before synchronization.
- **Clean committed branch allowing target fast-forward:** when `B` is an
  ancestor of the unchanged source tip `C`, establish whether exactly `C` was
  already verified. Use clear current-session evidence or ask the user. Only an
  affirmative determination, included in the approved plan, permits publication
  without new tests, audits, review or commits. Unknown or working status requires
  normal integrated verification. Any new content, including a closure diary
  edit, returns to the verified path. Record user confirmation as confirmation,
  not as freshly executed tests or reconstructed canonical evidence.

Synchronize in the selected task checkout. Use a real merge for divergent
histories by default; rebase requires an explicit user instruction and an approved
plan identifying rewritten commits. Do not silently squash. Do not stash, reset,
relocate work or sweep unrelated index entries into a working commit. An ambiguous
or unrelated obstruction requires a scoped decision.

Reconcile code, tests and every tracked Markdown path changed on either side of
a divergent merge, including conflict-free paths and both move endpoints. Review
the common-base version, both branches and the proposed result against the merged
code. Preserve decisions in the diary and merge message. A clean textual merge
does not establish semantic agreement.

Except for the approved unchanged fast-forward path, prepare the complete
integrated candidate relative to `B`, including working commits, committed task
work and pending changes. Follow [batch preparation](git-commits.md), with
compliance, applicable coverage review, language resolution, diary preparation,
independent approval and canonical verification. Divergent integration retains
full ordinary-workspace verification. Existing milestone triggers remain intact.
Freeze the tree, complete partition/messages, `B`, passing evidence and final
review together. Mechanical commits must consume exactly that frozen partition;
do not rerun expensive profiles solely because those commits change HEAD.

## Conditional Publication and Cleanup

Follow [git-close-task](../skills/git-close-task/SKILL.md) and the
[integration command reference](../../tools/verification/README.md#local-integration).
Publication must change the primary branch from exactly `B` to the sealed source
commit `C`, or refuse. Require `B` to be an ancestor of `C`, the approved tree to
equal `C^{tree}`, clean source state and matching retained evidence or the explicit
unchanged fast-forward confirmation. A separate
SHA comparison followed by an ordinary merge cannot supply this guarantee.
If the target advances, stop publication, present an updated plan and obtain
approval before synchronizing its new immutable tip into the task branch.
Prepare the new combined scope and review, and rerun the canonical profile.
Reuse only demonstrably valid groups; conservative reruns are acceptable.

The tool holds a shared process lock and native prepared reference locks while
advancing the clean target checkout and conditionally publishing the exact commit.
An interruption leaves a recoverable phase record. Inspect actual references,
index and files; recover a known consistent transition or stop for a concrete
decision. Never blindly reset. Reference updates are atomic; filesystem changes
are not. Locks coordinate publishers and reference writers, not arbitrary editors.
Observed interference blocks publication or recovery.

After confirmed publication, revalidate the exact source tip, ancestry, ownership,
pending changes and active commands. Preserve logs, review records and unique
ignored files under the surviving checkout's `.agent/tmp/`. A reused checkout
with later work remains intact. Run teardown from the surviving checkout; protect
the primary checkout and unrelated paths. Use safe native removal, check exit
codes and confirm both directory and registration removal. Never force removal,
prune unrelated registrations or recursively delete a leftover directory after
Git failure. Delete only the unchanged integrated local branch, conditionally,
after confirming it is not checked out elsewhere. Report publication separately
from cleanup failure; retry cleanup without repeating verification or commits.
