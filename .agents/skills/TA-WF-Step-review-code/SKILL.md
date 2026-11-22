---
name: TA-WF-Step-review-code
description: Coordinate independent reviewers for selected code changes, test protection and affected documentation, and return their findings without repairs.
---

# Coordinate the selected code review

Apply the [result contract](../../references/workflow-contract.md) and
[independent assessment criteria](../../references/review-common.md).
The main session coordinates independent review assignments for the selected
checkout, change scope, requirements and available test results. Assign review
work to agents independent of the author.

Choose review work from the actual change:

- Behavior, specification compliance and affected interactions use
  [TA-WF-Step-review-behavior](../TA-WF-Step-review-behavior/SKILL.md). A selected stage-wide
  assessment includes unchanged affected code and the full static semantic check.
- Architecture, ownership and internal migrations use
  [TA-WF-Step-review-architecture](../TA-WF-Step-review-architecture/SKILL.md).
- Code/test changes receive a separate tests-and-coverage reviewer instance using
  [TA-WF-Step-review-tests](../TA-WF-Step-review-tests/SKILL.md). Supply the actual coverage reports
  where measured. The reviewer assesses assertions, identifies meaningful
  missing scenarios and proposes tests to cover them. The reason for adding
  each test must go beyond improving the coverage percentage.
- Changed Design documentation, requirements or instruction ownership use
  [TA-WF-Step-review-docs](../TA-WF-Step-review-docs/SKILL.md) when that scope is affected,
  including its Design link/metadata audit and retained-knowledge assessment.

When the outer assignment requests a targeted reassessment, dispatch only the
reviewers selected for findings that the challenge upheld and that were selected
for repair. They reassess those findings, their corrections and affected
interactions.
Keep the same scope even if the overall candidate contains other code or test
changes. Use the review selection above for initial and full reviews. For a
final full review, assess the combined change across all initially selected
assignments and any newly affected scope, even where earlier reviews passed.

Dispatch the selected assignments to fresh independent `TA-code-reviewer` contexts.
The tests reviewer is a separate instance of that role, not the author or the
behavior/architecture reviewer reusing its existing context. When the outer
assignment selects measurement, `TA-test-runner` runs it before the reviewer
assesses the results. The reviewer reports missing data without launching a
measurement.
Give each reviewer only its scope, relevant requirements, files and actual
observations. Leave each reviewer to reach an independent conclusion without
using the author's reasoning or another reviewer's verdict as that conclusion.
Schedule within available runtime slots. If independent contexts are unavailable,
report the gap instead of claiming independent review.

Collect the findings, explain concrete consequences and distinguish overlapping
claims without discarding unique issues. Return a combined scoped assessment in
text and JSON, including unreadable inputs, scope not inspected and missing results.
There is no mandatory four-reviewer set, routing ledger or candidate seal.

Stop after review. Do not repair, stage, commit or start another stage. An outer
implementation/verification assignment may separately include a corrective pass.
