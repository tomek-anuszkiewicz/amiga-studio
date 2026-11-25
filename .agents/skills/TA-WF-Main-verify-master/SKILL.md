---
name: TA-WF-Main-verify-master
description: Verify delivered work on local master with full English scans, tests with coverage, required gates and challenged independent review, with up to two scoped corrective passes.
---

# Verify the delivered change on master

Apply the [result contract](../../references/workflow-contract.md). The main
session coordinates the selected assessment directly in the primary checkout on
`master`. Identify the delivered changes, approved requirements, applicable
pending files, requested test selection and any available plan and task context.
Preserve unrelated working files and index entries. If task resources are
unavailable, use retained evidence or the delivered Git changes and report any
missing inputs. Use the [subagent policy](../../../AGENTS.md#subagent-assignment-and-model-selection)
for each assignment, including repairs.

[Direct vocabulary additions](../../references/workflow-contract.md#vocabulary-only-maintenance)
alone start no scan, checks, review or corrective pass. Follow the steps below
for the assessment independently required by the delivered change.

## Prepare the evidence for each review

1. Before each review round, including the final full review on unchanged
   inputs, run [TA-WF-Step-correct-english](../TA-WF-Step-correct-english/SKILL.md)
   on master. Use its full-checkout scan and validated dictionary maintenance
   even when the delivered change contains no eligible text. After the
   language operation, use
   [TA-WF-Main-commit-changes](../TA-WF-Main-commit-changes/SKILL.md) to commit
   authorized language and vocabulary edits and the selected corrective
   changes on master. Preserve unrelated edits and omit merge. Unresolved
   required language findings or scan errors leave readiness unverified.
2. For the initial round, have `TA-test-runner` run the applicable tests and
   gates directly on master, even when implementation tests passed. Use
   [TA-WF-Step-verify-commit](../TA-WF-Step-verify-commit/SKILL.md) and its
   `commit` profile unless the engineer selected a narrower scope. The profile
   runs the full inventory on the current checkout, including every registered
   Python suite. Use those results for the initial review. Use the delivered
   changes and applicable pending files to scope the review, narrower checks
   selected by the engineer, and tests for separately required behavior.
   Run the full milestone vectors here only when expressly selected.
   [Verifier profiles](../../../tools/verification/README.md#completion-verification)
   · [Working-tree decision RD-002](../../references/review-decisions.md#rd-002-uncommitted-changes-during-final-verification)
3. Collect coverage in the same run as ordinary Rust tests. The `commit`
   profile already instruments the ordinary workspace. For planned whole-package
   or workspace tests outside that profile, use `test coverage crate <name>`
   or `test coverage workspace` through the verifier. Use the normal command
   for selected harness, Python and expressly filtered tests and report
   their instrumentation limits. Record command, checkout, test scope, status,
   reports and exclusions. A missing required export leaves assessment
   incomplete even after a test pass. Prose changes can have applicable
   document checks with no executable change to measure.
   [Coverage commands and reports](../../../tools/verification/README.md#coverage-guided-testing)
4. After each repair, repeat the failed and affected tests and other affected
   gates after the English operation and scoped commits. Use coverage in the
   same run where supported. Coordinate focused checks with the implementer
   so each selected suite runs once on the same inputs. Schedule the checks
   that supply round evidence after the English operation and scoped commits.
   Keep required reproductions or runs under different conditions. Refresh reports
   when code, tests, build inputs or measurement scope change. If prior
   results still apply, identify their original run and retain them. If the
   final round's language operation changes evidence inputs, refresh the
   affected results before review.

## Review, challenge and repair

Use [TA-WF-Step-review-code](../TA-WF-Step-review-code/SKILL.md) to coordinate
the applicable independent reviewers. For code or test changes, give a
separate [TA-WF-Step-review-tests](../TA-WF-Step-review-tests/SKILL.md)
reviewer the approved scenarios, test results, measured packages and current
coverage reports. The reviewer assesses execution and assertions together,
including executed paths whose assertions would miss the wrong behavior.
Record missing scenarios and their independently expected observations.
Distinguish pre-existing gaps and unmeasured suites from gaps in changed
behavior. [Testing criteria](../../references/testing-criteria.md#coverage-and-adequacy-assessment)

In each round, send reviewer findings and reported check failures with their
evidence to a fresh independent `TA-code-reviewer` using
[TA-WF-Step-challenge-findings](../TA-WF-Step-challenge-findings/SKILL.md).
Retain finding IDs, challenge decisions and required evidence gaps. A challenge
verdict leaves actual check results as recorded.

1. **Initial review.** Use all applicable review assignments on the delivered
   change and affected interactions. Keep the assignment scopes for the final
   full review. Finish if no required issues remain. Otherwise send supported
   in-scope repairs to `TA-implementer` using
   [TA-WF-Step-implement-change](../TA-WF-Step-implement-change/SKILL.md).
   Work directly on master. Coverage or assertion findings can justify
   meaningful tests for approved behavior.
2. **Targeted reassessment.** After the first repair and refreshed checks,
   send the upheld, repaired findings and affected interactions to fresh
   reviewers for the originating assignments. Include the tests reviewer for
   focused reassessment of affected assertions and refreshed coverage, even
   if that assignment raised no finding in round 1. Use checks alone when
   neither the originating reviewers nor a coverage or assertion reassessment
   is needed. Challenge the resulting findings and reported failures. Make a
   second corrective pass only for supported issues within scope.
3. **Final full review.** After any corrective work following round 1, run
   this review with all initial assignments and any newly affected scope.
   Check English again. When round 2 leads directly here and evidence inputs
   still match, reuse its test, gate and coverage results. Challenge final
   findings and failures. Return any required failure, unresolved finding or
   evidence gap for the engineer's decision.

The initial assessment uses no corrective pass. Two passes are available in
total. New findings or failures use the remaining allowance. Return missing
requirements, protected changes and out-of-scope work for the engineer's
decision before work that depends on them.

Report scans, tests, gates and coverage, review rounds, findings and challenge
decisions, corrective changes and commits, report locations and remaining work.
If verification fails or is incomplete, leave the delivered work in place and
report that readiness is unverified. Successful final evidence is required
before separately selected task closure.
