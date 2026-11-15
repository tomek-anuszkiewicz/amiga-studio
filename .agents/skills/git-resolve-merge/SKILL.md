---
name: git-resolve-merge
description: Prepare integration on the task branch, conditionally publish the reviewed commit, and perform authorized cleanup.
---

# Local Branch Integration

Follow [task location, authorization and integration rules](../../rules/git-merge-commits.md).
A merge-only request does not authorize cleanup; complete local integration does.
Use the selected task checkout for synchronization and preparation. Primary-branch
implementation uses ordinary batch preparation without self-integration.

## 1. Record the Baseline and Synchronize

Inspect native Git state in both registered checkouts. Resolve the primary branch
and record its immutable commit as `B`; do not repeatedly merge a moving name.
Preserve unrelated work and index entries. If pending changes prevent a safe
merge, follow the owning rule's prerequisite-commit or scoped-decision procedure.

In the task checkout, merge exactly `B`. For divergent histories:

```powershell
git merge --no-ff --no-commit <B>
```

For a simple synchronization, use `git merge --ff-only <B>`. This synchronization
does not publish the task result. Preserve real divergent merge parents.

## 2. Reconcile the Complete Result

For divergent integration, identify the common base of the original source tip
and `B`. Inventory the union of tracked Markdown paths changed on both sides:

```powershell
git diff --name-only --no-renames -z <common-base> <original-source-tip> -- '*.md'
git diff --name-only --no-renames -z <common-base> <B> -- '*.md'
```

Review every inventoried path, including additions, deletions, moves and files
without textual conflicts. Compare common-base, both branch versions and candidate
content against the integrated code. Reconcile features, tests, instructions,
links and diary decisions; stage only authorized resolutions. Keep a per-path
checklist under `.agent/tmp/`. Record scope and substantive decisions in the diary
and prepared merge message, including resolution, Markdown review and verification.

## 3. Prepare, Verify and Freeze

Prepare the complete candidate, partition, messages, compliance review, applicable
coverage review, diary and fresh-context independent review through
[batch preparation](../../rules/git-commits.md). Run:

```powershell
python tools/verification/verify.py commit --integration-base <B>
```

This includes already committed task changes. Divergent integration retains full
ordinary-workspace tests through coverage, without another uninstrumented run.
Milestone checks retain their own triggers.

Follow the [tool reference](../../../tools/verification/README.md#local-integration)
to record the review and prepared trees, freeze, make only the prepared commits,
then seal their exact mechanical consumption. Do not rerun expensive verification
solely because those commits change HEAD. A pre-existing clean committed result
can use an empty new-commit partition.

## 4. Publish and Complete the Authorized Scope

Run `integrate publish` using the sealed record. The operation conditionally
publishes exactly `B -> C` while advancing the target checkout under native
reference locks. Do not substitute an ordinary target-side merge. If the target
advances, merge its new immutable tip into the task branch and prepare the new
combined result. If publication is interrupted, use `integrate recover` from a
surviving checkout; unexpected edits require a decision and remain preserved.

For complete integration, run `integrate cleanup` from the surviving target
checkout. It preserves local material and removes only the unchanged integrated
source checkout and branch. A merge-only request stops after publication. Report
publication and cleanup separately; retry failed cleanup using its retained
record without repeating verification or creating replacement commits.