---
name: TA-WF-Main-verify-master
description: Verify integrated local master with English checks, tests and challenged independent review; commit up to two corrective passes, use targeted reassessment and final full review after repairs, and return unresolved issues for manual decision.
---

# Verify the integrated change on master

Apply the [result contract](../../references/workflow-contract.md).
Inputs: primary checkout on `master`, delivered change scope and requirements,
requested test selection and available task/plan context. Run before task closure
so the working plan and task resources remain available until final checks pass.
For a resumed assessment whose worktree is already absent, use retained context
or delivered Git changes; do not guess the task or recreate a checkout.

The main session coordinates. Apply the
[subagent policy](../../../AGENTS.md#subagent-assignment-and-model-selection)
to each runner, reviewer and corrective implementation assignment. Re-evaluate
model and effort for each correction instead of reusing earlier settings by habit.

1. Inspect master and preserve unrelated files/index entries. Tests and review run
   directly here. Do not copy master to another branch or compare it with a frozen
   baseline. Report interference that prevents testing the intended change.
2. Run [TA-WF-Step-correct-english](../TA-WF-Step-correct-english/SKILL.md) on the
   delivered authored text as it now exists on master, using the scanner's scope
   and exclusions. This check is required even when integration had no conflicts
   and no implementation repairs are needed. If it changes files, use
   [TA-WF-Main-commit-changes](../TA-WF-Main-commit-changes/SKILL.md) on master to
   commit only those selected corrections, omitting merge. Unresolved required
   language findings or scanner failures leave the final assessment incomplete.
3. Assign the requested tests, quality checks and selected coverage measurements
   to `TA-test-runner` in this master checkout, using the existing verifier.
   If no narrower test selection was supplied, use
   [TA-WF-Step-verify-commit](../TA-WF-Step-verify-commit/SKILL.md) for the ordinary `commit` check
   profile. This profile does not create commits. Full milestone vectors remain
   outside this stage unless expressly requested.
   On a committed clean master, working-file detection cannot discover already
   merged tool changes. Inspect the profile selection and add explicit affected
   Python/bootstrap test directory commands directly where not already covered,
   using the [verifier reference](../../../tools/verification/README.md).
   Do not treat an empty `test python` selection as coverage of delivered changes.
4. Run review round 1 through
   [TA-WF-Step-review-code](../TA-WF-Step-review-code/SKILL.md) on the delivered changes and
   interactions as they now exist on master, with the actual test results. Select
   all applicable reviewer assignments and retain their scopes for the final review.
5. Collect reviewer findings and reported test/verification failures with their
   actual evidence. Run
   [TA-WF-Step-challenge-findings](../TA-WF-Step-challenge-findings/SKILL.md) in a fresh
   independent `TA-code-reviewer` context before selecting repairs. Preserve finding
   IDs, upheld/rejected claims and missing evidence. A challenge verdict does not
   turn a failed or missing required check into a pass.
6. If round 1 and its challenge leave no required issues, finish without further
   review rounds. Otherwise make up to two corrective passes for findings and
   reproduced test/verification failures upheld by the challenge within the
   delivered scope:

   - Make the first corrective pass through
     [TA-WF-Step-implement-change](../TA-WF-Step-implement-change/SKILL.md) and
     repeat relevant checks.
   - Run review round 2 as a targeted reassessment through `TA-WF-Step-review-code`:
     select only the round 1 reviewer assignments whose findings were upheld by
     the challenge and selected for repair. Reassess their findings, corrections
     and affected interactions in fresh independent contexts. If there are no
     such assignments, assess the rerun checks without adding reviewer assignments.
     Repeat step 5 on the resulting findings and test/verification failures.
   - If that assessment leaves supported in-scope issues, make the second
     corrective pass and repeat relevant checks. Then proceed to the final review.

   Work directly on master. For each corrective pass use
   [TA-WF-Main-commit-changes](../TA-WF-Main-commit-changes/SKILL.md) there to create
   the corrective commits and repeat English checks, omitting merge, before the
   next review. The initial assessment does not count as a corrective pass.
   New findings or failures do not reset the count.
   Do not weaken checks, change approved behavior or start another task to obtain
   a pass.
   Coverage findings can justify test additions for approved behavior. Refresh
   the affected coverage measurement before claiming those protection gaps closed;
   do not use an old report for changed code/tests.
   Missing requirements, protected-change approval or work beyond the delivered
   scope returns to the user.
7. Whenever corrective work was needed after round 1, run final review round 3
   through `TA-WF-Step-review-code` on the complete delivered change as it now
   exists on master, including all corrections and affected interactions. Include
   every round 1 reviewer assignment, even those without findings, and any newly
   affected review scope. Supply current check/coverage results and repeat step 5
   on the final findings. This full review is required even when round 2 found no
   remaining issues. If required findings, failed checks or required evidence gaps
   remain, stop for the user's manual decision. Do not start another corrective
   pass or review round, even if only one corrective pass was used.
8. Return actual language checks, tests, reviewer findings, challenge decisions,
   completed review rounds and corrective passes, corrections/commits, report
   locations and unresolved work in text and JSON. State when a manual decision
   is needed. A failed or incomplete final assessment stays failed/partial even
   though integration already happened. Task closure remains blocked by unresolved
   required checks/findings. Do not claim to have rolled back the delivered work
   or start cleanup automatically.

Stop after the final assessment. Do not push, recreate/delete
worktrees or close a milestone. Further repair work returns to the user.
