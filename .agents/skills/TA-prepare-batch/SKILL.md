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
   For an adopted capture baseline, use the [approved-input policy](../../rules/git-commits.md#approved-execution-inputs)
   and identify the selected inventory, scenarios and attachment bytes.
2. Prepare a coherent partition and English Conventional Commit messages. Use
   native Git/temporary-index mechanics from the
   [integration reference](../../../tools/verification/README.md#local-integration)
   when exact intermediate trees are needed. Stage only explicitly authorized scope.
3. Prepare required diary entries with observed results and task/closure identity.
   Keep all content before freeze. Missing compaction, pruning, documentation,
   coverage, semantic review or milestone work becomes an explicit prerequisite,
   not an automatically invoked operation.
4. Collect supplied reports and four independent lane results, preserving original
   identities and any scoped carry comparisons. Identify missing, stale or failed
   requirements. Run lightweight partition/diff checks such as `git diff --check`.
5. Return candidate, partition/message artifacts, prepared records and remaining
   prerequisites. Preparation can complete while readiness remains false.

Stop after preparation. Do not dispatch audits/reviewers, run broad tests, freeze,
create commits, publish or clean up. The caller selects needed subsequent work.
