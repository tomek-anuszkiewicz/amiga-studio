---
name: TA-prepare-batch
description: Prepare an explicitly selected candidate, commit partition, messages and diary content while exposing missing readiness evidence.
---

# Prepare the selected batch

Apply [the invocation contract](../../references/workflow-contract.md) and
[batch policy](../../rules/git-commits.md).
Executor: main session; result kind: `batch`.
Inputs: authorized scope, exact baseline/candidate, desired completion boundary,
available reports and any selected integration/closure records.

1. Select coherent paths/hunks and explicit new files, separating unrelated work and
   existing staged versions. Establish the actual candidate that checks execute.
   Record the immutable integration base/source set when applicable.
   For an adopted capture baseline, use the [approved-input procedure](../../../tools/workflow/capture/README.md#starting-inputs)
   and identify the selected inventory, scenarios and attachment bytes.
2. Prepare a coherent partition and English Conventional Commit messages. Use
   native Git/temporary-index mechanics from the
   [integration reference](../../../tools/verification/README.md#local-integration)
   when exact intermediate trees are needed. Keep each concern with its tests,
   documentation and diary. Honor any authorized partition, stage only its
   selected scope and preserve unrelated staged versions.
3. Stage a prepared entry in the final, unnumbered `## Rolling Changelog`
   of [DIARY.md](../../../DIARY.md) for every ordinary or final integration
   commit. Record components, changes, rationale, observed checks, failures
   and work left unrun with the task or closure identity. The
   [working and assembly exceptions](../../rules/git-merge-commits.md#authorization-and-preparation)
   record unverified status in the final closure entry. Keep the diary stable
   between frozen commits and during publication. Put the final verification
   outcome in the commit messages and completion report.
   Prepare the closure entry before freeze and the post-publication English scan.
   Retain the selected task and execution IDs from the existing capture
   registration for publication and later preservation.
   The vocabulary-only closure commit needs no additional diary entry. Report
   missing compaction, pruning, documentation,
   coverage, semantic review or milestone work as explicit prerequisites.
4. Record one [storage observation](../../references/agent-tooling.md#local-artifact-storage)
   after builds and diagnostics and before freeze. Collect supplied reports and
   four independent lane results, preserving original
   identities and any scoped carry comparisons. Identify missing, stale or failed
   requirements. Run lightweight partition/diff checks such as `git diff --check`.
5. Return candidate, partition/message artifacts, prepared records and remaining
   prerequisites. Preparation can complete while readiness remains false.

Stop after preparation. Do not dispatch audits/reviewers, run broad tests, freeze,
create commits, publish or clean up. The caller selects needed subsequent work.
