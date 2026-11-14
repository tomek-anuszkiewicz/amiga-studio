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

Commits remain mandatory when closing a minor roadmap point or major milestone.
Do not label ordinary work as roadmap completion to trigger a commit. Outside
those closure points, the user controls commit timing.

## Prepare One Stable Batch

1. Select sessions, paths/hunks and commit groups, including any
   requested milestone scope. Session provenance guides grouping; a shared dirty
   directory does not establish authorship. Resolve dependencies and integration
   conflicts now. Prefer separate worktrees for simultaneous writers; follow
   [merge policy](git-merge-commits.md) for divergent integration.
2. Establish a candidate containing exactly that batch. The verifier executes the
   working tree, not just staged files. Isolate mixed or actively changing work
   using [git-worktree](../skills/git-worktree/SKILL.md); preserve unrelated source,
   staged entries and history. Compare pending changes against the current `HEAD`.
   Later edits belong to a later batch.
3. Perform [specification compliance](../skills/verify-spec-compliance/SKILL.md),
   compatible documentation enrichment, necessary refactoring, language resolution
   and applicable [coverage review](unit-testing-policy.md#coverage-during-development-and-review).
   Prepare every commit's diary entry, ordering and English Conventional Commit
   message. Each coherent concern includes its implementation, tests, documentation
   and diary entry; do not split a feature merely by file category. Record checks
   already observed in the diary, and the final profile outcome in commit messages
   and the completion response. Do not rewrite the diary just to repeat that outcome.
4. Arrange one independent [code-review](../skills/code-review/SKILL.md) by a
   separate `code_reviewer` started without inherited conversation history. It
   covers the cumulative change, affected interactions and proposed partition.
   Review may begin before final tests to find corrections early. Reuse this
   reviewer's context for corrections; its final verdict must cover final content
   and current evidence. Author self-review is not a substitute.
5. Finish any graph refresh that changes candidate content. Resolve cheap blockers
   and pending requirement decisions before expensive tests where possible. Run
   the canonical profile for the entire candidate. Use its preliminary gates;
   do not add a duplicate standalone language pass by default. Continue after
   corrections under the evidence rules below. Run milestone verification and
   closure procedures only at their own [trigger](roadmap-maintenance.md).
6. Measure [artifact storage](../references/agent-tooling.md#local-artifact-storage)
   once after builds and diagnostics, before freeze. Record observation time and
   limitations; reuse that observation in the completion response.
7. Confirm the final **APPROVED** verdict, all required passing evidence, exact
   candidate and complete commit partition. Freeze them together.

Compatible documentation and in-scope audit corrections belong to this same
preparation; they do not create a second approval channel. Optional behavior,
requirement changes, protected checks and references retain their existing
approval boundaries. Broader milestone audits can satisfy overlapping scoped
work when their coverage and candidate match.

## Canonical Verification and Continuation

Use native Git and the existing verifier. The complete required check set comes
from all pending changes against `HEAD`; reruns come from each successful group's
inputs. These comparisons must remain separate.

```powershell
python tools/verification/verify.py commit
```

The [verification reference](../../tools/verification/README.md#completion-verification)
owns record format, dependency boundaries and fallback behavior. Repeat the same
command after corrections; it automatically checks evidence from the current
batch. A changed `HEAD` starts a new batch. Documents remain conservative inputs.
The verifier alone issues the complete final verdict. Reuse only evidence
from this batch with matching inputs, commands, execution conditions, acceptance
criteria and intact artifacts. Coverage execution, export and summary are one
bundle. Failed, missing, interrupted or invalidated groups must execute; approval,
a question or a resumed conversation is neither a pass nor an invalidation.

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
  without automatic reset or history rewriting.
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
