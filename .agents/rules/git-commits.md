# Batch Preparation and Atomic Commits

## Authorization and Ordinary Completion

Ordinary implementation, fixes, reviews and documentation work authorize the
requested changes and relevant focused checks. They do not authorize staging or
commits, or trigger full coverage measurement, language scans, graph refresh,
documentation synchronization or broad audits. Reuse context and evidence while
their inputs remain valid. Handle requirement conflicts immediately.

An explicit request to **prepare a batch** authorizes staging its selected scope,
but not creating commits. A request to **commit** authorizes preparation and the
planned commits without another confirmation. Keep the request active through
routine in-scope corrections and user answers. Ask only for substantive unresolved
decisions, including protected verification or reference changes.

Follow the [task checkout choice](git-merge-commits.md#choose-the-task-checkout)
before new implementation. Preparation does not authorize a new task destination.
Commit-only authorization does not include merging or removal; complete local
integration follows [its owning rule](git-merge-commits.md#authorization-and-preparation).

Approved integration has two narrow exceptions owned by that rule: a local
working commit may precede all verification and record its status in the final
closure diary entry; an unchanged, already verified fast-forward result may be
published without a new preparation batch. Working commits are not acceptance
evidence. Ordinary commit requests retain all gates and never imply integration.

A request to commit, including a commit required by roadmap closure, delegates
commit partitioning to the agent. Inspect available session/task records, pending
diffs and recent history to identify the coherent changes within the authorized
scope. Split independent fixes or features into separate commits automatically;
one coherent change may remain one commit. Session boundaries are evidence, not
a requirement to separate tightly coupled work. Resolve shared files by hunks
and order dependent commits so each is coherent. Do not include unrelated work
merely because it is pending. Report the planned groups briefly, without asking
for routine partition approval, then list the resulting commit count, IDs and
scopes at completion. An explicit user-selected scope or partition takes priority.

Commits remain mandatory when closing a minor roadmap point or major milestone.
Do not label ordinary work as roadmap completion to trigger a commit. Outside
those closure points, the user controls commit timing.

## Prepare One Stable Batch

1. Select sessions, paths/hunks and commit groups, including any
   requested milestone scope. Session provenance guides grouping; a shared dirty
   directory does not establish authorship. Resolve dependencies and integration
   conflicts now. Use separate worktrees for simultaneous writers only after the
   user's location choice; follow
   [merge policy](git-merge-commits.md) for divergent integration.
2. Establish a candidate containing exactly that batch. The verifier executes the
   working tree, not just staged files. Isolate mixed or actively changing work
   using [git-worktree](../skills/git-worktree/SKILL.md) after that choice; preserve unrelated source,
   staged entries and history. Compare pending changes against the current `HEAD`.
   For local integration, compare the complete candidate against the recorded
   immutable primary-branch base instead. Later edits belong to a later batch.
3. Perform [specification compliance](../skills/verify-spec-compliance/SKILL.md),
   compatible documentation enrichment, necessary refactoring, language resolution
   and applicable [coverage review](unit-testing-policy.md#coverage-during-development-and-review).
   Prepare every commit's diary entry, ordering and English Conventional Commit
   message before starting expensive verification. Check the prepared partition
   with lightweight Git comparisons, including `git diff --check`, first.
   Each coherent concern includes its implementation, tests, documentation
   and diary entry; do not split a feature merely by file category. Record checks
   already observed in the diary, and the final profile outcome in commit messages
   and the completion response. Do not rewrite the diary just to repeat that outcome.
4. Arrange the four independent [review lanes](../skills/code-review/SKILL.md),
   each with a separate `code_reviewer` started without inherited conversation
   history. Together they cover the cumulative change, affected interactions and
   proposed partition. Start lanes concurrently within available slots. Review may
   begin before final tests. Route corrections only to affected lanes and carry
   unchanged scoped approvals under the skill's comparison procedure without
   re-invoking their reviewers. All four applicable verdicts must cover the final
   content and evidence; a changed global fingerprint alone does not reopen a lane.
   Author self-review is not a substitute.
5. Finish any graph refresh that changes candidate content. Resolve cheap blockers
   and pending requirement decisions before expensive tests where possible. Run
   the canonical profile for the entire candidate. Use its preliminary gates;
   do not add a duplicate standalone language pass by default. Continue after
   corrections under the evidence rules below. Run milestone verification and
   closure procedures only at their own [trigger](roadmap-maintenance.md).
6. Measure [artifact storage](../references/agent-tooling.md#local-artifact-storage)
   once after builds and diagnostics, before freeze. Record observation time and
   limitations; reuse that observation in the completion response.
7. Confirm all four applicable scoped **APPROVED** verdicts, renewed or validly carried,
   all required passing evidence, exact
   candidate and complete commit partition. Freeze them together.

After candidate edits stop, overlap independent work: start the independent review lanes,
the required Graphify refresh and the canonical verifier in the background when
their outputs are isolated. Graphify may overlap verification when it writes only
ignored `graphify-out/` artifacts; finish it before freeze. Review may inspect the
stable candidate while checks run, then assess their final evidence. Await every
required result and inspect its exit status; background launch is not completion.
Do not edit validated inputs, run competing graph writers, or launch duplicate
test commands beside a canonical profile that already selects them. Serialize
commands sharing Cargo targets or coverage reports. Limit Graphify extraction
workers when it competes with compilation, and compare elapsed time before
claiming an improvement. The verifier's internal groups currently run serially;
subagents do not change that scheduling or replace its final verdict.

Compatible documentation and in-scope audit corrections belong to this same
preparation; they do not create a second approval channel. Optional behavior,
requirement changes, protected checks and references retain their existing
approval boundaries. Broader milestone audits can satisfy overlapping scoped
work when their coverage and candidate match.

## Canonical Verification and Continuation

Use native Git and the existing verifier. The complete required check set comes
from all pending changes against `HEAD`, or the complete integration candidate
against its explicit immutable base; reruns come from each successful group's
inputs. These comparisons must remain separate.

```powershell
python tools/verification/verify.py commit
```

The [verification reference](../../tools/verification/README.md#completion-verification)
owns record format, dependency boundaries and fallback behavior. Repeat the same
command after corrections; it automatically checks evidence from the current
batch. A changed `HEAD` starts a new batch. A diary-only correction reruns its
language check and final aggregation while retaining valid independent results;
other documents remain conservative inputs. After an interruption, repeat the
same command and results directory to retain matching successful groups and
rerun unfinished work. The checkout lock excludes another live verifier; surviving
children after verifier termination are outside that guarantee.
The verifier alone issues the complete final verdict. Reuse only evidence
from this batch with matching inputs, commands, execution conditions, acceptance
criteria and intact artifacts. Coverage execution, export and summary are one
bundle. Failed, missing, interrupted or invalidated groups must execute; approval,
a question or a resumed conversation is neither a pass nor an invalidation.

Integration uses `commit --integration-base <commit>`; its complete comparison
scope is separate from HEAD/source/index stability. After freeze, the integration
tool validates the prepared partition against the resulting commits as a narrow
mechanical bridge. This does not permit general evidence reuse across changed HEAD.

Content changes return the batch to preparation. Rerun affected checks and update
review; retain demonstrably valid independent results. Unknown impact requires
conservative execution, up to the full profile. Never waive a failed gate with a
focused pass or explanation, including unrelated or environment failures. Missing
independent review also blocks freeze. Report the blocker and leave work
uncommitted when it cannot be resolved.

## Mechanical Completion After Freeze

- Create only the planned commits, each with its already prepared diary entry.
  Validate the intended index and tree using lightweight native Git comparisons.
  Stage selected paths explicitly; never sweep unrelated edits into the batch.
- Do not edit diary, documentation or code between commits. Detect interference
  before each operation, and compare the final tree with the frozen candidate.
  Partial failure stops the sequence; retain completed commits and pending work
  without automatic reset or history rewriting. For integration, seal the exact
  frozen result before conditional publication through the owning merge procedure.
- Evidence applies to the combined final candidate. Do not describe intermediate
  commits as individually tested unless that verification was explicitly required
  and completed during preparation.
- Report commit IDs and the collected verification, review and storage results.
  Do not repeat tests, audits, cleanup or storage scans after commits as routine
  completion. Deletion and pushing require their own authorization.

The verifier owns deterministic architecture and code-quality gates. Invoke the
manual [audit-code-quality](../skills/audit-code-quality/SKILL.md) only on explicit
request or for a major branch merge. Use no hooks, wrappers, session database or
cross-task verdict cache. Keep diagnostic artifacts under `.agent/tmp`.
