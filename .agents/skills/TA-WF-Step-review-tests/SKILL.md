---
name: TA-WF-Step-review-tests
description: Independently assess assertions, missing scenarios, supplied coverage, required verification gates and evidence freshness without repairs or measurement execution.
---

# Assess test protection and verification evidence

Apply the [result contract](../../references/workflow-contract.md),
[independent review contract](../../references/review-common.md) and
[tests and evidence criteria](references/tests-and-evidence.md).
The coordinating session assigns the `tests` review to `TA-code-reviewer`,
which assesses tests and coverage in its own independent context. The assignment
includes approved requirements and scenarios, the exact baseline and candidate,
the selected test scope and available execution and coverage evidence. It also
includes the proposed grouping of changes when relevant.

1. Trace independent expectations and observable assertions to the behavior and
   interactions they protect. Inspect selected tests, fixture limits, missing
   planned scenarios and weak assertions under the shared
   [testing criteria](../../references/testing-criteria.md).
2. For supplied coverage of changed code, identify the measured suites,
   unexecuted functions and source spans, and affected interactions. Separate
   pre-existing gaps, new gaps and measurement limits. Report missing coverage
   as a gap in the inputs. It leaves open whether a test protects the code.
   Assess the assertions even when coverage shows execution. Flag unsupported
   or unreachable paths separately.
3. Check required gates, actual commands, exit status, output scope and freshness
   against this candidate. Failed, missing, interrupted or unrun checks cannot
   establish readiness. Check applicable protected-change approvals and do not
   infer that separately passing branches verify their combined behavior.
4. When a behavior lacks test protection, explain the consequence and propose
   a meaningful scenario. State the independently expected observation and
   assertion, and say whether the scenario fits the approved task. Propose a
   test only when it adds meaningful protection.
5. Return the scope you inspected, scenario assessments, stable finding IDs,
   actual evidence, limits and unresolved decisions. When the evidence supports
   an origin, classify the finding as an implementation defect, a missing
   planned scenario, a repair regression or an integration-only issue.

Stop after assessment. Do not collect coverage, add tests, repair source, dispatch
work or claim approval from a completed audit alone. Preserve supplied finding
IDs when reassessing corrections.
