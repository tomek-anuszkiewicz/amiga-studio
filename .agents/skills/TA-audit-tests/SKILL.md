---
name: TA-audit-tests
description: Assess assigned scenarios, assertion quality and supplied test evidence without repairs or coverage execution.
---

# Assess the selected test protection

Apply [the invocation contract](../../references/workflow-contract.md).
Role: `TA-test-auditor`; result kind: `test_audit`.
Inputs: approved scenarios or explicit audit requirements, exact candidate, test
scope and supplied execution/coverage evidence.

1. Read [testing criteria](../../references/testing-criteria.md), applicable
   [testing policy](../../rules/unit-testing-policy.md), requirements and selected
   tests. Trace independent expectations and observable assertions to the behavior
   and interactions they protect.
2. Assess missing planned scenarios, weak assertions, fixture limits, stale
   evidence and supplied changed-code coverage. Execution or a percentage alone
   does not prove correctness. Separate pre-existing gaps from new ones.
3. Report scenario assessments, stable finding IDs, evidence limits and unresolved
   decisions. Classify origin only when supported: implementation defect, missing
   planned scenario, repair regression or integration-only issue.

Stop with the assessment. Missing coverage is reported, not measured; do not add
tests, repair code, dispatch work or claim batch approval. A completed audit may
find required gaps that still block readiness.
