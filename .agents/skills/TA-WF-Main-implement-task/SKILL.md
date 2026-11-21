---
name: TA-WF-Main-implement-task
description: Implement a selected task plan with tests, independent review and challenged findings; allow two corrective passes with targeted reassessment and a final full review after repairs, leaving unresolved issues for manual decision.
---

# Implement and review the selected task

Apply the [result contract](../../references/workflow-contract.md). This compound
operation runs in the selected task worktree. Inputs: the plan, agreed scenarios,
requirements, write scope, available execution context and any supplied limits.

The main session coordinates. Delegate each implementation or corrective pass to
`TA-implementer` using the [subagent policy](../../../AGENTS.md#subagent-assignment-and-model-selection).
Resolve model and effort for the actual assignment before each launch, including
repairs. Assign independent reviews and selected command/coverage execution to
their respective roles under the same policy.

1. Read the working plan and relevant requirements. Execute
   [TA-WF-Step-implement-change](../TA-WF-Step-implement-change/SKILL.md) for the assigned code,
   tests, necessary Design documentation and focused checks. Reuse its approved
   repair/optimization modes only where they belong to the task.
2. Run review round 1 through
   [TA-WF-Step-review-code](../TA-WF-Step-review-code/SKILL.md) on the actual task changes and
   affected interactions, providing requirements and actual check results. Select
   all applicable reviewer assignments and retain their scopes for the final
   review. Review is independent of the author and does not modify the files.
   If coverage assessment is part of the assignment, give existing applicable
   reports to the tests reviewer or have `TA-test-runner` collect the selected
   scope through [TA-WF-Step-measure-coverage](../TA-WF-Step-measure-coverage/SKILL.md) first.
   Missing measurement remains an explicit gap, not an inferred missing test.
3. Collect reviewer findings and reported test/verification failures with their
   actual evidence. Run
   [TA-WF-Step-challenge-findings](../TA-WF-Step-challenge-findings/SKILL.md) in a fresh
   independent `TA-code-reviewer` context before selecting repairs. Preserve finding
   IDs, upheld/rejected claims and missing evidence. A challenge verdict does not
   turn a failed or missing required check into a pass.
4. If round 1 and its challenge leave no required issues, finish without further
   review rounds. Otherwise make up to two corrective passes for in-scope findings
   and reproduced test/verification failures upheld by the challenge:

   - Make the first corrective pass through `TA-WF-Step-implement-change` and
     repeat affected checks.
   - Run review round 2 as a targeted reassessment through `TA-WF-Step-review-code`:
     select only the round 1 reviewer assignments whose findings were upheld by
     the challenge and selected for repair. Reassess their findings, corrections
     and affected interactions in fresh independent contexts. If there are no
     such assignments, assess the rerun checks without adding reviewer assignments.
     Repeat step 3 on the resulting findings and test/verification failures.
   - If that assessment leaves supported in-scope issues, make the second
     corrective pass and repeat affected checks. Then proceed to the final review.

   The initial implementation and assessment do not count as a corrective pass.
   New findings or failures do not reset the count. Missing requirements,
   protected-change approval or work beyond the selected scope returns to the user.
   Corrective passes can include meaningful test cases for already approved
   behavior based on the tests reviewer's coverage/assertion findings. Refresh a
   selected coverage measurement when needed to assess the added tests against
   the changed source.
5. Whenever corrective work was needed after round 1, run final review round 3
   through `TA-WF-Step-review-code` on the complete task change, including all
   corrections and affected interactions. Include every round 1 reviewer
   assignment, even those without findings, and any newly affected review scope.
   Supply current check/coverage results and repeat step 3 on the final findings.
   This full review is required even when round 2 found no remaining issues.
   If required findings, failed checks or required evidence gaps remain, stop for
   the user's manual decision. Do not start another corrective pass or review
   round, even if only one corrective pass was used, or report the task as verified.
6. Return changed paths, actual tests/reviews, challenge decisions, completed
   review rounds and corrective passes, repairs and remaining work in text and
   JSON. State when a manual decision is needed. Leave source, tests and
   documentation as working files. Commit-time English checks belong to the next
   selected commit operation, unless explicitly included here. Ordinary task
   implementation does not require a diary entry.

Stop before staging, commits, merge, worktree removal or final master verification.
Do not synchronize master into the task branch as a review prerequisite.
