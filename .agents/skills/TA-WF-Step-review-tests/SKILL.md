---
name: TA-WF-Step-review-tests
description: Independently assess assertions, missing scenarios, supplied coverage, required verification gates and evidence freshness without repairs or measurement execution.
---

# Assess test protection and verification evidence

Apply the [result contract](../../references/workflow-contract.md),
[independent review contract](../../references/review-common.md) and
[tests and evidence criteria](references/tests-and-evidence.md).
Role: `TA-code-reviewer`, in its own independent context for tests and coverage;
lane: `tests`.
Inputs: approved requirements/scenarios, exact baseline/candidate, selected test
scope, available execution/coverage evidence and proposed partition when relevant.

1. Trace independent expectations and observable assertions to the behavior and
   interactions they protect. Inspect selected tests, fixture limits, missing
   planned scenarios and weak assertions under the shared
   [testing criteria](../../references/testing-criteria.md).
2. Assess supplied changed-code coverage by measured suites, unexecuted
   functions/source spans and affected interactions. Separate pre-existing gaps,
   new gaps and measurement limits. Missing coverage is an input gap, not proof
   that no test protects the code. Execution or a percentage alone does not prove
   correctness; distinguish unsupported or unreachable paths.
3. Check required gates, actual commands, exit status, output scope and freshness
   against this candidate. Failed, missing, interrupted or unrun checks cannot
   establish readiness. Check applicable protected-change approvals and do not
   infer that separately passing branches verify their combined behavior.
4. For an actual protection gap, propose a meaningful scenario and independently
   expected observation/assertion, explain its consequence and identify whether
   it fits the approved task. Do not propose tests merely to raise a percentage.
5. Return inspected scope, scenario assessments, stable finding IDs, actual
   evidence, limits and unresolved decisions. Classify origin only when supported:
   implementation defect, missing planned scenario, repair regression or
   integration-only issue.

Stop after assessment. Do not collect coverage, add tests, repair source, dispatch
work or claim approval from a completed audit alone. Preserve supplied finding
IDs when reassessing corrections.
