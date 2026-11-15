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
local integration with cleanup. A commit alone permits neither publication nor
branch/worktree removal. Complete integration includes removal of the unchanged
integrated source without another permission question. No local operation implies
fetching, pushing or remote branch deletion. Work selected on the primary branch
uses ordinary preparation without a feature branch, self-merge or removal phase.

For integration between different branches, resolve the primary branch and its
checkout, then record its immutable commit as `B`. Inspect both checkouts, index
entries, pending Git operations and unrelated work. Synchronize the task branch
with exactly `B` in the task checkout. Divergent histories require a real merge;
never silently rebase or squash. Do not stash, reset, relocate work or make unverified
commits to bypass a dirty merge. Retain pending work only when Git can preserve it
and the candidate remains exact. Otherwise, under existing commit authorization,
explain and prepare the selected pending work through ordinary gates first, then
synchronize. An ambiguous or unrelated obstruction requires a scoped decision.

Reconcile code, tests and every tracked Markdown path changed on either side of
a divergent merge, including conflict-free paths and both move endpoints. Review
the common-base version, both branches and the proposed result against the merged
code. Preserve decisions in the diary and merge message. A clean textual merge
does not establish semantic agreement.

Prepare the complete integrated candidate relative to `B`, including committed
task work and pending changes. Follow [batch preparation](git-commits.md), with
compliance, applicable coverage review, language resolution, diary preparation,
independent approval and canonical verification. Divergent integration retains
full ordinary-workspace verification. Existing milestone triggers remain intact.
Freeze the tree, complete partition/messages, `B`, passing evidence and final
review together. Mechanical commits must consume exactly that frozen partition;
do not rerun expensive profiles solely because those commits change HEAD.

## Conditional Publication and Cleanup

Follow [git-resolve-merge](../skills/git-resolve-merge/SKILL.md) and the
[integration command reference](../../tools/verification/README.md#local-integration).
Publication must change the primary branch from exactly `B` to the sealed source
commit `C`, or refuse. Require `B` to be an ancestor of `C`, the approved tree to
equal `C^{tree}`, clean source state and matching retained evidence. A separate
SHA comparison followed by an ordinary merge cannot supply this guarantee.
If the target advances, synchronize its new immutable tip into the task branch,
prepare the new combined scope and review, and rerun the canonical profile.
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
