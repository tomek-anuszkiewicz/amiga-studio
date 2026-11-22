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

Apply [vocabulary-only maintenance](../../references/workflow-contract.md#vocabulary-only-maintenance)
throughout preparation and integration. Those additions do not initiate or repeat
language scans or other verification. Continue the already selected Git operation;
other authored-text changes retain their language-check requirements.

1. Inspect the selected changes and existing commits. Group unfinished work
   into one or more commits with a clear purpose. Keep a behavior change with
   its tests and required documentation. Give independent operations their own
   commits when that helps the reader understand them.
   Do not force a single commit, split mechanically by file or rewrite existing
   history without that authorization. Preserve unrelated files and staged versions.
2. Apply [TA-WF-Step-correct-english](../TA-WF-Step-correct-english/SKILL.md) to the selected authored
   text, including selected documentation changes, using the existing scanner's scope
   and exclusions. Correct clear findings and address justified vocabulary
   additions. If a required language finding remains unresolved, report the gap
   instead of claiming the check passed. Review `git diff --check` and the selected
   diff, then stage the selected changes and create meaningful commits with English
   messages. Preserve unrelated index entries.
   A commit does not require a diary entry. Include diary changes only when they
   are already part of the selected work or explicitly requested.
3. In the primary checkout on local `master`, integrate the selected task branch
   using [the Git procedure](../TA-WF-Main-close-task/references/local-integration.md#authorization-and-preparation):
   fast-forward when possible, otherwise merge the diverged histories.
   Do not merge master into the task branch to run tests first, build
   an assembly branch or freeze a source version. Use
   [the merge procedure](../TA-WF-Main-close-task/references/local-integration.md#merge-conflicts)
   to resolve ordinary conflicts within scope. Stop the dependent work if
   a behavior decision is needed or interference makes integration unsafe.
   Reconcile overlapping diary and other text changes, and rerun the scoped
   language check if conflict resolution changes authored text.
4. Return the commits created, scope included, integration outcome, check results
   and remaining work. When working directly on `master` for selected
   final-review corrections, commit just those corrections there and skip
   the merge.
   Report passing tests on master only when there is test evidence for that claim.

Stop with committed work on master or an explained partial failure. Do not run a
verification/review campaign, delete the worktree/plan/branch or start the final
master stage. Those are separately selected operations.
