---
name: TA-WF-Main-commit-changes
description: Partition selected changes into meaningful commits with English checks, then integrate the task branch into local master by fast-forward when possible or merge otherwise; also support scoped commits already on master.
---

# Commit the selected changes and merge to master

Apply the [result contract](../../references/workflow-contract.md),
[commit preparation criteria](references/commit-preparation.md) and shared
[Git procedure](../TA-WF-Main-close-task/references/local-integration.md#authorization-and-preparation).
Inputs: selected task checkout/branch, primary checkout, delivered scope, plan when available and
available check results. Selecting this stage authorizes its scoped staging,
commits and local merge. It does not authorize a push or cleanup.

1. Inspect selected changes and existing commits. Group unfinished work into one
   or several logical commits: keep a behavior change with its tests and required
   documentation; separate independent operations where that helps understanding.
   Do not force a single commit, split mechanically by file or rewrite existing
   history without that authorization. Preserve unrelated files and staged versions.
2. Apply [TA-WF-Step-correct-english](../TA-WF-Step-correct-english/SKILL.md) to the selected authored
   text, including selected documentation changes, using the existing scanner's scope
   and exclusions. Correct clear findings and justified vocabulary. A required
   unresolved language finding returns its gap; do not invent a passing check.
   Review `git diff --check` and the selected diff, then stage and create the
   meaningful English commits while preserving the unrelated index.
   A commit does not require a diary entry. Include diary changes only when they
   are already part of the selected work or explicitly requested.
3. In the primary checkout on local `master`, integrate the selected task branch
   using [the Git procedure](../TA-WF-Main-close-task/references/local-integration.md#authorization-and-preparation):
   fast-forward when possible, otherwise merge the diverged histories.
   Do not merge master into the task branch to run tests first, build
   an assembly branch or freeze a source version. Resolve ordinary in-scope merge
   conflicts using [the merge procedure](../TA-WF-Main-close-task/references/local-integration.md#merge-conflicts); a genuine
   behavior decision or unsafe interference stops the dependent work. Reconcile
   overlapping diary/text changes and rerun the scoped language check if authored
   text changes during conflict resolution.
4. Return the created commits, included scope, integration outcome, check results and
   remaining work. When already operating on `master` for selected final-review
   corrections, create the scoped corrective commits there and omit self-merge.
   Do not claim that a local merge or commit proves tests passed on master.

Stop with committed work on master or an explained partial failure. Do not run a
verification/review campaign, delete the worktree/plan/branch or start the final
master stage. Those are separately selected operations.
