---
name: TA-WF-Step-review-code
description: Coordinate independent reviewers for selected code changes, test protection and affected documentation, and return their findings without repairs.
---

# Coordinate the selected code review

Apply the [result contract](../../references/workflow-contract.md) and
[independent assessment criteria](../../references/review-common.md).
The main session coordinates this compound operation. Inputs: selected checkout,
change scope, requirements and available test results. The author is not its own
independent reviewer.

Choose review work from the actual change:

- Behavior, specification compliance and affected interactions use
  [TA-WF-Step-review-behavior](../TA-WF-Step-review-behavior/SKILL.md). A selected stage-wide
  assessment includes unchanged affected code and the full static semantic check.
- Architecture, ownership and internal migrations use
  [TA-WF-Step-review-architecture](../TA-WF-Step-review-architecture/SKILL.md).
- Code/test changes receive a separate tests-and-coverage reviewer instance using
  [TA-WF-Step-review-tests](../TA-WF-Step-review-tests/SKILL.md). Supply the actual coverage reports
  where measured. It assesses assertions and meaningful missing scenarios and
  proposes test additions; a percentage alone does not justify them.
- Changed Design documentation, requirements or instruction ownership use
  [TA-WF-Step-review-docs](../TA-WF-Step-review-docs/SKILL.md) when that scope is affected,
  including its Design link/metadata audit and retained-knowledge assessment.

When the outer assignment requests a targeted reassessment, dispatch only its
selected reviewer assignments for findings upheld by the challenge and selected
for repair. Reassess those findings, corrections and affected interactions; do not
add other lanes merely because the overall change contains code or tests. The
lane selection above applies to initial and full reviews. A final full review
includes all initially selected assignments and any newly affected scope; earlier
passing verdicts do not replace assessment of the final combined change.

Dispatch the selected assignments to fresh independent `TA-code-reviewer` contexts.
The tests reviewer is a separate instance of that role, not the author or the
behavior/architecture reviewer reusing its existing context. Measurement, when
selected by the outer assignment, is executed by `TA-test-runner` before its
results are assessed; a reviewer reports missing data without launching a measurement.
Give each only its scope, relevant requirements, files and actual observations;
do not feed the author's reasoning or another reviewer's verdict as its conclusion.
Schedule within available runtime slots. If independent contexts are unavailable,
report the gap instead of claiming independent review.

Collect the findings, explain concrete consequences and distinguish overlapping
claims without discarding unique issues. Return a combined scoped assessment in
text and JSON, including unreadable inputs, scope not inspected and missing results.
There is no mandatory four-reviewer set, routing ledger or candidate seal.

Stop after review. Do not repair, stage, commit or start another stage. An outer
implementation/verification assignment may separately include a corrective pass.
