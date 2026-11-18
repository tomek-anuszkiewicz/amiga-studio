# Batch authority and readiness

## Authorization and Ordinary Completion

Ordinary implementation, fixes, review and documentation authorize scoped changes
and relevant focused checks. They leave work unstaged and do not automatically
start coverage, language scans, graph refresh, synchronization, broad audits or
closure. Reuse context and evidence while inputs remain valid; handle requirement
conflicts immediately.

A batch-preparation request authorizes staging its selected scope, not commits.
A commit request authorizes preparation and its commits. Preserve unrelated index
entries, including staged versions of files changed later. Existing authorization
persists through routine corrections; it does not grant protected changes,
integration, cleanup or push. A request may explicitly cover several operations;
narrow skills never infer the rest of the lifecycle. The
[checkout choice](git-merge-commits.md#choose-the-task-checkout) remains required.

Commits are mandatory for actual minor-roadmap or major-milestone closure; ordinary
work is not closure. The user otherwise controls commit timing. Partition coherent
concerns with their implementation, tests, documentation and diary; user-selected
scope/partition takes precedence. Session provenance is evidence, not ownership of
all dirty paths. Integration's unverified working/intermediate-merge and unchanged
tested-tip exceptions remain narrowly owned by [integration policy](git-merge-commits.md).

## Readiness Before Freeze

The caller selects preparation, required assessment, verification and repairs as
separate operations. [TA-prepare-batch](../skills/TA-prepare-batch/SKILL.md) establishes
the exact candidate, partition/messages, authorized staging and diary/closure content;
missing prerequisites are reported rather than launched. Readiness requires:

- The full selected candidate, including new files and all intended hunks, isolated
  from unrelated/concurrent work. The verifier runs the working tree, not just the
  index. Ordinary comparison is against HEAD; integration compares the whole result
  against immutable target base `B`.
- Scoped specification compliance, compatible documentation enrichment, resolved
  language findings and applicable changed-code coverage assessment. All in-scope
  required gaps must be resolved, including new scenarios selected by the caller.
- Every prepared commit's diary entry, ordering and English Conventional Commit
  message before final checks/freeze. Record only already observed checks in the
  diary; final profile outcome belongs in the commit message and final report.
- Four fresh independent `TA-code-reviewer` instances covering behavior,
  architecture, tests/evidence and documentation. Use their fixed lane skills and
  [review contract](../references/review-common.md), with no inherited author history.
  Available slots may require waves. Author self-review or a prior audit is no substitute.
- Passing canonical commit evidence for the combined final candidate, plus milestone
  evidence only at its actual trigger. Failed, missing, interrupted or invalidated
  gates block freeze regardless of focused passes or environmental explanations.
- Applicable project-only Graphify freshness, completed candidate edits and one
  [storage observation](../references/agent-tooling.md#local-artifact-storage) after
  builds/diagnostics and before freeze. Reference graphs never refresh implicitly.

Independent checks/reviews may overlap after candidate edits stop when their
outputs and resources are isolated. Serialize shared Cargo/coverage targets;
await results and exit statuses. A background launch is not completion. The
verifier owns its internal scheduling; do not dispatch duplicate selected suites.

## Canonical Verification and Continuation

[TA-verify-batch](../skills/TA-verify-batch/SKILL.md) executes
`python tools/verification/verify.py commit`; integration adds
`--integration-base <immutable-commit>`. The
[verification reference](../../tools/verification/README.md#completion-verification)
owns selection, artifacts and evidence validity. Repeat the same command/results
location after corrections or interruption: successful matching groups can remain,
failed/missing/invalidated groups must run. HEAD changes start a new ordinary batch;
only the existing frozen mechanical-commit protocol bridges accepted integration
commits. Coverage execution/export/summary form one evidence bundle.

Content corrections return the candidate to preparation and affected checks/review.
Carry an unchanged scoped approval only after comparing its relevant content,
requirements, criteria, partition and evidence, retaining the original identity and
comparison basis. A global fingerprint change alone does not reopen every lane.
Unknown impact requires conservative rechecking. Never convert confirmation,
a resumed conversation or a report-format repair into verification evidence.

## Mechanical Completion After Freeze

Freeze the complete candidate/partition/messages, required passing evidence and all
four applicable approvals together. [TA-commit-batch](../skills/TA-commit-batch/SKILL.md)
creates only authorized prepared commits and seals readiness where applicable;
it does not repair missing prerequisites. Use lightweight native Git comparisons
for index/tree integrity, stage selected scope only, and detect interference before
each operation. No content or diary edits between frozen commits. Partial failure
preserves completed commits and pending work without automatic reset or rewriting.

Evidence covers the combined result, not each intermediate commit unless separately
verified. Report actual IDs, checks/reviews and retained storage observation. Do not
repeat tests, audits or storage scans merely because mechanical commits finished.
Publication and cleanup remain separate authorized operations.

## Execution-Plan Lifecycle

Keep `.agent/tasks/` plans while active, paused or blocked. Stable task IDs identify
work across copies and continuations. On verified completion or explicit closure,
preserve delivered scope, decisions, actual verification and limits in the next
required DIARY/Git closure record. Transfer remaining work to an active plan or
roadmap unless explicitly waived. Retain the plan until its closure record is
committed; authorized cleanup then removes only unchanged copies of that closed
plan. Closure/deletion never turns failed or unrun checks into passes. No permanent
plan archive, hooks, session database or cross-task verdict cache is introduced.
