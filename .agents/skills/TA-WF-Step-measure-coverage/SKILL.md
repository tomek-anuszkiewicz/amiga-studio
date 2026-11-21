---
name: TA-WF-Step-measure-coverage
description: Collect one explicitly selected coverage measurement through the existing verifier and return its scope and limits.
---

# Measure the assigned coverage scope

Apply [the result contract](../../references/workflow-contract.md).
Role: `TA-test-runner`.
Inputs: selected crate or workspace scope, exact candidate, reason/trigger for
measurement, output location and execution conditions.

1. Check that scope and required tools/fixtures are available. Read the coverage
   command contract in the [verifier reference](../../../tools/verification/README.md).
   Preserve the ordinary-test boundary excluding `test_runner`.
   Collect coverage only for the selected request or preparation scope after the
   implementation stabilizes; earlier collection needs an explicit request or
   diagnostic reason. Keep the candidate and execution conditions stable.
2. Execute the supplied selection using
   `python tools/verification/verify.py test coverage crate <name>` or the
   already selected `test coverage workspace` scope. This parameter identifies
   the same measurement operation; it does not select an audit or repair workflow.
3. Preserve execution, export and summary as one evidence bundle. Return command,
   exit status, candidate/build identity, reports, measured suites, covered scope,
   exclusions and observation limits. Retained results identify their original run
   and validity basis; do not count them as fresh execution.

Stop after measurement. Do not add tests, interpret uncovered code as an automatic
repair assignment, broaden the scope or launch a test audit.
