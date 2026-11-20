# Task location and local integration

## Choose the Task Checkout

The [advance checkout choice](../../AGENTS.md#choose-the-task-checkout)
applies before selecting these integration and worktree procedures.

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
merge that preserves both histories.
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

## Discover and Select Deliveries

At the integration decision use the authoritative task checkouts. An eligible
source has a recorded implementation completion and a clean worktree and index
at the recorded commit. Its task-owned declaration binds the plan revision,
plan and design input digests, material-amendment record, matching implementation
result pair, branch, original tip, dependency revisions and remaining acceptance
work. Record the actual disposition of required blockers before declaring it.
Combined verification and review can still be due.

The completing implementer runs `delivery.py snapshot` in the task checkout
before returning its result pair. Use the
[source receipt procedure](../references/workflow-guidance.md#record-the-source-at-implementation-completion).
The main session declares readiness in that checkout using the implemented
plan and the result that binds the receipt:

```powershell
python tools/workflow/delivery.py declare --plan <plan> --result <result.json> --blocker-disposition "<decision>" --remaining verification --remaining review --remaining publication
```

Repeat `--remaining` for the applicable steps. Use `closure-content` when final
closure text is still pending, `--working-commit` for an unverified working commit
and `--depends-on TASK=REVISION` for each required plan. Use
`delivery.py withdraw --task <task-id> --reason "<decision>"` in the owning
checkout when a ready task becomes paused or resumes implementation. Re-declare
only after reconciling new inputs and obtaining current completion evidence.

From the primary checkout, save a concrete offer:

```powershell
python tools/workflow/delivery.py discover --output .agent/tmp/delivery-offer.json
```

The offer lists the current local master `B`, applicable plan IDs, revisions,
branches and exact original tips `S_i` in dependency order. Git registrations
without declarations, duplicate ownership, stale inputs, moved tips, dirty sources,
unmet dependencies and published plans yield no offer for those sources. A tip
already present in master requires inspection of the surviving publication before
resuming any outstanding step.

For one eligible plan continue with its selected delivery. For two or more,
present one combined choice under the
[saved-plan delivery guidance](../references/workflow-guidance.md#start-or-deliver-a-saved-plan).
A decline continues the already selected single plan. Record the exact answer
and the offered set used to obtain it:

```powershell
python tools/workflow/delivery.py select --offer .agent/tmp/delivery-offer.json --task <task-id> --answer "<decision>" --output .agent/tmp/delivery-selection.json
```

Repeat `--task` for the selected combined sources and supply `--assembly-branch`.
To authorize teardown, repeat `--cleanup refs/heads/<branch>` for the chosen
branches. Record an authorized ordinary push with `--push-remote <remote>`.
Omitting either option leaves that step unselected. A changed offered source
or base requires a fresh comparison before the selection is used.

The single-plan route uses the current ancestry and supplied confirmation.
Pass `--confirmation <path>` for the approved exact tested-tip record in the
source checkout. Its short route uses
`python tools/verification/verify.py integrate accept-tested confirmation <path> target master`.
A working commit, unconfirmed tip or pending closure content goes through full
verification and review. A divergent task first merges recorded master in its
own checkout. Both routes seal the accepted tip and run the full English scan.

## Combined Sources

The advanced protocol remains at the
[integration tool reference](../../tools/verification/README.md#selected-source-set).
Revalidate the selected artifact before provisioning the assembly. Create the
separate checkout and branch from `B` using the existing provisioning command
`tools/worktree.ps1 add <assembly-branch> -Base <B>`. Revalidate again just before
the first merge and export its native sources manifest:

```powershell
python tools/workflow/delivery.py revalidate .agent/tmp/delivery-selection.json
python tools/workflow/delivery.py revalidate .agent/tmp/delivery-selection.json --provisioned --sources-output <assembly-checkout>/.agent/tmp/sources.json
```

Use the recorded merge order in the assembly checkout and merge its exact
original source tips. Preserve the initial assembly tip and every original
source tip in assembly history. Leave the original checkouts available and unchanged.
Intermediate assembly commits may precede final preparation/diary but await combined
verification; record their identities and retain source worktrees.

Verify the assembled history and complete remaining candidate once against `B`.
The canonical record's HEAD binds assembly; the frozen partition covers only later
mechanical commits. Several sources always require combined canonical evidence;
source-only passes or sequential publication cannot replace it. Explicit combined
verification also overrides the single-source tested-tip exception. Integration
alone is not roadmap or milestone closure. Use the explicitly selected Combined
Sources protocol, including the exported native sources manifest at freeze and
the recorded selection at publication.

## Conditional Publication and Cleanup

[TA-close-task](../skills/TA-close-task/SKILL.md) consumes already committed, reviewed,
verified and sealed readiness or the eligible supplied tested-tip record. It
publishes exactly `B -> C` through the existing conditional tool or refuses stale
prerequisites. Require `B` in `C`'s ancestry, the accepted tree equal to `C^{tree}`,
clean unchanged source state and a passing full English receipt for the exact
sealed candidate. A separate SHA comparison followed
by an ordinary merge is insufficient. If master advances, return `master_changed`;
the caller records the fresh immutable base and rechecks the selected sources.
Repeat applicable synchronization, reconciliation, verification, review and scan
under the [recorded refresh delegation](../references/workflow-guidance.md#start-or-deliver-a-saved-plan).
Return to the user when changed requirements, sources or consequences exceed it.
For ordinary teardown, select the existing registered capture execution at
publication. No publisher repair, new checks, commits or content edits are allowed.

For advanced combined publication, every original source tip remains in accepted
history; other sources stay clean/unchanged. Prepared reference locks cover all
sources and one target transition. Assembly cleanup uses final `C`; other source
cleanup uses original `S_i`. Reference updates are atomic; filesystem changes are
not. Recover only known consistent transitions through existing phase records;
observed interference blocks recovery. Never blindly reset or delete uncertain locks.

Include `delivery <selection-path>` on the native `integrate publish` command for
a selected delivery. This carries the original plan identities and authorized
cleanup and push scopes into the surviving publication record. After a successful
update, refresh the project graph for `C` in the primary checkout under the
[post-publication closure procedure](../../tools/verification/README.md#post-publication-closure).
A failed refresh leaves closure open and the publication intact. Resume the
outstanding stage from the surviving record.

After the graph stage, complete the
[whole-file preservation step](../../tools/workflow/capture/README.md#preserve-selected-files-at-task-closure)
using the surviving publication record. Keep the source and retry inputs while a
required file remains active, uncopied or unavailable. Use the established
approved-input version evidence and confirm required Git ancestry and committed
input bytes when applicable. Keep tracked approved inputs unless an explicit
closure change authorizes removal. Separately selected archive analysis and
assessment do not gate ordinary cleanup.
For selected cleanup, run `delivery.py preservation-inputs <publication-record>`
to identify the exact required files for each source plan. Include them in the
existing whole-file preservation selection. The combined execution records each
plan's preserved paths and digests and can resume incomplete copies.

After confirmed publication and completed closure stages, separately authorized
[TA-cleanup-task](../skills/TA-cleanup-task/SKILL.md) processes only the recorded
authorized branches and rechecks their exact tips, ancestry,
ownership, pending work and active commands. Preserve later/unrelated work. Remove
all ignored contents with the task worktree without inventorying, hashing, comparing,
copying or archiving them. Use existing non-forced teardown from the surviving
checkout; confirm directory and registration removal, then conditionally delete
only the unchanged integrated branch not checked out elsewhere. No recursive
fallback after Git failure, unrelated pruning or implicit bootstrap execution.

Keep the published control record under `.agent/tmp/integration/<batch-id>/` in the
surviving target until cleanup completes. `delivery.py status <publication-record>`
reports the original and published tips for each selected plan, pending or completed
cleanup, and any selected push. For an authorized push run
`delivery.py push <publication-record>` in the primary checkout. It attempts only
the published commit and recorded remote master. The surviving closure record
keeps the push outcome for retry. A rejection leaves local publication intact.
Partial cleanup retries use the surviving record without re-verification,
new commits or publication. Review bootstrap procedure changes and recommend needed
primary-checkout steps without running them automatically.
